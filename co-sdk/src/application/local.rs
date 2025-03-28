use super::{
	application::ApplicationSettings, co_context::CoPinningKey, identity::create_identity_resolver,
	reducer::ReducerChangedHandler,
};
use crate::{
	library::{
		local_secret::{FileLocalSecret, KeychainLocalSecret, LocalSecret, MemoryLocalSecret},
		locals::{ApplicationLocal, FileLocals, Locals, MemoryLocals},
		to_external_cid::{to_external_cid_opt, to_external_cids_opt_map},
		to_internal_cid::{to_internal_cid, to_internal_cids},
	},
	reducer::core_resolver::dynamic::DynamicCoreResolver,
	types::{
		co_reducer::{CoReducerContext, CoReducerContextRef},
		cores::{CO_CORE_NAME_CO, CO_CORE_STORAGE},
	},
	CoReducer, CoStorage, CoreResolver, Cores, Reducer, ReducerBuilder, ReducerChangeContext, Runtime, TaskSpawner,
	CO_CORE_KEYSTORE, CO_CORE_MEMBERSHIP, CO_CORE_NAME_KEYSTORE, CO_CORE_NAME_MEMBERSHIP, CO_CORE_NAME_STORAGE,
};
use anyhow::anyhow;
use async_trait::async_trait;
use cid::Cid;
use co_core_storage::StorageAction;
use co_identity::{Identity, LocalIdentity};
use co_log::Log;
use co_primitives::{tags, Did};
use co_runtime::RuntimePool;
use co_storage::{BlockStorage, BlockStorageContentMapping, EncryptedBlockStorage, EncryptionReferenceMode};
use futures::{pin_mut, stream, Stream, StreamExt, TryStreamExt};
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
		core_resolver: impl FnOnce(CoReducerContextRef) -> R,
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
		let watcher = self.settings.setting_co_local_watch();
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
	encrypted_storage: EncryptedBlockStorage<CoStorage>,
	locals: L,
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
		key: Box<dyn LocalSecret + Send + Sync + 'static>,
		core_resolver: impl FnOnce(CoReducerContextRef) -> R,
		watcher: bool,
	) -> Result<(Self, CoReducer), anyhow::Error>
	where
		R: CoreResolver<CoStorage> + Send + Sync + 'static,
	{
		// create storage
		let encrypted_storage: EncryptedBlockStorage<CoStorage> = create_encrypted_storage(storage, key, true).await?;
		let storage = CoStorage::new(encrypted_storage.clone());

		// create log
		let log = Log::new(CO_ID_LOCAL.as_bytes().to_vec(), create_identity_resolver(), Default::default());

		// result
		let result = Self {
			locals: locals.clone(),
			encrypted_storage: encrypted_storage.clone(),
			identifier: local_co.settings.identifier.clone(),
		};
		let context = Arc::new(result.clone());

		// create builder
		let mut builder = ReducerBuilder::new(DynamicCoreResolver::new(core_resolver(context.clone())), log)
			.with_initialize(local_co.initialize);

		// load locals as snapshots
		//  the latest heads will be automatically determined by the reducer
		{
			let locals_stream = result.load_locals();
			pin_mut!(locals_stream);
			while let Some(next) = locals_stream.next().await {
				let (state, heads) = next?;

				// apply to builder as snapshot
				builder = builder.with_snapshot(state, heads);
			}
		}

		// create reducer
		let mut reducer = builder.build(&storage, runtime.runtime()).await?;

		// write
		reducer.add_change_handler(Box::new(result.clone()));

		// create empty
		if reducer.is_empty() {
			setup_local_co(&storage, runtime.runtime(), &local_co.identity, &mut reducer, &local_co.settings).await?;
		}

		// reducer
		let co_reducer = CoReducer::new(CO_ID_LOCAL.into(), None, storage, runtime, reducer, context);

		// watch
		if watcher {
			let watch_reducer: CoReducer = co_reducer.clone();
			let watch_locals = result.locals.clone();
			let watch_encrypted_storage = encrypted_storage.clone();
			tasks.spawn(async move {
				let watcher = watch_locals.watch().take_until(shutdown.clone().cancelled_owned());
				pin_mut!(watcher);
				while let Some(local) = watcher.next().await {
					// convert heads to unencrypted
					let local_heads = match stream::iter(local.heads.iter())
						.then(|cid| async {
							watch_encrypted_storage
								.to_mapped(cid)
								.await
								.ok_or_else(|| anyhow!("Map head failed: {}", *cid))
						})
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
		}

		// result
		Ok((result, co_reducer))
	}

	/// Write state to disk.
	/// Returns false and does nothing if reducer is empty.
	pub async fn write<S, R>(
		&mut self,
		storage: &S,
		reducer: &Reducer<S, R>,
		mapping: Option<Cid>,
	) -> Result<bool, anyhow::Error>
	where
		S: BlockStorage + BlockStorageContentMapping + Clone + Sync + Send + 'static,
		R: CoreResolver<S> + Send + Sync + 'static,
	{
		if let Some(state) = reducer.state() {
			// heads
			let plain_heads_map = to_external_cids_opt_map(storage, reducer.heads().clone())
				.await
				.ok_or_else(|| anyhow!("Failed to map heads: {:?}", reducer.heads()))?;

			// state
			let plain_state = to_external_cid_opt(storage, Some(*state))
				.await
				.ok_or_else(|| anyhow!("Failed to map state: {:?}", state))?;

			// make sure the root mappings are available in parent storage
			self.encrypted_storage
				.insert_mappings([(*state, plain_state)].into_iter().chain(plain_heads_map.clone()))
				.await;

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

	fn load_locals(&self) -> impl Stream<Item = Result<(Cid, BTreeSet<Cid>), anyhow::Error>> + '_ {
		async_stream::try_stream! {
			for local in self.locals.get().await? {
				// load additional encryption mappings
				if let Some(mapping) = &local.mapping {
					self.encrypted_storage.load_mapping(mapping).await?;
				}

				// convert state/heads to internal
				let state = to_internal_cid(&self.encrypted_storage, local.state).await;
				let heads = to_internal_cids(&self.encrypted_storage, local.heads.clone()).await;

				// apply
				yield (state, heads)
			}
		}
	}
}
#[async_trait]
impl<L, S, R> ReducerChangedHandler<S, R> for LocalCoInstance<L>
where
	S: BlockStorage + BlockStorageContentMapping + Clone + Sync + Send + 'static,
	R: CoreResolver<S> + Send + Sync + 'static,
	L: Locals + Clone + Debug + Send + Sync + 'static,
{
	async fn on_state_changed(
		&mut self,
		storage: &S,
		reducer: &Reducer<S, R>,
		_context: ReducerChangeContext,
	) -> Result<(), anyhow::Error> {
		self.write(storage, reducer, None).await?;
		Ok(())
	}
}
#[async_trait]
impl<L> CoReducerContext for LocalCoInstance<L>
where
	L: Locals + Clone + Debug + Send + Sync + 'static,
{
	fn storage(&self, _force_local: bool) -> CoStorage {
		// the LocalCo never uses networking and is always encrypted
		CoStorage::new(self.encrypted_storage.clone())
	}

	async fn refresh(&self, _parent: CoReducer, co: CoReducer) -> anyhow::Result<()> {
		// read and apply locals
		//  this will manually re-read all local files
		self.load_locals()
			.try_for_each(|(state, heads)| {
				let co = &co;
				async move {
					co.insert_snapshot(state, heads.clone()).await?;
					co.join(&heads).await?;
					Ok(())
				}
			})
			.await?;

		// done
		Ok(())
	}

	/// Clear reducer caches.
	async fn clear(&self, co: CoReducer) {
		let mut reducer = co.reducer.write().await;

		// remember root cids
		let mut roots = BTreeSet::new();
		roots.extend(reducer.state().clone().into_iter());
		roots.extend(reducer.heads().clone().into_iter());

		// clear log
		reducer.log_mut().clear();

		// clear reducer
		reducer.clear();

		// clear storage
		self.encrypted_storage.clear_mapping(roots).await;
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
	// we have plain references as we may have unencrypted shared COs but all references to it should be Weak.
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
#[tracing::instrument(err, skip(runtime, reducer, storage))]
async fn setup_local_co<S, R>(
	storage: &S,
	runtime: &RuntimePool,
	identity: &LocalIdentity,
	reducer: &mut Reducer<S, R>,
	settings: &ApplicationSettings,
) -> Result<(), anyhow::Error>
where
	S: BlockStorage + Sync + Send + Clone + 'static,
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
		CO_CORE_NAME_STORAGE.to_owned(),
		co_core_co::Core {
			binary: Cores::default().binary(CO_CORE_STORAGE).expect(CO_CORE_STORAGE),
			tags: tags!("core": CO_CORE_STORAGE),
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
	reducer.push(storage, runtime, identity, CO_CORE_NAME_CO, &action).await?;

	// setup storage core
	reducer
		.push(
			storage,
			runtime,
			identity,
			CO_CORE_NAME_STORAGE,
			&StorageAction::PinCreate(
				CoPinningKey::State.to_string(&CO_ID_LOCAL.into()),
				settings.setting_co_default_max_state(),
				Default::default(),
			),
		)
		.await?;
	reducer
		.push(
			storage,
			runtime,
			identity,
			CO_CORE_NAME_STORAGE,
			&StorageAction::PinCreate(
				CoPinningKey::Log.to_string(&CO_ID_LOCAL.into()),
				settings.setting_co_default_max_log(),
				Default::default(),
			),
		)
		.await?;

	// done
	Ok(())
}
