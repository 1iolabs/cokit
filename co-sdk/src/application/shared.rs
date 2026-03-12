// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use super::identity::create_identity_resolver;
#[cfg(feature = "network")]
use crate::services::application::KeyRequestAction;
use crate::{
	find_membership,
	library::{
		builtin_cores::builtin_cores, core_source::CoreSource, find_co_secret::find_co_secret_by_reference,
		is_membership_heads_encrypted::is_membership_heads_encrypted, membership_all_heads::membership_all_heads,
		wait_response::request_response_timeout,
	},
	reducer::{
		change::membership_writer::MembershipWriter,
		core_resolver::{dynamic::DynamicCoreResolver, log::LogCoreResolver},
		state_resolver::MembershipStateResolver,
	},
	services::{
		reducer::{FlushInfo, ReducerBlockStorage, ReducerFlush},
		reducers::ReducerStorage,
	},
	types::co_reducer_context::{CoReducerContext, CoReducerFeature},
	Action, ApplicationMessage, CoCoreResolver, CoReducer, CoReducerState, CoStorage, CoUuid, Cores, Reducer,
	ReducerBuilder, Runtime, TaskSpawner, CO_CORE_CO, CO_CORE_NAME_CO, CO_CORE_NAME_KEYSTORE, CO_CORE_NAME_MEMBERSHIP,
};
use anyhow::anyhow;
use async_trait::async_trait;
use cid::Cid;
use co_actor::ActorHandle;
use co_core_co::{CoAction, CreateAction};
use co_core_keystore::{Key, KeyStoreAction};
use co_core_membership::{Membership, MembershipsAction};
use co_identity::PrivateIdentity;
use co_log::{IdentityEntryVerifier, Log};
use co_primitives::{
	tags, BlockLinks, BlockStorageCloneSettings, CloneWithBlockStorageSettings, CoDate, CoId, DynamicCoDate,
	OptionMappedCid, Tags,
};
use co_storage::{Algorithm, BlockStorageContentMapping, EncryptedBlockStorage, EncryptionReferenceMode, Secret};
use serde::{Deserialize, Serialize};
use std::{
	collections::{BTreeMap, BTreeSet},
	fmt::Debug,
	sync::Arc,
	time::Duration,
};

