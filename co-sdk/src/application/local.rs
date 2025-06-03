use super::{application::ApplicationSettings, identity::create_identity_resolver};
#[cfg(feature = "pinning")]
use crate::types::{
	co_pinning_key::CoPinningKey,
	cores::{CO_CORE_NAME_STORAGE, CO_CORE_STORAGE},
};
use crate::{
	library::{
		local_secret::{FileLocalSecret, KeychainLocalSecret, LocalSecret, MemoryLocalSecret},
		locals::{ApplicationLocal, FileLocals, Locals, MemoryLocals},
	},
	reducer::core_resolver::dynamic::DynamicCoreResolver,
	services::reducer::ReducerFlush,
	types::{
		co_reducer_context::{CoReducerContext, CoReducerFeature},
		co_reducer_state::MappedCoReducerState,
	},
	ApplicationMessage, CoReducer, CoReducerState, CoStorage, CoreResolver, Cores, DynamicCoDate, Reducer,
	ReducerBuilder, Runtime, TaskSpawner, CO_CORE_CO, CO_CORE_KEYSTORE, CO_CORE_MEMBERSHIP, CO_CORE_NAME_CO,
	CO_CORE_NAME_KEYSTORE, CO_CORE_NAME_MEMBERSHIP,
};
use async_trait::async_trait;
use cid::Cid;
use co_actor::ActorHandle;
use co_identity::{Identity, LocalIdentity};
use co_log::Log;
use co_primitives::{tags, CloneWithBlockStorageSettings, Did, OptionMappedCid};
use co_runtime::RuntimePool;
use co_storage::{
	BlockStorage, BlockStorageContentMapping, EncryptedBlockStorage, EncryptionReferenceMode, ExtendedBlockStorage,
};
use futures::{pin_mut, Stream, StreamExt, TryStreamExt};
use std::{
	collections::{BTreeMap, BTreeSet},
	fmt::Debug,
	sync::Arc,
};
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
		date: DynamicCoDate,
		application_handle: ActorHandle<ApplicationMessage>,
		#[cfg(feature = "pinning")] pinning: crate::library::storage_pinning::StoragePinningContext,
	) -> Result<CoReducer, anyhow::Error>
	where
		R: CoreResolver<CoStorage> + Send + Sync + 'static,
	{
		// key
		let key: Option<Box<dyn LocalSecret + Send + Sync + 'static>> = if self.settings.feature_co_local_encryption() {
			Some(if self.settings.keychain {
				Box::new(KeychainLocalSecret::new("co.app".to_owned(), self.identity.identity().to_owned()))
			} else if let Some(application_path) = &self.settings.application_path {
				Box::new(FileLocalSecret::new(application_path.parent().expect("etc folder").join("key.cbor")))
			} else {
				Box::new(MemoryLocalSecret::new())
			})
		} else {
			None
		};

		// create
		let watcher = self.settings.feature_co_local_watch();
		match &self.settings.application_path {
			Some(application_path) => {
				let config_path = application_path
					.parent()
					.ok_or(anyhow::anyhow!("application_path to have a parent: {:?}", application_path))?;
				let locals = FileLocals::new(config_path.to_owned(), self.settings.identifier.clone(), true)?;
				Ok(LocalCoInstance::create(
					runtime,
					self,
					storage,
					shutdown,
					tasks,
					locals,
					key,
					core_resolver,
					watcher,
					date,
					application_handle,
					#[cfg(feature = "pinning")]
					pinning,
				)
				.await?
				.1)
			},
			None => {
				let locals = MemoryLocals::new(None);
				Ok(LocalCoInstance::create(
					runtime,
					self,
					storage,
					shutdown,
					tasks,
					locals,
					key,
					core_resolver,
					watcher,
					date,
					application_handle,
					#[cfg(feature = "pinning")]
					pinning,
				)
				.await?
				.1)
			},
		}
	}
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
		locals: L,
		key: Option<Box<dyn LocalSecret + Send + Sync + 'static>>,
		core_resolver: R,
		watcher: bool,
		date: DynamicCoDate,
		application_handle: ActorHandle<ApplicationMessage>,
		#[cfg(feature = "pinning")] pinning: crate::library::storage_pinning::StoragePinningContext,
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
		let log = Log::new(CO_ID_LOCAL.as_bytes().to_vec(), create_identity_resolver(), Default::default());

		// create builder
		let mut builder =
			ReducerBuilder::new(DynamicCoreResolver::new(core_resolver), log).with_initialize(local_co.initialize);

		// load locals as snapshots
		//  the latest heads will be automatically determined by the reducer
		{
			let locals_stream = result.load_locals();
			pin_mut!(locals_stream);
			while let Some(next) = locals_stream.try_next().await? {
				// apply to builder as snapshot
				if let Some((state, heads)) = next.some() {
					builder = builder.with_snapshot(state, heads);
				}
			}
		}

		// create reducer
		let mut reducer = builder.build(&storage, runtime.runtime(), date).await?;

		// create empty
		if reducer.is_empty() {
			setup_local_co(&storage, runtime.runtime(), &local_co.identity, &mut reducer, &local_co.settings).await?;
		}

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
			storage,
			tasks.clone(),
			runtime,
			reducer,
			context,
			Box::new(flush),
		)?;

		// watch
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

	async fn refresh(&self, _parent: CoReducer, co: CoReducer) -> anyhow::Result<()> {
		// read and apply locals
		//  this will manually re-read all local files
		self.load_locals()
			.try_for_each(|state| {
				let co = &co;
				async move {
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
	async fn flush(
		&mut self,
		storage: &S,
		reducer: &mut Reducer<S, R>,
		_new_roots: Vec<CoReducerState>,
		_removed_blocks: BTreeSet<OptionMappedCid>,
	) -> anyhow::Result<()> {
		// write references
		#[cfg(feature = "pinning")]
		{
			let new_roots = _new_roots;
			let removed_blocks = _removed_blocks;
			let (last_reducer_state, context) = &self.pinning;

			// add last state from disk
			//  we add the lastest state (from disk) as first root
			//  this contains the pinnings (state updates below) from the last time
			//  we need this to have a full hisotry of heads
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
					reducer.insert_snapshot(state, heads.clone());
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
		let builtin_cores = Cores::default()
			.built_in_native_mapping()
			.into_iter()
			.map(|(cid, _)| cid)
			.collect();
		EncryptionReferenceMode::DisallowPlainExcept(builtin_cores)
	} else {
		EncryptionReferenceMode::Warning
	};
	Ok(EncryptedBlockStorage::new(storage.clone(), key.fetch().await?.into(), Default::default(), Default::default())
		.with_encryption_reference_mode(reference_mode))
}

/// Setup the Local CO by adding cores.
#[tracing::instrument(level = tracing::Level::TRACE, err, skip(runtime, reducer, storage))]
async fn setup_local_co<S, R>(
	storage: &S,
	runtime: &RuntimePool,
	identity: &LocalIdentity,
	reducer: &mut Reducer<S, R>,
	settings: &ApplicationSettings,
) -> Result<(), anyhow::Error>
where
	S: ExtendedBlockStorage + Sync + Send + Clone + 'static,
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
	#[cfg(feature = "pinning")]
	cores.insert(
		CO_CORE_NAME_STORAGE.to_owned(),
		co_core_co::Core {
			binary: Cores::default().binary(CO_CORE_STORAGE).expect(CO_CORE_STORAGE),
			tags: tags!("core": CO_CORE_STORAGE),
			state: create_storage_core_state(storage, settings).await?,
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
		binary: Cores::default().binary(CO_CORE_CO).expect(CO_CORE_CO),
	};
	reducer.push(storage, runtime, identity, CO_CORE_NAME_CO, &action).await?;

	// done
	Ok(())
}

#[cfg(feature = "pinning")]
async fn create_storage_core_state<S: BlockStorage + Clone + 'static>(
	storage: &S,
	settings: &ApplicationSettings,
) -> Result<Option<Cid>, anyhow::Error> {
	Ok(co_core_storage::Storage::initial_state(
		storage,
		vec![
			co_core_storage::StorageAction::PinCreate(
				CoPinningKey::State.to_string(&CO_ID_LOCAL.into()),
				settings.setting_co_default_max_state(),
				Default::default(),
			),
			co_core_storage::StorageAction::PinCreate(
				CoPinningKey::Log.to_string(&CO_ID_LOCAL.into()),
				settings.setting_co_default_max_log(),
				Default::default(),
			),
		],
	)
	.await?
	.into())
}
