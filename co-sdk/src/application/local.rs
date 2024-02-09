use super::reducer::ReducerChangedHandler;
use crate::{
	library::{fs_read::fs_read_option, fs_write::fs_write},
	types::cores::CO_CORE_NAME_CO,
	CoCoreResolver, CoReducer, CoStorage, Cores, Reducer, ReducerBuilder, Runtime, CO_CORE_KEYSTORE,
	CO_CORE_MEMBERSHIP, CO_CORE_NAME_KEYSTORE,
};
use anyhow::Context;
use async_trait::async_trait;
use co_log::{LocalIdentityResolver, Log};
use co_primitives::{tags, Did};
use co_runtime::RuntimePool;
use co_storage::{Algorithm, BlockStorage, EncryptedBlockStorage, Secret};
use futures::{pin_mut, Stream, StreamExt};
use libipld::{Cid, DefaultParams};
use serde::{Deserialize, Serialize};
use std::{
	collections::{BTreeMap, BTreeSet},
	io::ErrorKind,
	path::PathBuf,
};

type LocalReducerBuilder = ReducerBuilder<CoStorage, CoCoreResolver>;
type LocalReducer = Reducer<CoStorage, CoCoreResolver>;

#[derive(Debug, Clone)]
pub struct LocalCo {
	/// Our application identifier.
	identifier: String,

	/// The application base path.
	/// Normally compused of `{base_path}/etc/{identifier}`.
	/// The read method tries to read states of all applications by searching for `{application_path}/../*/local.cbor`
	/// files.
	application_path: PathBuf,

	/// Whether to use the keychain or a file.
	keychain: bool,
}
impl LocalCo {
	pub fn new(identifier: String, application_path: PathBuf, keychain: bool) -> Self {
		Self { identifier, application_path, keychain }
	}

	/// Create LocalCO instance.
	pub async fn into_reducer<S>(self, storage: S, runtime: Runtime) -> Result<CoReducer, anyhow::Error>
	where
		S: BlockStorage<StoreParams = DefaultParams> + Sync + Send + Clone + 'static,
	{
		Ok(LocalCoInstance::create(runtime, self, storage).await?.1)
	}

	/// Read the local co state from disk.
	fn read(&self) -> impl Stream<Item = Result<(PathBuf, ApplicationLocal), anyhow::Error>> {
		let application_path = self.application_path.clone();
		async_stream::try_stream! {
			let config_path = application_path
				.parent()
				.ok_or(anyhow::anyhow!("application_path to have a parent: {:?}", application_path))?;

			// read applications
			let mut dir = match tokio::fs::read_dir(config_path).await {
				Err(e) if e.kind() == ErrorKind::NotFound => {
					// create
					tokio::fs::create_dir_all(config_path).await?;

					// retry
					tokio::fs::read_dir(config_path).await
				},
				i => i,
			}?;
			while let Some(child) = dir.next_entry().await? {
				// skip non directories
				if !child.file_type().await?.is_dir() {
					continue;
				}

				// try to read local.cbor
				let local_path = child.path().join("local.cbor");
				let local = ApplicationLocal::read(&local_path).await?;
				if let Some(local) = local {
					yield (local_path, local);
				}
			}
		}
	}

	/// Key path if no keychain should be used.
	fn key_path(&self) -> Option<PathBuf> {
		// use file
		if !self.keychain {
			if let Some(parent) = self.application_path.parent() {
				return Some(parent.join("key.cbor"))
			}
		}

		// use keychain
		None
	}
}

#[derive(Debug, Clone)]
struct LocalCoInstance<S> {
	local_co: LocalCo,
	encrypted_storage: EncryptedBlockStorage<S>,
}
impl<S> LocalCoInstance<S>
where
	S: BlockStorage<StoreParams = DefaultParams> + Sync + Send + Clone + 'static,
{
	/// Read the local co state from disk.
	/// As we trust all of the local states we use all the states without fuhter checks to continue.
	///
	/// Note: This assumes the same encryption key is used by all local applications.
	async fn create(runtime: Runtime, local_co: LocalCo, storage: S) -> Result<(Self, CoReducer), anyhow::Error> {
		// create storage
		let mut encrypted_storage: EncryptedBlockStorage<S> =
			create_encrypted_storage(storage, local_co.key_path()).await?;

		// create log
		let log = create_local_log(CoStorage::new(encrypted_storage.clone()), Default::default())?;

		// create builder
		let mut builder: LocalReducerBuilder = create_reducer_builder(log)?;

		// create reducer
		let locals = local_co.read();
		pin_mut!(locals);
		while let Some(local_result) = locals.next().await {
			// get local and log
			let local = match local_result {
				Ok((local_path, local)) => {
					tracing::trace!(app = ?local_co.identifier, path = ?local_path, state = ?local.state, heads = ?local.heads, "local-co-read");
					local
				},
				Err(e) => {
					tracing::warn!(app = ?local_co.identifier, err = ?e, "local-co-read-failure");
					continue;
				},
			};

			// load additional encryption mappings
			encrypted_storage.load_mapping(&local.mapping).await?;

			// apply to builder as snapshot
			builder = builder.with_snapshot(local.state, local.heads);
		}
		let mut reducer = builder.build(runtime.runtime()).await?;

		// result
		let result = Self { encrypted_storage, local_co };

		// write
		reducer.add_change_handler(Box::new(result.clone()));

		// create empty
		if reducer.is_empty() {
			setup_local_co(runtime.runtime(), &mut reducer).await?;
		}

		// result
		Ok((result, CoReducer::new(runtime, reducer)))
	}

	/// Write state to disk.
	/// Returns false and does nothing if reducer is empty.
	pub async fn write(&self, reducer: &LocalReducer, mapping: Cid) -> Result<bool, anyhow::Error> {
		if let Some(state) = reducer.state() {
			let path = self.local_co.application_path.join("local.cbor");

			// trace
			tracing::trace!(app = ?self.local_co.identifier, ?path, ?state, "local-co-write");

			// create format
			let local = ApplicationLocal::new(reducer.heads().clone(), state.clone(), mapping);

			// write
			local.write(&path).await.map(|_| true)
		} else {
			Ok(false)
		}
	}
}
#[async_trait]
impl<S> ReducerChangedHandler<CoStorage, CoCoreResolver> for LocalCoInstance<S>
where
	S: BlockStorage<StoreParams = DefaultParams> + Sync + Send + Clone + 'static,
{
	async fn on_state_changed(&self, reducer: &LocalReducer) -> Result<(), anyhow::Error> {
		let mapping = self.encrypted_storage.flush_mapping().await?;
		self.write(reducer, mapping).await?;
		Ok(())
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
		Ok(
			match fs_read_option(path)
				.await
				.with_context(|| format!("Reading file: {:?}", path))?
			{
				Some(data) => {
					let result: ApplicationLocal = serde_ipld_dagcbor::from_slice(&data)?;
					if result.version != Self::version() {
						return Err(anyhow::anyhow!("Invalid file version"));
					}
					Some(result)
				},
				None => None,
			},
		)
	}

	async fn write(&self, path: &PathBuf) -> anyhow::Result<()> {
		// serialize
		let data = serde_ipld_dagcbor::to_vec(self)?;

		// write
		fs_write(path, data, true)
			.await
			.with_context(|| format!("Writing file: {:?}", path))?;

		// result
		Ok(())
	}
}

