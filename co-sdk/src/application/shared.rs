use super::identity::create_identity_resolver;
use crate::{
	find_membership,
	library::{find_co_secret::find_co_secret_by_reference, membership_all_heads::membership_all_heads},
	reducer::{
		change::membership_writer::MembershipWriter,
		core_resolver::{dynamic::DynamicCoreResolver, log::LogCoreResolver},
	},
	services::{
		reducer::{FlushInfo, ReducerBlockStorage, ReducerFlush},
		reducers::ReducerStorage,
	},
	types::co_reducer_context::{CoReducerContext, CoReducerFeature},
	ApplicationMessage, CoCoreResolver, CoDate, CoReducer, CoReducerState, CoStorage, CoUuid, Cores, DynamicCoDate,
	Reducer, ReducerBuilder, Runtime, TaskSpawner, CO_CORE_CO, CO_CORE_NAME_CO, CO_CORE_NAME_KEYSTORE,
	CO_CORE_NAME_MEMBERSHIP,
};
use anyhow::anyhow;
use async_trait::async_trait;
use cid::Cid;
use co_actor::ActorHandle;
use co_core_co::{CoAction, Core, CreateAction};
use co_core_keystore::{Key, KeyStoreAction};
use co_core_membership::{Membership, MembershipsAction};
use co_identity::PrivateIdentity;
use co_log::{IdentityEntryVerifier, Log};
use co_primitives::{tags, BlockStorageSettings, CloneWithBlockStorageSettings, CoId, OptionMappedCid, Tags};
use co_storage::{
	unixfs_add, Algorithm, BlockStorageContentMapping, EncryptedBlockStorage, EncryptionReferenceMode, Secret,
};
use futures::io::Cursor;
use serde::{Deserialize, Serialize};
use std::{
	collections::{BTreeMap, BTreeSet},
	fmt::Debug,
	sync::Arc,
};

/// Shared CO Builder.
/// The Shared CO state is stored in a membership of an other CO (typically the Local CO).
pub struct SharedCoBuilder {
	parent: CoReducer,
	keystore_core_name: String,
	membership_core_name: String,
	membership: Membership,
	initialize: bool,
}
impl SharedCoBuilder {
	pub fn new(parent: CoReducer, membership: Membership) -> Self {
		Self {
			parent,
			membership,
			membership_core_name: CO_CORE_NAME_MEMBERSHIP.to_string(),
			keystore_core_name: CO_CORE_NAME_KEYSTORE.to_string(),
			initialize: true,
		}
	}

	pub fn with_membership_core_name(self, membership_core_name: String) -> Self {
		Self { membership_core_name, ..self }
	}

	pub fn with_keystore_core_name(self, keystore_core_name: String) -> Self {
		Self { keystore_core_name, ..self }
	}

	pub fn with_initialize(self, initialize: bool) -> Self {
		Self { initialize, ..self }
	}

	/// Read (latest) secret from parent CO.
	pub async fn secret(&self) -> anyhow::Result<Option<co_primitives::Secret>> {
		if let Some(key_reference) = &self.membership.key {
			Ok(Some(find_co_secret_by_reference(&self.parent, key_reference, Some(&self.keystore_core_name)).await?))
		} else {
			Ok(None)
		}
	}

