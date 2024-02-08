use crate::{CoCoreResolver, Cores, Reducer, ReducerBuilder, CO_CORE_CO, CO_CORE_KEYSTORE, CO_CORE_MEMBERSHIP};
use anyhow::Context;
use co_log::{LocalIdentityResolver, Log};
use co_primitives::{tags, Did};
use co_runtime::RuntimePool;
use co_storage::{Algorithm, BlockStorage, EncryptedBlockStorage, Secret};
use libipld::Cid;
use serde::{Deserialize, Serialize};
use std::{
	collections::{BTreeMap, BTreeSet},
	io::ErrorKind,
	path::PathBuf,
};

type LocalReducerBuilder<S> = ReducerBuilder<EncryptedBlockStorage<S>, CoCoreResolver>;
type LocalReducer<S> = Reducer<EncryptedBlockStorage<S>, CoCoreResolver>;

pub struct LocalCo {
	/// Our application identifier.
	identifier: String,
	/// The application base path.
	application_path: PathBuf,
}
impl LocalCo {
	pub fn new(identifier: String, application_path: PathBuf) -> Self {
		Self { identifier, application_path }
	}

	/// Read the local co state from disk.
	/// As we trust all of the local states we use all the states without fuhter checks to continue.
	///
	/// Note: This assumes the same encryption key is used by all local applications.
	pub async fn read<S>(&self, storage: S, runtime: &RuntimePool) -> Result<LocalReducer<S>, anyhow::Error>
	where
		S: BlockStorage + Sync + Send + Clone + 'static,
	{
		// read applications
		let mut builder: Option<LocalReducerBuilder<S>> = None;
		let mut dir = match tokio::fs::read_dir(&self.application_path).await {
			Err(e) if e.kind() == ErrorKind::NotFound => {
				// create
				tokio::fs::create_dir_all(&self.application_path).await?;

				// retry
				tokio::fs::read_dir(&self.application_path).await
			},
			i => i,
		}?;
		while let Some(child) = dir.next_entry().await? {
			let local = ApplicationLocal::read(&child.path().join("local.cbor")).await?;
			if let Some(local) = local {
				builder = Some(match builder {
					Some(builder) => builder.with_snapshot(local.state, local.heads),
					None => local.reducer_builder(storage.clone()).await?,
				});
			}
		}

		// result
		Ok(match builder {
			// load
			Some(builder) => builder.build(runtime).await?,
			// create empty
			None => {
				// create empty reducer
				let mut reducer = create_reducer_builder(
					create_local_log(create_encrypted_storage(storage).await?, Default::default()).await?,
				)
				.await?
				.build(runtime)
				.await?;

				// setup
				setup_local_co(runtime, &mut reducer).await?;

				//result
				reducer
			},
		})
	}

