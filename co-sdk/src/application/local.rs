// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

#[cfg(feature = "fs")]
use crate::library::local_secret_file::FileLocalSecret;
#[cfg(feature = "keychain")]
use crate::library::local_secret_keychain::KeychainLocalSecret;
#[cfg(feature = "fs")]
use crate::library::locals_file::FileLocals;
use crate::{
	application::application::ApplicationSettings,
	library::{
		builtin_cores::builtin_cores,
		core_source::CoreSource,
		local_secret::{LocalSecret, MemoryLocalSecret},
		locals::{ApplicationLocal, Locals},
		locals_memory::MemoryLocals,
	},
	reducer::core_resolver::dynamic::DynamicCoreResolver,
	services::reducer::{FlushInfo, ReducerFlush},
	types::{
		co_reducer_context::{CoReducerContext, CoReducerFeature},
		co_reducer_state::MappedCoReducerState,
	},
	ApplicationMessage, CoReducer, CoReducerState, CoStorage, CoreResolver, Cores, Reducer, ReducerBuilder, Runtime,
	TaskSpawner, CO_CORE_CO, CO_CORE_KEYSTORE, CO_CORE_MEMBERSHIP, CO_CORE_NAME_CO, CO_CORE_NAME_KEYSTORE,
	CO_CORE_NAME_MEMBERSHIP,
};
#[cfg(feature = "pinning")]
use crate::{
	library::create_storage_core_state::create_storage_core_state,
	types::cores::{CO_CORE_NAME_STORAGE, CO_CORE_STORAGE},
};
#[cfg(all(feature = "indexeddb", target_arch = "wasm32"))]
use crate::{library::locals_indexeddb::IndexedDbLocals, CoStorageSetting};
use async_trait::async_trait;
use cid::Cid;
use co_actor::ActorHandle;
use co_identity::{Identity, LocalIdentity};
use co_log::Log;
use co_primitives::{tags, BlockLinks, CloneWithBlockStorageSettings, DynamicCoDate, OptionMappedCid};
use co_storage::{
	BlockStorage, BlockStorageContentMapping, EncryptedBlockStorage, EncryptionReferenceMode, ExtendedBlockStorage,
};
use futures::{pin_mut, Stream, StreamExt, TryStreamExt};
use std::{collections::BTreeSet, fmt::Debug, sync::Arc};
use tokio_util::sync::CancellationToken;

pub const CO_ID_LOCAL: &str = "local";

/// Local CO Builder.
/// A local co is special because it's root state will be saved locally to a file on a device.
#[derive(Debug, Clone)]
pub struct LocalCoBuilder {
	/// Our application identifier.
	settings: ApplicationSettings,

	/// The local identity.
	identity: LocalIdentity,

	/// Whether to initialize the reducer (compute latest state).
	initialize: bool,

	/// Verify Links
	verify_links: Option<BlockLinks>,
}
impl LocalCoBuilder {
	pub fn new(settings: ApplicationSettings, identity: LocalIdentity, initialize: bool) -> Self {
		Self { settings, identity, initialize, verify_links: None }
	}

	pub fn with_initialize(self, initialize: bool) -> Self {
		Self { initialize, ..self }
	}

	pub fn with_verify_links(self, verify_links: Option<BlockLinks>) -> Self {
		Self { verify_links, ..self }
	}

