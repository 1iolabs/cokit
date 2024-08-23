use super::{application::ApplicationSettings, identity::create_identity_resolver, reducer::ReducerChangedHandler};
use crate::{
	library::{
		local_secret::{FileLocalSecret, KeychainLocalSecret, LocalSecret, MemoryLocalSecret},
		locals::{ApplicationLocal, FileLocals, Locals, MemoryLocals},
		to_plain::{to_plain, to_plain_one},
	},
	reducer::core_resolver::dynamic::DynamicCoreResolver,
	types::{
		co_reducer::CoReducerContext,
		co_storage::CoBlockStorageContentMapping,
		cores::{CO_CORE_NAME_CO, CO_CORE_NAME_PIN, CO_CORE_PIN},
	},
	CoReducer, CoStorage, CoreResolver, Cores, Reducer, ReducerBuilder, ReducerChangeContext, Runtime, TaskSpawner,
	CO_CORE_KEYSTORE, CO_CORE_MEMBERSHIP, CO_CORE_NAME_KEYSTORE, CO_CORE_NAME_MEMBERSHIP,
};
use anyhow::anyhow;
use async_trait::async_trait;
use co_identity::{Identity, LocalIdentity};
use co_log::Log;
use co_primitives::{tags, Did, KnownMultiCodec, MultiCodec};
use co_runtime::RuntimePool;
use co_storage::{BlockStorage, BlockStorageContentMapping, EncryptedBlockStorage, StorageError};
use futures::{pin_mut, stream, StreamExt, TryStreamExt};
use libipld::{Cid, DefaultParams};
use std::{collections::BTreeMap, sync::Arc};
use tokio_util::sync::CancellationToken;

pub const CO_ID_LOCAL: &str = "local";

/// Local CO Builder.
/// A local co is special because it's root state will be saved locally to an fiel on an device.
#[derive(Debug, Clone)]
pub struct LocalCoBuilder {
	/// Our application identifier.
	settings: ApplicationSettings,

	/// The local identity.
	identity: LocalIdentity,

	/// Whether to initialize the reducer (compute latest state).
	initialize: bool,
}
impl LocalCoBuilder {
	pub fn new(settings: ApplicationSettings, identity: LocalIdentity, initialize: bool) -> Self {
		Self { settings, identity, initialize }
	}

	pub fn with_initialize(self, initialize: bool) -> Self {
		Self { initialize, ..self }
	}

	/// Create LocalCO instance.
	pub async fn build<R>(
		self,
		storage: CoStorage,
		runtime: Runtime,
		shutdown: CancellationToken,
		tasks: TaskSpawner,
		core_resolver: R,
	) -> Result<CoReducer, anyhow::Error>
	where
		R: CoreResolver<CoStorage> + Send + Sync + 'static,
	{
		// key
		let key: Box<dyn LocalSecret + Send + Sync + 'static> = if self.settings.keychain {
			Box::new(KeychainLocalSecret::new("co.app".to_owned(), self.identity.identity().to_owned()))
		} else if let Some(application_path) = &self.settings.application_path {
			Box::new(FileLocalSecret::new(application_path.parent().expect("etc folder").join("key.cbor")))
		} else {
			Box::new(MemoryLocalSecret::new())
		};

		// create
		match &self.settings.application_path {
			Some(application_path) => {
				let config_path = application_path
					.parent()
					.ok_or(anyhow::anyhow!("application_path to have a parent: {:?}", application_path))?;
				let mut locals = FileLocals::new(config_path.to_owned(), self.settings.identifier.clone());
				locals.update().await?;
				Ok(LocalCoInstance::create(runtime, self, storage, shutdown, tasks, locals, key, core_resolver)
					.await?
					.1)
			},
			None => {
				let locals = MemoryLocals::new(None);
				Ok(LocalCoInstance::create(runtime, self, storage, shutdown, tasks, locals, key, core_resolver)
					.await?
					.1)
			},
		}
	}
}