	/// Write state to disk.
	/// Returns false and does nothing if reducer is empty.
	pub async fn write<S>(&self, reducer: &LocalReducer<S>) -> Result<bool, anyhow::Error>
	where
		S: BlockStorage + Sync + Send + Clone + 'static,
	{
		if let Some(state) = reducer.state() {
			let local = ApplicationLocal::new(
				reducer.heads().clone(),
				state.clone(),
				reducer.log().storage().clone().flush_mapping().await?,
			);
			local
				.write(&self.application_path.join(&self.identifier).join("local.cbor"))
				.await
				.map(|_| true)
		} else {
			Ok(false)
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ApplicationLocal {
	/// The application local version.
	#[serde(rename = "v")]
	pub version: u8,

	/// The latest heads.
	/// Todo: Do we need this as this is encoded in the state anyway?
	#[serde(rename = "h")]
	pub heads: BTreeSet<Cid>,

	/// The latest state.
	#[serde(rename = "s")]
	pub state: Cid,

	/// The latest encryption mapping.
	#[serde(rename = "m")]
	pub mapping: Cid,
}
impl ApplicationLocal {
	pub fn version() -> u8 {
		1
	}

	pub fn new(heads: BTreeSet<Cid>, state: Cid, mapping: Cid) -> Self {
		Self { heads, state, version: Self::version(), mapping }
	}

	async fn read(path: &PathBuf) -> anyhow::Result<Option<ApplicationLocal>> {
		match tokio::fs::read(path).await {
			Ok(data) => {
				let result: ApplicationLocal = serde_ipld_dagcbor::from_slice(&data)?;
				if result.version != Self::version() {
					return Err(anyhow::anyhow!("Invalid file version"));
				}
				Ok(Some(result))
			},
			Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
			Err(e) => Err(e),
		}
		.context(format!("while reading file {:?}", path))
	}

	async fn write(&self, path: &PathBuf) -> anyhow::Result<()> {
		let data = serde_ipld_dagcbor::to_vec(self)?;
		tokio::fs::write(path, data).await?;
		Ok(())
	}

	pub async fn storage<S>(&self, storage: S) -> Result<EncryptedBlockStorage<S>, anyhow::Error>
	where
		S: BlockStorage + Sync + Send + Clone + 'static,
	{
		let mut encrypted_storage = create_encrypted_storage(storage).await?;
		encrypted_storage.load_mapping(&self.mapping).await?;
		Ok(encrypted_storage)
	}

	pub async fn log<S>(&self, storage: S) -> Result<Log<EncryptedBlockStorage<S>>, anyhow::Error>
	where
		S: BlockStorage + Sync + Send + Clone + 'static,
	{
		create_local_log(self.storage(storage).await?, self.heads.clone()).await
	}

	pub async fn reducer_builder<S>(&self, storage: S) -> Result<LocalReducerBuilder<S>, anyhow::Error>
	where
		S: BlockStorage + Sync + Send + Clone + 'static,
	{
		create_reducer_builder(self.log(storage).await?).await
	}
}

/// Create encrypted storage by using `storage` as unterlying storage.
/// Tries to receive the key from the OS keychain.
/// If no key exists a new random one will be created.
///
/// Todo: What happens if muliple applications try to access the same key?
async fn create_encrypted_storage<S>(storage: S) -> Result<EncryptedBlockStorage<S>, anyhow::Error>
where
	S: BlockStorage + Sync + Send + Clone + 'static,
{
	let key = fetch_secret("co.app", "did:local:device", true)?;
	Ok(EncryptedBlockStorage::new(storage.clone(), key, Default::default()))
}

/// Get or create encryption key in OS Keychain.
fn fetch_secret(service: &str, user: &str, allow_create: bool) -> Result<Secret, anyhow::Error> {
	let entry = keyring::Entry::new(service, user)?;
	let key_as_base64 = match entry.get_password() {
		Ok(p) => p,
		Err(keyring::Error::NoEntry) if allow_create => {
			// generate and set key
			let secret = Algorithm::default().generate_serect();
			let secret_base64 = multibase::encode(multibase::Base::Base64, secret.divulge());
			entry.set_password(&secret_base64)?;

			// fetch again to make sure the key has persisted
			return fetch_secret(service, user, false)
		},
		Err(e) => return Err(e.into()),
	};
	Ok(Secret::new(multibase::decode(key_as_base64)?.1))
}

async fn create_local_log<S>(
	encrypted_storage: EncryptedBlockStorage<S>,
	heads: BTreeSet<Cid>,
) -> Result<Log<EncryptedBlockStorage<S>>, anyhow::Error>
where
	S: BlockStorage + Sync + Send + Clone + 'static,
{
	Ok(Log::new(
		"local".as_bytes().to_vec(),
		LocalIdentityResolver::default().private_identity("did:local:device")?,
		Box::new(LocalIdentityResolver::default()),
		encrypted_storage,
		heads,
	))
}

async fn create_reducer_builder<S>(log: Log<EncryptedBlockStorage<S>>) -> Result<LocalReducerBuilder<S>, anyhow::Error>
where
	S: BlockStorage + Sync + Send + Clone + 'static,
{
	Ok(ReducerBuilder::new(CoCoreResolver::with_mapping(Cores::default().built_in_native_mapping()), log))
}

/// Setup the Local CO by adding cores.
async fn setup_local_co<S>(runtime: &RuntimePool, reducer: &mut LocalReducer<S>) -> Result<(), anyhow::Error>
where
	S: BlockStorage + Sync + Send + Clone + 'static,
{
	// create
	let mut cores = BTreeMap::<String, co_core_co::Core>::new();
	cores.insert(
		Cores::to_core_name(CO_CORE_MEMBERSHIP).to_owned(),
		co_core_co::Core {
			binary: Cores::default().binary(CO_CORE_MEMBERSHIP).expect(CO_CORE_MEMBERSHIP),
			tags: tags!( "core": Cores::to_core_name(CO_CORE_MEMBERSHIP) ),
			state: None,
		},
	);
	cores.insert(
		Cores::to_core_name(CO_CORE_KEYSTORE).to_owned(),
		co_core_co::Core {
			binary: Cores::default().binary(CO_CORE_KEYSTORE).expect(CO_CORE_KEYSTORE),
			tags: tags!( "core": Cores::to_core_name(CO_CORE_KEYSTORE) ),
			state: None,
		},
	);
	let mut participants = BTreeMap::<Did, co_core_co::Participant>::new();
	participants.insert(
		"did:local:device".to_owned(),
		co_core_co::Participant {
			did: "did:local:device".to_owned(),
			state: co_core_co::ParticipantState::Active,
			tags: tags!(),
		},
	);
	let action = co_core_co::CoAction::Create { id: "local".to_owned(), name: "local".to_owned(), cores, participants };
	reducer.push(runtime, Cores::to_core_name(CO_CORE_CO), &action).await?;

	// done
	Ok(())
}