/// Shared CO Builder.
/// The Shared CO state is stored in a membership of an other CO (typically the Local CO).
pub struct SharedCoBuilder {
	parent: CoReducer,
	keystore_core_name: String,
	membership_core_name: String,
	membership: Membership,
	initialize: bool,
	key_request_timeout: Duration,
	verify_links: Option<BlockLinks>,
}
impl SharedCoBuilder {
	pub fn new(parent: CoReducer, membership: Membership) -> Self {
		Self {
			parent,
			membership,
			membership_core_name: CO_CORE_NAME_MEMBERSHIP.to_string(),
			keystore_core_name: CO_CORE_NAME_KEYSTORE.to_string(),
			initialize: true,
			key_request_timeout: Duration::from_secs(30),
			verify_links: None,
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

	pub fn with_key_request_timeout(self, key_request_timeout: Duration) -> Self {
		Self { key_request_timeout, ..self }
	}

	pub fn with_verify_links(self, verify_links: Option<BlockLinks>) -> Self {
		Self { verify_links, ..self }
	}

	/// Read (latest) secret from parent CO or network.
	///
	/// If secret if not avilalbe it will be fetched using `handle`.
	pub async fn secret(
		&self,
		_handle: Option<ActorHandle<ApplicationMessage>>,
	) -> anyhow::Result<Option<co_primitives::Secret>> {
		if let Some(key_reference) = &self.membership.key {
			Ok(Some(find_co_secret_by_reference(&self.parent, key_reference, Some(&self.keystore_core_name)).await?))
		} else if is_membership_heads_encrypted(&self.parent.storage(), &self.membership).await? {
			#[cfg(feature = "network")]
			if let Some(handle) = _handle {
				return Ok(Some(self.request_secret(handle).await?));
			}
			Err(anyhow!("Key not available"))
		} else {
			Ok(None)
		}
	}

	/// Request secret from network using handle.
	#[cfg(feature = "network")]
	pub async fn request_secret(
		&self,
		handle: ActorHandle<ApplicationMessage>,
	) -> Result<co_primitives::Secret, anyhow::Error> {
		let request = KeyRequestAction {
			co: self.membership.id.clone(),
			parent_co: self.parent.id().to_owned(),
			key: None,
			from: Some(self.membership.did.clone()),
			network: None,
		};
		let key = request_response_timeout(
			handle,
			self.key_request_timeout,
			Action::KeyRequest(request.clone()),
			move |action| match action {
				Action::KeyRequestComplete(action_request, action_result) if action_request == &request => {
					Some(action_result.clone())
				},
				_ => None,
			},
		)
		.await??;
		find_co_secret_by_reference(&self.parent, &key, Some(&self.keystore_core_name)).await
	}

	#[allow(clippy::too_many_arguments)]
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
		#[cfg(feature = "pinning")] pin_strategy: co_core_storage::PinStrategy,
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
			// note: it uses the same mapping as the instance without networking
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

		// membership states
		let parent_storage = self.parent.storage();
		let mut membership_states = MembershipStateResolver::default();
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
			membership_states.insert(reducer_state);
		}

		// reducer builder
		let reducer_builder = ReducerBuilder::new(core_resolver, log)
			.with_initialize(false)
			.with_state_resolver(membership_states);

		// use states from pinning
		#[cfg(feature = "pinning")]
		let reducer_builder = {
			reducer_builder.with_state_resolver(crate::reducer::state_resolver::StorageStateResolver::new(
				self.parent.clone(),
				pinning.identity.clone(),
				pin_strategy,
				self.membership.id.clone(),
			))
		};

		// reducer
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
		CoReducer::spawn(
			application_handle,
			application_identifier,
			self.membership.id,
			Some(self.parent.id().clone()),
			tasks,
			runtime,
			reducer,
			context,
			Box::new(flush),
			self.initialize,
			self.verify_links,
		)
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
			let id = &self.membership_writer.id;

			// compute
			let parent_pinning_state = crate::library::storage_pinning::storage_pinning(
				&self.pinning,
				None,
				&parent.storage(),
				parent.reducer_state().await,
				id,
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
	id: CoId,
	name: String,
	algorithm: Option<Algorithm>,
	cores: BTreeMap<String, (CoreSource, Tags)>,
	guards: BTreeMap<String, (CoreSource, Tags)>,
}
impl CreateCo {
	pub fn new(id: impl Into<CoId>, name: Option<String>) -> Self {
		let id = id.into();
		let name = name.unwrap_or_else(|| id.to_string());
		CreateCo {
			id,
			name,
			algorithm: Some(Default::default()),
			cores: Default::default(),
			guards: Default::default(),
		}
		.with_co_guard()
	}

	pub fn generate(name: String) -> Self {
		Self::new(uuid::Uuid::new_v4().to_string(), Some(name))
	}

	pub fn id(&self) -> &CoId {
		&self.id
	}

	pub fn with_id(mut self, id: impl Into<CoId>) -> Self {
		self.id = id.into();
		self
	}

	pub fn name(&self) -> &str {
		&self.name
	}

	pub fn with_name(mut self, name: String) -> Self {
		self.name = name;
		self
	}

	pub fn with_core(mut self, core_name: &str, core_type: &str, core_binary: Cid) -> Self {
		self.cores
			.insert(core_name.to_owned(), (CoreSource::Reference(core_binary), tags!("type": core_type)));
		self
	}

	pub fn with_core_bytes(mut self, core_name: &str, core_type: &str, core_binary: impl Into<Vec<u8>>) -> Self {
		self.cores
			.insert(core_name.to_owned(), (CoreSource::Bytes(core_binary.into()), tags!("type": core_type)));
		self
	}

	pub fn without_co_guard(mut self) -> Self {
		self.guards.remove(CO_CORE_NAME_CO.as_ref());
		self
	}

	pub fn with_co_guard(mut self) -> Self {
		self.guards.insert(
			CO_CORE_NAME_CO.to_string(),
			(CoreSource::Builtin(CO_CORE_CO.to_owned()), tags!("type": CO_CORE_CO)),
		);
		self
	}

	pub fn with_guard(mut self, guard_name: &str, guard_type: &str, guard_binary: Cid) -> Self {
		self.guards
			.insert(guard_name.to_owned(), (CoreSource::Reference(guard_binary), tags!("type": guard_type)));
		self
	}

	pub fn with_guard_bytes(mut self, guard_name: &str, guard_type: &str, guard_binary: impl Into<Vec<u8>>) -> Self {
		self.guards
			.insert(guard_name.to_owned(), (CoreSource::Bytes(guard_binary.into()), tags!("type": guard_type)));
		self
	}

	pub fn with_algorithm(mut self, algorithm: Option<Algorithm>) -> Self {
		self.algorithm = algorithm;
		self
	}

	pub fn public(&self) -> bool {
		self.algorithm.is_none()
	}

	pub fn with_public(self, public: bool) -> Self {
		self.with_algorithm(if public { None } else { Some(Algorithm::default()) })
	}
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
						storage.load_mapping(cid).await?;
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
	co: CreateCo,
}
impl SharedCoCreator {
	pub fn new(parent: CoReducer, co: CreateCo) -> Self {
		Self {
			parent,
			co,
			membership_core_name: CO_CORE_NAME_MEMBERSHIP.to_string(),
			keystore_core_name: CO_CORE_NAME_KEYSTORE.to_string(),
		}
	}

	pub fn with_membership_core_name(self, membership_core_name: String) -> Self {
		Self { membership_core_name, ..self }
	}

	pub fn with_keystore_core_name(self, keystore_core_name: String) -> Self {
		Self { keystore_core_name, ..self }
	}

	// TODO: Cleanup when something fails?
	#[allow(clippy::too_many_arguments)]
	pub async fn create<I>(
		self,
		storage: CoStorage,
		runtime: Runtime,
		cores: &Cores,
		identity: I,
		date: impl CoDate,
		uuid: impl CoUuid,
		#[cfg(feature = "pinning")] pinning: crate::library::storage_pinning::StoragePinningContext,
		#[cfg(feature = "pinning")] pin_strategy: co_core_storage::PinStrategy,
	) -> Result<CoId, anyhow::Error>
	where
		I: PrivateIdentity + Clone + Debug + Send + Sync + 'static,
	{
		let date = date.boxed();

		// storage
		let (co_storage, encrypted_storage): (CoStorage, Option<(EncryptedBlockStorage<CoStorage>, String, Secret)>) =
			match self.co.algorithm {
				Some(algorithm) => {
					let key_uri = format!("urn:co:{}:{}", self.co.id, uuid.uuid());
					let key = algorithm.generate_serect();
					let result_storage =
						EncryptedBlockStorage::new(storage.clone(), key.clone(), algorithm, Default::default())
							.with_encryption_reference_mode(EncryptionReferenceMode::DisallowExcept(builtin_cores()));
					(CoStorage::new(result_storage.clone()), Some((result_storage, key_uri, key)))
				},
				None => (storage.clone_with_settings(BlockStorageCloneSettings::new().with_detached()), None),
			};

		// log
		let log = Log::new(
			self.co.id.as_str().as_bytes().to_vec(),
			IdentityEntryVerifier::new(create_identity_resolver()),
			Default::default(),
		);

		// reducer
		let core_resolver = CoCoreResolver::default();
		let core_resolver = LogCoreResolver::new(core_resolver, self.co.id.clone(), date.clone());
		let reducer_builder = ReducerBuilder::new(core_resolver, log);
		#[cfg(feature = "pinning")]
		let reducer_builder = {
			reducer_builder.with_state_resolver(crate::reducer::state_resolver::StorageStateResolver::new(
				self.parent.clone(),
				pinning.identity.clone(),
				pin_strategy,
				self.co.id.clone(),
			))
		};
		let mut reducer = reducer_builder.build(&co_storage, runtime.runtime(), date).await?;

		// initialize
		let mut create = CreateAction::new(
			self.co.id.to_owned(),
			self.co.name.to_owned(),
			CoreSource::built_in(CO_CORE_CO).binary(&storage, cores).await?,
		)
		.with_participant(identity.identity().to_owned(), tags!())
		.with_key(encrypted_storage.as_ref().map(|(_, key_uri, _)| key_uri.clone()));
		for (core_name, (core_source, tags)) in self.co.cores {
			create = create.with_core(core_name, core_source.to_core(&co_storage, cores, tags).await?);
		}
		for (guard_name, (guard_source, tags)) in self.co.guards {
			create = create.with_guard(guard_name, guard_source.to_guard(&co_storage, cores, tags).await?);
		}
		reducer
			.push(&co_storage, runtime.runtime(), &identity, CO_CORE_NAME_CO, &CoAction::Create(create))
			.await?;
		let reducer_state = CoReducerState::new(*reducer.state(), reducer.heads().clone());

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

		// result
		Ok(self.co.id)
	}
}