#[derive(Clone)]
struct LocalCoInstance<L> {
	identifier: String,
	encrypted_storage: EncryptedBlockStorage<CoStorage>,
	locals: L,
}
impl<L> LocalCoInstance<L>
where
	L: Locals + Clone + Send + Sync + 'static,
{
	/// Read the local co state from disk.
	/// As we trust all of the local states we use all the states without fuhter checks to continue.
	///
	///	We use a explicit shutdown signal for this as the reducer is self referencial (through a box) and will not be
	/// dropped when a watcher is active.
	///
	/// NOTE: This assumes the same encryption key is used by all local applications.
	async fn create<R>(
		runtime: Runtime,
		local_co: LocalCoBuilder,
		storage: CoStorage,
		shutdown: CancellationToken,
		tasks: TaskSpawner,
		mut locals: L,
		key: Box<dyn LocalSecret + Send + Sync + 'static>,
		core_resolver: R,
	) -> Result<(Self, CoReducer), anyhow::Error>
	where
		R: CoreResolver<CoStorage> + Send + Sync + 'static,
	{
		// create storage
		let encrypted_storage: EncryptedBlockStorage<CoStorage> = create_encrypted_storage(storage, key).await?;
		let storage = CoStorage::new(encrypted_storage.clone());

		// create log
		let log =
			Log::new(CO_ID_LOCAL.as_bytes().to_vec(), create_identity_resolver(), storage.clone(), Default::default());

		// create builder
		let mut builder =
			ReducerBuilder::new(DynamicCoreResolver::new(core_resolver), log).with_initialize(local_co.initialize);

		// context
		let context = LocalContext { encrypted_storage: encrypted_storage.clone() };

		// create reducer
		for local in locals.get().await? {
			let mut state = local.state;
			let mut heads = local.heads.clone();

			// get local and log
			tracing::trace!(app = ?local_co.settings.identifier, state = ?local.state, heads = ?local.heads, "local-co-read");

			// load additional encryption mappings
			if let Some(mapping) = &local.mapping {
				encrypted_storage.load_mapping(mapping).await?;

				// convert state/heads to internal
				state = context.to_internal_cid(state).await?;
				heads = stream::iter(heads.iter())
					.then(|cid| async { context.to_internal_cid(*cid).await })
					.try_collect()
					.await?;
			}

			// apply to builder as snapshot
			builder = builder.with_snapshot(state, heads);
		}
		let mut reducer = builder.build(runtime.runtime()).await?;

		// result
		let result =
			Self { locals, encrypted_storage: encrypted_storage.clone(), identifier: local_co.settings.identifier };

		// write
		reducer.add_change_handler(Box::new(result.clone()));

		// create empty
		if reducer.is_empty() {
			setup_local_co(runtime.runtime(), &local_co.identity, &mut reducer).await?;
		}

		// reducer
		let co_reducer = CoReducer::new(CO_ID_LOCAL.into(), None, runtime, reducer, Arc::new(context));

		// watch
		let watch_reducer: CoReducer = co_reducer.clone();
		let watch_locals = result.locals.clone();
		let watch_encrypted_storage = encrypted_storage.clone();
		tasks.spawn(async move {
			let watcher = watch_locals.watch().take_until(shutdown.clone().cancelled_owned());
			pin_mut!(watcher);
			while let Some(local) = watcher.next().await {
				// convert heads to unencrypted
				let local_heads = match stream::iter(local.heads.iter())
					.then(|cid| async { watch_reducer.context.to_internal_cid(*cid).await })
					.try_collect()
					.await
				{
					Ok(local_heads) => local_heads,
					Err(err) => {
						tracing::trace!(?err, ?local.heads, "local-watch-cids-failed");
						continue;
					},
				};

				// skip?
				let (_, heads) = watch_reducer.reducer_state().await;
				if heads == local_heads {
					tracing::trace!(?local_heads, "local-watch-skip");
				} else {
					tracing::trace!(?local_heads, ?local.mapping, "local-watch");
				}

				// mappings
				if let Some(mapping) = local.mapping {
					match watch_encrypted_storage.load_mapping(&mapping).await {
						Ok(_) => {},
						Err(err) => tracing::warn!(?err, "local-watch-mapping-failed"),
					}
				}

				// heads
				match watch_reducer.join(&local_heads).await {
					Ok(change) => {
						if change {
							tracing::trace!("local-watch-join");
						}
					},
					Err(err) => tracing::warn!(?err, ?local_heads, "local-watch-join-failed"),
				}
			}
		});

		// result
		Ok((result, co_reducer))
	}

	/// Write state to disk.
	/// Returns false and does nothing if reducer is empty.
	pub async fn write<S, R>(&mut self, reducer: &Reducer<S, R>, mapping: Option<Cid>) -> Result<bool, anyhow::Error>
	where
		S: BlockStorage<StoreParams = DefaultParams> + Sync + Send + Clone + 'static,
		R: CoreResolver<S> + Send + Sync + 'static,
	{
		if let Some(state) = reducer.state() {
			let content_mapping = Some(self.encrypted_storage.content_mapping());

			// heads
			let plain_heads = to_plain(&content_mapping, true, reducer.heads().iter().cloned())
				.await
				.map_err(|err| anyhow!("Failed to map head: {}", err))?;

			// state
			let plain_state = to_plain_one(&content_mapping, true, *state)
				.await
				.map_err(|err| anyhow!("Failed to map state: {}", err))?;

			// create format
			let local = ApplicationLocal::new(plain_heads, plain_state, mapping);

			// log
			#[cfg(debug_assertions)]
			tracing::trace!(app = ?self.identifier, ?local.state, ?local.heads, ?local.mapping,  "local-co-write");
			#[cfg(not(debug_assertions))]
			tracing::trace!(app = ?self.identifier, ?local.state, ?local.heads, ?local.mapping, "local-co-write");

			// write
			self.locals.set(local).await.map(|_| true)
		} else {
			Ok(false)
		}
	}
}
#[async_trait]
impl<L, S, R> ReducerChangedHandler<S, R> for LocalCoInstance<L>
where
	S: BlockStorage<StoreParams = DefaultParams> + Sync + Send + Clone + 'static,
	R: CoreResolver<S> + Send + Sync + 'static,
	L: Locals + Clone + Send + Sync + 'static,
{
	async fn on_state_changed(
		&mut self,
		reducer: &Reducer<S, R>,
		_context: ReducerChangeContext,
	) -> Result<(), anyhow::Error> {
		let mapping = self.encrypted_storage.flush_mapping().await?;
		self.write(reducer, mapping).await?;
		Ok(())
	}
}