	pub async fn build<I>(
		self,
		tasks: TaskSpawner,
		storage: ReducerStorage,
		runtime: Runtime,
		identity: I,
		core_resolver: DynamicCoreResolver<CoStorage>,
		date: DynamicCoDate,
		application_handle: ActorHandle<ApplicationMessage>,
		#[cfg(feature = "pinning")] pinning: crate::library::storage_pinning::StoragePinningContext,
	) -> Result<CoReducer, anyhow::Error>
	where
		I: PrivateIdentity + Debug + Send + Sync + Clone + 'static,
	{
		let encrypted_storage = storage.encrypted_storage().cloned();

		// network
		//  we want to layer the network storage before the encryption to we only send encrypted blocks over network
		//  `BASE <- NETWORK <- ENCRYPTION``
		let network_storage = {
			// get base storage
			let base_storage = if let Some(encrypted_storage) = storage.encrypted_storage() {
				encrypted_storage.storage().clone()
			} else {
				storage.storage().clone()
			};

			// create network storage
			let network_storage = ReducerBlockStorage::new(
				self.parent.id().clone(),
				self.membership.id.clone(),
				base_storage,
				application_handle.clone(),
				Default::default(),
			);

			// create encrypted storage which uses the network storage as base
			// note: it uses the same mapping as the instance withput networking
			let next_storage = if let Some(encrypted_storage) = storage.encrypted_storage() {
				let mut encrypted_storage = encrypted_storage.clone();
				encrypted_storage.set_storage(CoStorage::new(network_storage));
				CoStorage::new(encrypted_storage)
			} else {
				CoStorage::new(network_storage)
			};

			// result
			next_storage
		};

		// context
		let context = Arc::new(SharedContext {
			storage: match &encrypted_storage {
				Some(storage) => storage.storage().clone(),
				None => storage.storage().clone(),
			},
			encrypted_storage: encrypted_storage.clone(),
			network_storage,
			id: self.membership.id.clone(),
		});
		let co_storage = context.storage(false);

		// // states
		// let states = stream::iter(self.membership.state.clone())
		// 	.then(|co_state| {
		// 		let context = &context;
		// 		async move {
		// 			// get (unencrypted) state/heads
		// 			let state = context.to_internal_cid(co_state.state).await?;
		// 			let heads: BTreeSet<Cid> = stream::iter(co_state.heads.iter())
		// 				.then(|cid| async { context.to_internal_cid(*cid).await })
		// 				.try_collect()
		// 				.await?;
		// 			Result::<(Cid, BTreeSet<Cid>), anyhow::Error>::Ok((state, heads))
		// 		}
		// 	})
		// 	.try_collect::<BTreeSet<_>>()
		// 	.await?;
		// let latest_state = if states.len() == 1 { states.first().cloned() } else { None };

		// log
		let log = Log::new(
			self.membership.id.as_str().as_bytes().to_vec(),
			IdentityEntryVerifier::new(create_identity_resolver()),
			Default::default(),
		);

		// reducer
		let mut reducer_builder = ReducerBuilder::new(core_resolver, log).with_initialize(false);
		let parent_storage = self.parent.storage();
		for co_state in self.membership.state.iter() {
			// reducer state
			//  note: external states (from invite) get mapped in the reducer initialize to not have network request
			//   before the reducer actor is spawned.
			let reducer_state = CoReducerState::from_co_state(&parent_storage, co_state).await?;

			// load state/heads mappings from parent into our mapping
			if let Some(parent_mappings) = reducer_state.to_external_mapping(&parent_storage).await {
				co_storage.insert_mappings(parent_mappings).await;
			}

			// add to builder
			if let Some((state, heads)) = reducer_state.some() {
				reducer_builder = reducer_builder.with_snapshot(state, heads);
			}
		}
		let reducer = reducer_builder.build(&co_storage, runtime.runtime(), date).await?;

		// setup auto write to parent co
		let membership_writer = MembershipWriter::new(
			self.membership.id.clone(),
			self.parent.clone(),
			self.membership_core_name,
			identity.clone().boxed(),
			encrypted_storage.clone(),
			self.membership.state.clone(),
		);

		// build flush
		let flush = SharedFlush {
			membership_writer,
			#[cfg(feature = "pinning")]
			pinning,
		};

		// result
		let application_identifier = self
			.parent
			.handle()
			.tags()
			.string("application")
			.ok_or_else(|| anyhow!("Missing parent tag: application"))?
			.to_owned();
		Ok(CoReducer::spawn(
			application_handle,
			application_identifier,
			self.membership.id,
			Some(self.parent.id().clone()),
			context.storage(false),
			tasks,
			runtime,
			reducer,
			context,
			Box::new(flush),
			self.initialize,
		)?)
	}
}