	/// Create LocalCO instance.
	pub async fn build<R>(self, context: LocalCoContext<R>, cores: &Cores) -> Result<CoReducer, anyhow::Error>
	where
		R: CoreResolver<CoStorage> + Send + Sync + 'static,
	{
		// key
		let key: Option<Box<dyn LocalSecret + Send + Sync + 'static>> =
			if self.settings.feature_co_local_encryption() { Some(self.build_local_secret()) } else { None };

		// file
		#[cfg(feature = "fs")]
		if let Some(application_path) = &self.settings.application_path {
			let watcher = self.settings.feature_co_local_watch();
			let config_path = application_path
				.parent()
				.ok_or(anyhow::anyhow!("application_path to have a parent: {:?}", application_path))?;
			let locals =
				FileLocals::new(context.tasks.clone(), config_path.to_owned(), self.settings.identifier.clone(), true)?;
			return Ok(LocalCoInstance::create(context, cores, self, locals, key, watcher).await?.1);
		}

		// indexeddb
		#[cfg(all(feature = "indexeddb", target_arch = "wasm32"))]
		if let CoStorageSetting::IndexedDb = &self.settings.storage {
			let watcher = self.settings.feature_co_local_watch();
			let locals = IndexedDbLocals::new(format!("co-locals::{}", self.settings.identifier))?;
			return Ok(LocalCoInstance::create(context, cores, self, locals, key, watcher).await?.1);
		}

		// memory
		let locals = MemoryLocals::new(None);
		Ok(LocalCoInstance::create(context, cores, self, locals, key, false).await?.1)
	}

	fn build_local_secret(&self) -> Box<dyn LocalSecret + Send + Sync> {
		// keychain
		#[cfg(feature = "keychain")]
		if self.settings.keychain {
			return Box::new(KeychainLocalSecret::new("co.app".to_owned(), self.identity.identity().to_owned()));
		}

		// fs
		#[cfg(feature = "fs")]
		if let Some(application_path) = &self.settings.application_path {
			return Box::new(FileLocalSecret::new(application_path.parent().expect("etc folder").join("key.cbor")));
		}

		// memory
		Box::new(MemoryLocalSecret::new())
	}
}

/// Context for the LocalCo
pub struct LocalCoContext<R> {
	pub storage: CoStorage,
	pub runtime: Runtime,
	pub shutdown: CancellationToken,
	pub tasks: TaskSpawner,
	pub core_resolver: R,
	pub date: DynamicCoDate,
	pub application_handle: ActorHandle<ApplicationMessage>,
	#[cfg(feature = "pinning")]
	pub pinning: crate::library::storage_pinning::StoragePinningContext,
}

