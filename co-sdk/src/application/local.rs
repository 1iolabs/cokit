use super::{application::ApplicationSettings, identity::create_identity_resolver, reducer::ReducerChangedHandler};
#[cfg(feature = "pinning")]
use crate::types::{
	co_pinning_key::CoPinningKey,
	cores::{CO_CORE_NAME_STORAGE, CO_CORE_STORAGE},
};
use crate::{
	library::{
		local_secret::{FileLocalSecret, KeychainLocalSecret, LocalSecret, MemoryLocalSecret},
		locals::{ApplicationLocal, FileLocals, Locals, MemoryLocals},
		storage_cleanup::storage_cleanup,
		to_external_cid::{to_external_cid_opt_force, to_external_cids_opt_map_force},
	},
	reducer::{change::reference_writer::lastest_storage_reference, core_resolver::dynamic::DynamicCoreResolver},
	services::reducer::ReducerFlush,
	types::{
		co_dispatch::CoDispatch,
		co_reducer_context::{CoReducerContext, CoReducerFeature},
	},
	ApplicationMessage, CoReducer, CoReducerState, CoStorage, CoreResolver, Cores, DynamicCoDate, Reducer,
	ReducerBuilder, ReducerChangeContext, Runtime, TaskSpawner, CO_CORE_KEYSTORE, CO_CORE_MEMBERSHIP, CO_CORE_NAME_CO,
	CO_CORE_NAME_KEYSTORE, CO_CORE_NAME_MEMBERSHIP,
};
use anyhow::anyhow;
use async_trait::async_trait;
use cid::Cid;
use co_actor::ActorHandle;
#[cfg(feature = "pinning")]
use co_core_storage::StorageAction;
use co_identity::{Identity, LocalIdentity, PrivateIdentity, PrivateIdentityBox};
use co_log::Log;
use co_primitives::{tags, CloneWithBlockStorageSettings, Did, MappedCid, OptionMappedCid};
use co_runtime::RuntimePool;
use co_storage::{
	BlockStorage, BlockStorageContentMapping, EncryptedBlockStorage, EncryptionReferenceMode, ExtendedBlockStorage,
};
use futures::{pin_mut, Stream, StreamExt, TryStreamExt};
use serde::Serialize;
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
				)
				.await?
				.1)
			},
		}
	}
}