struct LocalContext {
	encrypted_storage: EncryptedBlockStorage<CoStorage>,
}
#[async_trait]
impl CoReducerContext for LocalContext {
	fn content_mapping(&self) -> Option<CoBlockStorageContentMapping> {
		Some(CoBlockStorageContentMapping::new(self.encrypted_storage.content_mapping()))
	}

	async fn refresh(&self, _parent: CoReducer, _co: CoReducer) -> anyhow::Result<()> {
		Ok(())
	}

	/// Map external [`Cid`] to internal [`Cid`].
	/// If no mapping is needed/available return the original [`Cid`].
	async fn to_internal_cid(&self, cid: Cid) -> Result<Cid, StorageError> {
		match MultiCodec::from(&cid) {
			MultiCodec::Known(KnownMultiCodec::CoEncryptedBlock) => {
				Ok(*self.encrypted_storage.get_unencrypted(&cid).await?.cid())
			},
			_ => Ok(cid),
		}
	}

	/// Map internal [`Cid`] to external [`Cid`].
	/// If no mapping is needed/available return the original [`Cid`].
	async fn to_external_cid(&self, cid: Cid) -> Result<Cid, StorageError> {
		Ok(self.encrypted_storage.content_mapping().to_plain(&cid).await.unwrap_or(cid))
	}
}

/// Create encrypted storage by using `storage` as unterlying storage.
/// Tries to receive the key from the OS keychain.
/// If no key exists a new random one will be created.
///
/// Todo: What happens if muliple applications try to access the same key?
async fn create_encrypted_storage<S>(
	storage: S,
	key: Box<dyn LocalSecret + Send + Sync + 'static>,
) -> Result<EncryptedBlockStorage<S>, anyhow::Error>
where
	S: BlockStorage + Sync + Send + Clone + 'static,
{
	Ok(EncryptedBlockStorage::new(storage.clone(), key.fetch().await?.into(), Default::default(), Default::default()))
}

/// Setup the Local CO by adding cores.
#[tracing::instrument(err, skip(runtime, reducer))]
async fn setup_local_co<S, R>(
	runtime: &RuntimePool,
	identity: &LocalIdentity,
	reducer: &mut Reducer<S, R>,
) -> Result<(), anyhow::Error>
where
	S: BlockStorage<StoreParams = DefaultParams> + Sync + Send + Clone + 'static,
	R: CoreResolver<S> + Send + Sync + 'static,
{
	// create
	let mut cores = BTreeMap::<String, co_core_co::Core>::new();
	cores.insert(
		CO_CORE_NAME_MEMBERSHIP.to_owned(),
		co_core_co::Core {
			binary: Cores::default().binary(CO_CORE_MEMBERSHIP).expect(CO_CORE_MEMBERSHIP),
			tags: tags!( "core": CO_CORE_MEMBERSHIP ),
			state: None,
		},
	);
	cores.insert(
		CO_CORE_NAME_KEYSTORE.to_owned(),
		co_core_co::Core {
			binary: Cores::default().binary(CO_CORE_KEYSTORE).expect(CO_CORE_KEYSTORE),
			tags: tags!( "core": CO_CORE_KEYSTORE ),
			state: None,
		},
	);
	cores.insert(
		CO_CORE_NAME_PIN.to_owned(),
		co_core_co::Core {
			binary: Cores::default().binary(CO_CORE_PIN).expect(CO_CORE_PIN),
			tags: tags!("core": CO_CORE_PIN),
			state: None,
		},
	);
	let mut participants = BTreeMap::<Did, co_core_co::Participant>::new();
	participants.insert(
		identity.identity().to_owned(),
		co_core_co::Participant {
			did: identity.identity().to_owned(),
			state: co_core_co::ParticipantState::Active,
			tags: tags!(),
		},
	);
	let action = co_core_co::CoAction::Create {
		id: CO_ID_LOCAL.into(),
		name: CO_ID_LOCAL.to_owned(),
		cores,
		participants,
		key: None,
	};
	reducer.push(runtime, identity, CO_CORE_NAME_CO, &action).await?;

	// done
	Ok(())
}