/// Create encrypted storage by using `storage` as unterlying storage.
/// Tries to receive the key from the OS keychain.
/// If no key exists a new random one will be created.
///
/// Todo: What happens if muliple applications try to access the same key?
async fn create_encrypted_storage<S>(
	storage: S,
	key_path: Option<PathBuf>,
) -> Result<EncryptedBlockStorage<S>, anyhow::Error>
where
	S: BlockStorage + Sync + Send + Clone + 'static,
{
	let key = match key_path {
		Some(key_path) => fetch_secret_cbor(key_path, true).await?,
		None => fetch_secret_keychain("co.app", "did:local:device", true)?,
	};
	Ok(EncryptedBlockStorage::new(storage.clone(), key, Default::default()))
}

async fn fetch_secret_cbor(key_path: PathBuf, allow_create: bool) -> Result<Secret, anyhow::Error> {
	match fs_read_option(&key_path).await {
		Ok(Some(data)) => {
			let result: Vec<u8> = serde_ipld_dagcbor::from_slice(&data)?;
			Ok(Secret::new(result))
		},
		Ok(None) if allow_create => {
			// create
			let secret = Algorithm::default().generate_serect();
			let contents: Vec<u8> = serde_ipld_dagcbor::to_vec(secret.divulge())?;
			fs_write(&key_path, contents, true).await?;

			// result
			Ok(secret)
		},
		Ok(None) => Err(Into::<std::io::Error>::into(ErrorKind::NotFound).into()),
		Err(e) => Err(e.into()),
	}
}

/// Get or create encryption key in OS Keychain.
fn fetch_secret_keychain(service: &str, user: &str, allow_create: bool) -> Result<Secret, anyhow::Error> {
	let entry = keyring::Entry::new(service, user)?;
	let key_as_base64 = match entry.get_password() {
		Ok(p) => p,
		Err(keyring::Error::NoEntry) if allow_create => {
			// generate and set key
			let secret = Algorithm::default().generate_serect();
			let secret_base64 = multibase::encode(multibase::Base::Base64, secret.divulge());
			entry.set_password(&secret_base64)?;

			// fetch again to make sure the key has persisted
			return fetch_secret_keychain(service, user, false)
		},
		Err(e) => return Err(e.into()),
	};
	Ok(Secret::new(multibase::decode(key_as_base64)?.1))
}

fn create_local_log<S>(storage: S, heads: BTreeSet<Cid>) -> Result<Log<S>, anyhow::Error>
where
	S: BlockStorage + Sync + Send + Clone + 'static,
{
	Ok(Log::new(
		"local".as_bytes().to_vec(),
		LocalIdentityResolver::default().private_identity("did:local:device")?,
		Box::new(LocalIdentityResolver::default()),
		storage,
		heads,
	))
}

fn create_reducer_builder<S>(log: Log<S>) -> Result<ReducerBuilder<S, CoCoreResolver>, anyhow::Error>
where
	S: BlockStorage + Sync + Send + Clone + 'static,
{
	Ok(ReducerBuilder::new(CoCoreResolver::default(), log))
}

/// Setup the Local CO by adding cores.
async fn setup_local_co(runtime: &RuntimePool, reducer: &mut LocalReducer) -> Result<(), anyhow::Error> {
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
		CO_CORE_NAME_KEYSTORE.to_owned(),
		co_core_co::Core {
			binary: Cores::default().binary(CO_CORE_KEYSTORE).expect(CO_CORE_KEYSTORE),
			tags: tags!( "core": CO_CORE_NAME_KEYSTORE ),
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
	reducer.push(runtime, CO_CORE_NAME_CO, &action).await?;

	// done
	Ok(())
}