#[derive(Debug, Clone)]
struct LocalCoInstance<L> {
	storage: CoStorage,
	encrypted_storage: Option<EncryptedBlockStorage<CoStorage>>,
	locals: L,
	#[cfg(feature = "pinning")]
	pinning: (CoReducerState, crate::library::storage_pinning::StoragePinningContext),
}
impl<L> LocalCoInstance<L>
where
	L: Locals + Clone + Debug + Send + Sync + 'static,
{
	/// Read the local co state from disk.
	/// As we trust all of the local states we use all the states without fuhter checks to continue.
	/// We use a explicit shutdown signal for this as the reducer is self referencial (through a box) and will not be
	/// dropped when a watcher is active.
	///
	/// NOTE: This assumes the same encryption key is used by all local applications.
	async fn create<R>(
		LocalCoContext {
			storage,
			runtime,
			shutdown,
			tasks,
			core_resolver,
			date,
			application_handle,
			#[cfg(feature = "pinning")]
			pinning,
		}: LocalCoContext<R>,
		cores: &Cores,
		local_co: LocalCoBuilder,
		locals: L,
		key: Option<Box<dyn LocalSecret + Send + Sync + 'static>>,
		watcher: bool,
	) -> Result<(Self, CoReducer), anyhow::Error>
	where
		R: CoreResolver<CoStorage> + Send + Sync + 'static,
	{
		// create storage
		let (base_storage, storage, encrypted_storage) = match key {
			Some(key) => {
				let encrypted_storage = create_encrypted_storage(storage.clone(), key, true).await?;
				(storage, CoStorage::new(encrypted_storage.clone()), Some(encrypted_storage))
			},
			None => (storage.clone(), storage, None),
		};

		// result
		let result = Self {
			locals: locals.clone(),
			storage: base_storage,
			encrypted_storage: encrypted_storage.clone(),
			#[cfg(feature = "pinning")]
			pinning: (Default::default(), pinning),
		};
		let context = Arc::new(result.clone());

		// create log
		let log = Log::new_local(CO_ID_LOCAL.as_bytes().to_vec(), Default::default());

		// create builder
		let mut reducer_builder =
			ReducerBuilder::new(DynamicCoreResolver::new(core_resolver), log).with_initialize(local_co.initialize);

		// load locals as snapshots
		//  the latest heads will be automatically determined by the reducer
		{
			let locals_stream = result.load_locals();
			pin_mut!(locals_stream);
			while let Some(next) = locals_stream.try_next().await? {
				// apply to builder as snapshot
				if let Some((state, heads)) = next.some() {
					reducer_builder = reducer_builder.with_snapshot(state, heads);
				}
			}
		}

		// use storage core
		#[cfg(feature = "pinning")]
		{
			reducer_builder = reducer_builder.with_state_resolver(
				crate::reducer::state_resolver::LocalStorageStateResolver::new(CO_ID_LOCAL.into()),
			);
		}

		// create reducer
		let reducer = reducer_builder.build(&storage, runtime.runtime(), date).await?;
		let initial = reducer.is_empty();

		// flush
		let flush = result.clone();
		#[cfg(feature = "pinning")]
		let flush = {
			let mut flush = flush;
			flush.pinning.0 = CoReducerState::new_reducer(&reducer);
			flush
		};

		// reducer
		let co_reducer = CoReducer::spawn(
			application_handle,
			local_co.settings.identifier.clone(),
			CO_ID_LOCAL.into(),
			None,
			tasks.clone(),
			runtime,
			reducer,
			context,
			Box::new(flush),
			false,
			local_co.verify_links,
		)?;

		// setup
		if initial {
			setup_local_co(&co_reducer, &local_co.identity, &local_co.settings, cores).await?;
		}

		// watch
		#[cfg(any(feature = "fs", all(feature = "indexeddb", target_arch = "wasm32")))]
		if watcher {
			let watch_reducer: CoReducer = co_reducer.clone();
			let watch_locals = result.locals.clone();
			let watch_encrypted_storage = encrypted_storage.clone();
			tasks.spawn(async move {
				let watcher = watch_locals.watch().take_until(shutdown.clone().cancelled_owned());
				pin_mut!(watcher);
				while let Some(local) = watcher.next().await {
					// convert to unencrypted
					let local_state = if let Some(watch_encrypted_storage) = &watch_encrypted_storage {
						match local.reducer_state().to_internal_force(watch_encrypted_storage).await {
							Ok(local_state) => local_state,
							Err(err) => {
								tracing::trace!(?err, ?local, "local-watch-cids-failed");
								continue;
							},
						}
					} else {
						local.reducer_state()
					};

					// skip?
					let previous_state = watch_reducer.reducer_state().await;
					if previous_state == local_state {
						tracing::trace!(?local_state, "local-watch-skip");
						continue;
					} else {
						tracing::trace!(?previous_state, ?local_state, ?local.mapping, "local-watch");
					}

					// mappings
					if let Some(watch_encrypted_storage) = &watch_encrypted_storage {
						if let Some(mapping) = local.mapping {
							match watch_encrypted_storage.load_mapping(&mapping).await {
								Ok(_) => {},
								Err(err) => tracing::warn!(?err, "local-watch-mapping-failed"),
							}
						}
					}

					// join
					match watch_reducer.join_state(local_state.clone()).await {
						Ok(next_state) => {
							tracing::trace!(?previous_state, ?next_state, ?local_state, "local-watch-joined");
						},
						Err(err) => tracing::warn!(?err, ?previous_state, ?local_state, "local-watch-join-failed"),
					}
				}
			});
		}

		// result
		Ok((result, co_reducer))
	}

	/// Write state to disk.
	/// Returns false and does nothing if reducer is empty.
	pub async fn write(
		&mut self,
		external_reducer_state: CoReducerState,
		mapping: Option<Cid>,
	) -> Result<bool, anyhow::Error> {
		if let Some(external_state) = external_reducer_state.state() {
			// create format
			let local = ApplicationLocal::new(external_reducer_state.heads(), external_state, mapping);

			// log
			#[cfg(feature = "logging-verbose")]
			tracing::trace!(?local.state, ?local.heads, ?local.mapping, "local-co-write");

			// write
			self.locals.set(local).await.map(|_| true)
		} else {
			Ok(false)
		}
	}

	fn load_locals(&self) -> impl Stream<Item = Result<CoReducerState, anyhow::Error>> + '_ {
		async_stream::try_stream! {
			for local in self.locals.get().await? {
				// encryption
				let state = if let Some(encrypted_storage) = &self.encrypted_storage {
					// load additional encryption mappings
					if let Some(mapping) = &local.mapping {
						encrypted_storage.load_mapping(mapping).await?;
					}

					// convert state/heads to internal
					local.reducer_state().to_internal_force(encrypted_storage).await?
				} else {
					local.reducer_state()
				};

				// apply
				yield state
			}
		}
	}
}
#[async_trait]
impl<L> CoReducerContext for LocalCoInstance<L>
where
	L: Locals + Clone + Debug + Send + Sync + 'static,
{
	/// The LocalCo never uses networking.
	fn storage(&self, _force_local: bool) -> CoStorage {
		// encrypted
		if let Some(encrypted_storage) = &self.encrypted_storage {
			return CoStorage::new(encrypted_storage.clone());
		}

		// base
		self.storage.clone()
	}

	#[tracing::instrument(level = tracing::Level::TRACE, skip(_parent, co))]
	async fn refresh(&self, _parent: CoReducer, co: CoReducer) -> anyhow::Result<()> {
		// read and apply locals
		//  this will manually re-read all local files
		self.load_locals()
			.try_for_each(|state| {
				let co = &co;
				async move {
					tracing::info!(?state, "LOAD");
					co.join_state(state).await?;
					Ok(())
				}
			})
			.await?;

		// done
		Ok(())
	}

	/// Clear reducer caches.
	async fn clear(&self, co: CoReducer) {
		// clear reducer
		let state = co.clear().await;

		// clear storage
		if let Some(encrypted_storage) = &self.encrypted_storage {
			encrypted_storage.clear_mapping(state.0.into_iter().chain(state.1)).await;
		}
	}

	/// Test for reducer feature.
	fn has_feature(&self, feature: &CoReducerFeature<'_>) -> bool {
		match feature {
			CoReducerFeature::Encryption => self.encrypted_storage.is_some(),
			_ => false,
		}
	}
}
#[async_trait]
impl<L, S, R> ReducerFlush<S, R> for LocalCoInstance<L>
where
	L: Locals + Clone + Debug + Send + Sync + 'static,
	S: ExtendedBlockStorage
		+ BlockStorageContentMapping
		+ CloneWithBlockStorageSettings
		+ Clone
		+ Sync
		+ Send
		+ 'static,
	R: CoreResolver<S> + Send + Sync + 'static,
{
	/// Flush changed nodes.
	///
	/// # Pinning
	/// We need to write intermediate states/heads to in order to have them recycled eventually.
	/// When we rect this point they alredy has been flushed to the permananet storage.
	async fn flush(
		&mut self,
		storage: &S,
		reducer: &mut Reducer<S, R>,
		_info: &FlushInfo,
		_new_roots: Vec<CoReducerState>,
		_removed_blocks: BTreeSet<OptionMappedCid>,
	) -> anyhow::Result<()> {
		// write references
		#[cfg(feature = "pinning")]
		if _info.local {
			let new_roots = _new_roots;
			let removed_blocks = _removed_blocks;
			let (last_reducer_state, context) = &self.pinning;

			// add last state from disk
			//  we add the lastest state (from disk) as first root
			//  this contains the pinnings (state updates below) from the last time
			//  we need this to have a full history of heads
			//  TODO: do we need to add intermediate heads from below (or encapsulate them into one transaction)?
			let mut new_roots = new_roots;
			if !last_reducer_state.is_empty() {
				new_roots.insert(0, last_reducer_state.clone());
			}
			let next_reducer_state = CoReducerState::new_reducer(reducer);
			if !new_roots.contains(&next_reducer_state) {
				return Err(anyhow::anyhow!("Missing current state from roots"));
			}

			// compute
			let pinning_state = crate::library::storage_pinning::storage_pinning(
				context,
				None,
				storage,
				next_reducer_state,
				&CO_ID_LOCAL.into(),
				storage,
				new_roots,
				removed_blocks,
			)
			.await?;

			// apply
			if let Some(pinning_state) = pinning_state {
				if let Some((state, heads)) = pinning_state.some() {
					reducer.insert_snapshot(storage, state, heads.clone()).await?;
					reducer.join(storage, &heads, context.runtime.runtime()).await?;
				}
			}

			// write including the pinning changes
			self.pinning.0 = CoReducerState::new_reducer(reducer);
		}

		// forward mapping to root storage
		let external_reducer_state = if let Some(encrypted_storage) = &self.encrypted_storage {
			let mapped_reducer_state = MappedCoReducerState::new_reducer(storage, reducer).await;
			let extenal_reducer_state = mapped_reducer_state.force_external()?;
			encrypted_storage.insert_mappings(mapped_reducer_state.iter_mapped()).await;
			extenal_reducer_state
		} else {
			CoReducerState::new_reducer(reducer)
		};

		// write local
		self.write(external_reducer_state, None).await?;

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
	key: Box<dyn LocalSecret + Send + Sync + 'static>,
	disallow_plain: bool,
) -> Result<EncryptedBlockStorage<S>, anyhow::Error>
where
	S: BlockStorage + Sync + Send + Clone + 'static,
{
	// we have plain references:
	// - buildin core references
	//   - third party cores are expected to be encrypted
	// - unencrypted shared COs
	//   - all references to it should be [`CoReference::Weak`].
	let reference_mode = if disallow_plain {
		EncryptionReferenceMode::DisallowPlainExcept(builtin_cores())
	} else {
		EncryptionReferenceMode::Warning
	};
	Ok(EncryptedBlockStorage::new(storage.clone(), key.fetch().await?.into(), Default::default(), Default::default())
		.with_encryption_reference_mode(reference_mode))
}

/// Setup the Local CO by adding cores.
#[tracing::instrument(level = tracing::Level::TRACE, err(Debug), skip(reducer))]
async fn setup_local_co(
	reducer: &CoReducer,
	identity: &LocalIdentity,
	settings: &ApplicationSettings,
	cores: &Cores,
) -> Result<(), anyhow::Error> {
	let storage = reducer.storage();

	// create
	let create = co_core_co::CreateAction::new(
		CO_ID_LOCAL.into(),
		CO_ID_LOCAL.to_owned(),
		CoreSource::built_in(CO_CORE_CO).binary(&storage, cores).await?,
	)
	.with_core(
		CO_CORE_NAME_MEMBERSHIP.to_string(),
		CoreSource::built_in(CO_CORE_MEMBERSHIP)
			.to_core(&storage, cores, tags!( "core": CO_CORE_MEMBERSHIP ))
			.await?,
	)
	.with_core(
		CO_CORE_NAME_KEYSTORE.to_string(),
		CoreSource::built_in(CO_CORE_KEYSTORE)
			.to_core(&storage, cores, tags!( "core": CO_CORE_KEYSTORE ))
			.await?,
	)
	.with_participant(identity.identity().to_owned(), tags!());

	// pinning
	#[cfg(feature = "pinning")]
	let create = create.with_core(
		CO_CORE_NAME_STORAGE.to_string(),
		CoreSource::built_in(CO_CORE_STORAGE)
			.to_core(&storage, cores, tags!( "core": CO_CORE_STORAGE ))
			.await?
			.with_state(create_storage_core_state(&reducer.storage(), settings, &CO_ID_LOCAL.into()).await?),
	);

	// create
	let action = co_core_co::CoAction::Create(create);
	reducer.push(identity, CO_CORE_NAME_CO, &action).await?;

	// done
	Ok(())
}