#[derive(Debug, Clone)]
struct LocalCoInstance<L> {
	identifier: String,
	storage: CoStorage,
	encrypted_storage: Option<EncryptedBlockStorage<CoStorage>>,
	locals: L,
	#[cfg(feature = "pinning")]
	reference_writer: Option<(ReducerDispatchContext, crate::reducer::change::reference_writer::ReferenceWriter)>,
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
			identifier: local_co.settings.identifier.clone(),
			#[cfg(feature = "pinning")]
			reference_writer: None,
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
			flush.reference_writer = Some((
				ReducerDispatchContext {
					core: CO_CORE_NAME_STORAGE.to_owned(),
					identity: local_co.identity.clone().boxed(),
					runtime: runtime.clone(),
				},
				crate::reducer::change::reference_writer::ReferenceWriter::new(
					Some(CO_ID_LOCAL.into()),
					CoReducerState::new_reducer(&reducer),
				),
			));
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
						tracing::trace!(?local_state, ?local.mapping, "local-watch");
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
							tracing::trace!(?previous_state, ?next_state, ?local_state, "local-watch-join");
						},
						Err(err) => tracing::warn!(?err, ?local_state, "local-watch-join-failed"),
					}
				}
			});
		}

		// result
		Ok((result, co_reducer))
	}

	/// Write state to disk.
	/// Returns false and does nothing if reducer is empty.
	pub async fn write<S>(
		&mut self,
		storage: &S,
		reducer_state: CoReducerState,
		mapping: Option<Cid>,
	) -> Result<bool, anyhow::Error>
	where
		S: ExtendedBlockStorage + BlockStorageContentMapping + Clone + Sync + Send + 'static,
	{
		if let Some(state) = reducer_state.state() {
			// heads
			let plain_heads_map = to_external_cids_opt_map_force(storage, reducer_state.heads())
				.await
				.ok_or_else(|| anyhow!("Failed to map heads: {:?}", reducer_state.heads()))?;

			// state
			let plain_state = to_external_cid_opt_force(storage, Some(state))
				.await
				.ok_or_else(|| anyhow!("Failed to map state: {:?}", state))?;

			// make sure the root mappings are available in parent storage
			// TODO: remove? not belongs here?
			if let Some(encrypted_storage) = &self.encrypted_storage {
				encrypted_storage
					.insert_mappings(
						[(state, plain_state)]
							.into_iter()
							.chain(plain_heads_map.clone())
							.map(MappedCid::from),
					)
					.await;
			}

			// create format
			let local = ApplicationLocal::new(plain_heads_map.values().cloned().collect(), plain_state, mapping);

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
impl<L, S, R> ReducerChangedHandler<S, R> for LocalCoInstance<L>
where
	S: ExtendedBlockStorage + BlockStorageContentMapping + Clone + Sync + Send + 'static,
	R: CoreResolver<S> + Send + Sync + 'static,
	L: Locals + Clone + Debug + Send + Sync + 'static,
{
	async fn on_state_changed(
		&mut self,
		storage: &S,
		reducer: &Reducer<S, R>,
		_context: ReducerChangeContext,
	) -> Result<(), anyhow::Error> {
		let reducer_state = CoReducerState::new(*reducer.state(), reducer.heads().to_owned());
		self.write(storage, reducer_state, None).await?;
		Ok(())
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
		removed_blocks: BTreeSet<OptionMappedCid>,
	) -> anyhow::Result<()> {
		// write references
		//  we execute references for the previous state as the references always one state late
		//  we always want the state pinned that actually was returned by the last operation
		//  this is not possible if we pin the intermediate state generated by pin/reference/cleanup.
		#[cfg(feature = "pinning")]
		if let Some((context, reference_writer)) = &mut self.reference_writer {
			let mut dispatch = ReducerDispatch { context, reducer, storage };

			// TODO: Insert pins for new_roots

			// remove blocks
			crate::library::storage_dispatch_remove::storage_dispatch_remove(
				&mut dispatch,
				futures::stream::iter(removed_blocks),
				<S::StoreParams as co_primitives::StoreParams>::MAX_BLOCK_SIZE,
			)
			.await?;

			// resolve previous
			//  find latest pin using the reducer state loaded from disk (disk-1 ≏ disk)
			let next_reducer_state = reference_writer.previous_reducer_state().clone();
			let previous_reference_state = CoReducerState::new(
				lastest_storage_reference(
					storage,
					next_reducer_state.co(),
					&reference_writer.pinning_key(CoPinningKey::State),
				)
				.await?,
				BTreeSet::default(),
			);
			reference_writer.set_previous_reducer_state(previous_reference_state.clone());

			// log
			tracing::trace!(
				next_state = ?next_reducer_state.state(),
				previous_state = ?previous_reference_state.state(),
				"local-flush"
			);

			// apply
			let next_state = reference_writer
				.write(&mut dispatch, storage, next_reducer_state.clone(), true)
				.await?;

			// cleanup
			storage_cleanup(&mut dispatch, storage, next_state.into()).await?;

			// write including the pinning changes
			reference_writer.set_previous_reducer_state(CoReducerState::new_reducer(reducer));
		}

		// write local
		let reducer_state = CoReducerState::new_reducer(reducer);
		self.write(storage, reducer_state, None).await?;

		Ok(())
	}
}

#[derive(Debug, Clone)]
struct ReducerDispatchContext {
	identity: PrivateIdentityBox,
	runtime: Runtime,
	core: String,
}
struct ReducerDispatch<'a, 'b, 'c, S, R> {
	context: &'a ReducerDispatchContext,
	storage: &'b S,
	reducer: &'c mut Reducer<S, R>,
}
#[async_trait]
impl<'a, 'b, 'c, A, S, R> CoDispatch<A> for ReducerDispatch<'a, 'b, 'c, S, R>
where
	S: ExtendedBlockStorage + BlockStorageContentMapping + Clone + Sync + Send + 'static,
	R: CoreResolver<S> + Send + Sync + 'static,
	A: Serialize + Send + Sync,
{
	async fn dispatch(&mut self, action: &A) -> Result<Option<Cid>, anyhow::Error> {
		Ok(self
			.reducer
			.push(&self.storage, self.context.runtime.runtime(), &self.context.identity, &self.context.core, action)
			.await?)
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
			StorageAction::PinCreate(
				CoPinningKey::State.to_string(&CO_ID_LOCAL.into()),
				settings.setting_co_default_max_state(),
				Default::default(),
			),
			StorageAction::PinCreate(
				CoPinningKey::Log.to_string(&CO_ID_LOCAL.into()),
				settings.setting_co_default_max_log(),
				Default::default(),
			),
		],
	)
	.await?
	.into())
}