struct SharedFlush {
	membership_writer: MembershipWriter,
	#[cfg(feature = "pinning")]
	pinning: crate::library::storage_pinning::StoragePinningContext,
}
#[async_trait]
impl ReducerFlush<CoStorage, DynamicCoreResolver<CoStorage>> for SharedFlush {
	async fn flush(
		&mut self,
		storage: &CoStorage,
		reducer: &mut Reducer<CoStorage, DynamicCoreResolver<CoStorage>>,
		_info: &FlushInfo,
		_new_roots: Vec<CoReducerState>,
		_removed_blocks: BTreeSet<OptionMappedCid>,
	) -> anyhow::Result<()> {
		let reducer_state = CoReducerState::new_reducer(reducer);

		// membership
		self.membership_writer.write(storage, reducer_state.clone()).await?;

		// pinning
		#[cfg(feature = "pinning")]
		{
			let new_roots = _new_roots;
			let removed_blocks = _removed_blocks;
			let parent = &self.membership_writer.parent;

			// compute
			let parent_pinning_state = crate::library::storage_pinning::storage_pinning(
				&self.pinning,
				None,
				&parent.storage(),
				parent.reducer_state().await,
				parent.id(),
				storage,
				new_roots,
				removed_blocks,
			)
			.await?;

			// apply
			if let Some(parent_pinning_state) = parent_pinning_state {
				parent.join_state(parent_pinning_state).await?;
			}
		}

		Ok(())
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCo {
	pub id: CoId,
	pub name: String,
	pub algorithm: Option<Algorithm>,
	cores: BTreeMap<String, CoreSource>,
}
impl CreateCo {
	pub fn new(id: impl Into<CoId>, name: Option<String>) -> Self {
		let id = id.into();
		let name = name.unwrap_or_else(|| id.to_string());
		CreateCo { id, name, algorithm: Some(Default::default()), cores: Default::default() }
	}

	pub fn generate(name: String) -> Self {
		CreateCo {
			id: uuid::Uuid::new_v4().to_string().into(),
			name,
			algorithm: Some(Default::default()),
			cores: Default::default(),
		}
	}

	pub fn with_core(mut self, core_name: &str, core_type: &str, core_binary: Cid) -> Self {
		self.cores
			.insert(core_name.to_owned(), CoreSource::Reference(tags!("type": core_type), core_binary));
		self
	}

	pub fn with_core_bytes(mut self, core_name: &str, core_type: &str, core_binary: impl Into<Vec<u8>>) -> Self {
		self.cores
			.insert(core_name.to_owned(), CoreSource::Bytes(tags!("type": core_type), core_binary.into()));
		self
	}

	pub fn with_algorithm(mut self, algorithm: Option<Algorithm>) -> Self {
		self.algorithm = algorithm;
		self
	}

	pub fn with_public(self, public: bool) -> Self {
		self.with_algorithm(if public { None } else { Some(Algorithm::default()) })
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum CoreSource {
	Reference(Tags, Cid),
	Bytes(Tags, Vec<u8>),
}

#[derive(Debug)]
struct SharedContext {
	id: CoId,

	/// The base storage.
	///
	/// # Layers
	/// `BASE`
	storage: CoStorage,

	/// The encrypted storage.
	/// If encryption is enabled.
	/// Note: Without networking!
	///
	/// # Layers
	/// `BASE + ENCRYPTION`
	encrypted_storage: Option<EncryptedBlockStorage<CoStorage>>,

	/// The networking storage.
	/// If network is enabled.
	///
	/// # Layers
	/// `BASE + NETWORK + ENCRYPTION`
	network_storage: CoStorage,
}
impl SharedContext {
	/// Update `co` membership if necessary.
	async fn update_membership(&self, parent: CoReducer, co: CoReducer) -> Result<(), anyhow::Error> {
		if let Some(membership) = find_membership(&parent, co.id()).await? {
			let storage = co.storage();
			let parent_storage = parent.storage();
			let co_heads = co.heads().await;
			let membetship_heads = membership_all_heads(&parent_storage, membership.state.iter()).await?;
			if co_heads != membetship_heads {
				tracing::info!(co = ?co.id(), from = ?co_heads, to = ?membership, "membership-update");
				for state in membership.state.iter() {
					// encryption mapping
					if let (Some(storage), Some(cid)) = (&self.encrypted_storage, &state.encryption_mapping) {
						storage.load_mapping(&cid).await?;
					}

					// state
					let reducer_state = CoReducerState::from_co_state(&parent_storage, state).await?;

					// mappings
					if let Some(mappings) = reducer_state.to_external_mapping(&parent_storage).await {
						storage.insert_mappings(mappings).await;
					}

					// join
					co.join_state(reducer_state).await?;
				}
			}
		}
		Ok(())
	}
}
#[async_trait]
impl CoReducerContext for SharedContext {
	fn storage(&self, force_local: bool) -> CoStorage {
		// network
		if !force_local {
			return self.network_storage.clone();
		}

		// encrypted
		if let Some(encrypted_storage) = &self.encrypted_storage {
			return CoStorage::new(encrypted_storage.clone());
		}

		// base
		self.storage.clone()
	}

	async fn refresh(&self, parent: CoReducer, co: CoReducer) -> anyhow::Result<()> {
		if co.id() != &self.id {
			return Err(anyhow!("Invalid co {} expected {}", co.id(), &self.id));
		}
		if co.parent_id() != Some(parent.id()) {
			return Err(anyhow!("Invalid parent co {} for {}", parent.id(), co.id()));
		}
		self.update_membership(parent, co).await
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
			CoReducerFeature::Network => true,
			CoReducerFeature::Encryption => self.encrypted_storage.is_some(),
			_ => false,
		}
	}
}

pub struct SharedCoCreator {
	parent: CoReducer,
	keystore_core_name: String,
	membership_core_name: String,
	#[cfg(feature = "pinning")]
	storage_core_name: String,
	co: CreateCo,
}
impl SharedCoCreator {
	pub fn new(parent: CoReducer, co: CreateCo) -> Self {
		Self {
			parent,
			co,
			membership_core_name: CO_CORE_NAME_MEMBERSHIP.to_string(),
			keystore_core_name: CO_CORE_NAME_KEYSTORE.to_string(),
			#[cfg(feature = "pinning")]
			storage_core_name: crate::CO_CORE_NAME_STORAGE.to_string(),
		}
	}

	pub fn with_membership_core_name(self, membership_core_name: String) -> Self {
		Self { membership_core_name, ..self }
	}

	pub fn with_keystore_core_name(self, keystore_core_name: String) -> Self {
		Self { keystore_core_name, ..self }
	}

	pub fn with_storage_core_name(self, _storage_core_name: String) -> Self {
		#[cfg(feature = "pinning")]
		return Self { storage_core_name: _storage_core_name, ..self };
		#[cfg(not(feature = "pinning"))]
		return self;
	}

	/// TODO: Cleanup when something fails?
	pub async fn create<I>(
		self,
		storage: CoStorage,
		runtime: Runtime,
		identity: I,
		date: impl CoDate,
		uuid: impl CoUuid,
	) -> Result<CoId, anyhow::Error>
	where
		I: PrivateIdentity + Clone + Debug + Send + Sync + 'static,
	{
		// storage
		let (co_storage, encrypted_storage): (CoStorage, Option<(EncryptedBlockStorage<CoStorage>, String, Secret)>) =
			match self.co.algorithm {
				Some(algorithm) => {
					let key_uri = format!("urn:co:{}:{}", self.co.id, uuid.uuid());
					let key = algorithm.generate_serect();
					let builtin_cores = Cores::default()
						.built_in_native_mapping()
						.into_iter()
						.map(|(cid, _)| cid)
						.collect();
					let result_storage =
						EncryptedBlockStorage::new(storage.clone(), key.clone(), algorithm, Default::default())
							.with_encryption_reference_mode(EncryptionReferenceMode::DisallowExcept(builtin_cores));
					(CoStorage::new(result_storage.clone()), Some((result_storage, key_uri, key)))
				},
				None => (storage.clone_with_settings(BlockStorageSettings::new().with_detached()), None),
			};

		// log
		let log = Log::new(
			self.co.id.as_str().as_bytes().to_vec(),
			IdentityEntryVerifier::new(create_identity_resolver()),
			Default::default(),
		);

		// reducer
		let core_resolver = CoCoreResolver::default();
		let core_resolver = LogCoreResolver::new(core_resolver, self.co.id.clone());
		let mut reducer = ReducerBuilder::new(core_resolver, log)
			.build(&co_storage, runtime.runtime(), date)
			.await?;

		// initialize
		let mut create = CreateAction::new(
			self.co.id.to_owned(),
			self.co.name.to_owned(),
			Cores::default().binary(CO_CORE_CO).expect(CO_CORE_CO),
		)
		.with_participant(identity.identity().to_owned(), tags!())
		.with_key(encrypted_storage.as_ref().map(|(_, key_uri, _)| key_uri.clone()));
		for (core_name, core_source) in self.co.cores {
			let core = match core_source {
				CoreSource::Reference(tags, binary) => Core { binary, tags, state: None },
				CoreSource::Bytes(tags, binary_bytes) => {
					let mut binary_stream = Cursor::new(&binary_bytes);
					let binary = unixfs_add(&co_storage, &mut binary_stream)
						.await?
						.pop()
						.ok_or(anyhow!("Add Core binary failed {}", binary_bytes.len()))?;
					Core { binary, tags, state: None }
				},
			};
			create = create.with_core(core_name, core);
		}
		reducer
			.push(&co_storage, runtime.runtime(), &identity, CO_CORE_NAME_CO, &CoAction::Create(create))
			.await?;
		let reducer_state = CoReducerState::new(reducer.state().clone(), reducer.heads().clone());

		// store key in parent co
		let (key_uri, _encrypted_storage) = if let Some((encrypted_storage, key_uri, secret)) = encrypted_storage {
			let key = Key {
				uri: key_uri.clone(),
				name: format!("co ({})", self.co.name),
				description: "".to_owned(),
				secret: co_core_keystore::Secret::SharedKey(secret.into()),
				tags: tags!(),
			};
			self.parent
				.push(&identity, &self.keystore_core_name, &KeyStoreAction::Set(key))
				.await?;
			(Some(key_uri), Some(encrypted_storage))
		} else {
			(None, None)
		};

		// add membership to parent co
		let parent_storage = self.parent.storage();
		let (state, _mappings) = reducer_state
			.to_co_state(&parent_storage, &co_storage)
			.await?
			.ok_or(anyhow::anyhow!("Expected state after create"))?;
		self.parent
			.push(
				&identity,
				&self.membership_core_name,
				&MembershipsAction::Join(Membership {
					id: self.co.id.to_owned(),
					did: identity.identity().to_owned(),
					state: BTreeSet::from([state]),
					key: key_uri,
					membership_state: co_core_membership::MembershipState::Active,
					tags: tags!(),
				}),
			)
			.await?;

		// pin
		#[cfg(feature = "pinning")]
		{
			// add pin to parent co
			let pin_state = co_core_storage::StorageAction::PinCreate(
				crate::types::co_pinning_key::CoPinningKey::State.to_string(&self.co.id),
				co_core_storage::PinStrategy::Unlimited,
				Default::default(),
			);
			let pin_log = co_core_storage::StorageAction::PinCreate(
				crate::types::co_pinning_key::CoPinningKey::Log.to_string(&self.co.id),
				co_core_storage::PinStrategy::Unlimited,
				Default::default(),
			);
			self.parent.push(&identity, &self.storage_core_name, &pin_log).await?;
			self.parent.push(&identity, &self.storage_core_name, &pin_state).await?;

			// pin initial state
			crate::reducer::change::reference_writer::write_storage_references(
				co_storage.clone(),
				&mut self
					.parent
					.dispatcher(crate::CO_CORE_NAME_STORAGE.with_name(&self.storage_core_name), identity.clone()),
				co_primitives::BlockLinks::default(),
				Some(crate::types::co_pinning_key::CoPinningKey::State.to_string(&self.co.id)),
				None,
				reducer_state.state().ok_or(anyhow::anyhow!("Expected state after create"))?,
				None,
			)
			.await?;
		}

		// result
		Ok(self.co.id)
	}
}
