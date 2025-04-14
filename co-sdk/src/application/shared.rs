use super::identity::create_identity_resolver;
use crate::{
	find_membership,
	library::{
		connections_peer_provider::ConnectionsPeerProvider, find_co_secret::find_co_secret_by_reference,
		membership_all_heads::membership_all_heads, push_heads::PushHeads,
	},
	reducer::{change::membership_writer::MembershipWriter, core_resolver::dynamic::DynamicCoreResolver},
	services::{
		connections::ConnectionMessage,
		network::{CoHeadsPublish, CoNetworkTaskSpawner},
		reducers::ReducerStorage,
	},
	types::co_reducer_context::CoReducerContext,
	CoCoreResolver, CoDate, CoReducer, CoReducerState, CoStorage, CoToken, CoTokenParameters, CoUuid, ReducerBuilder,
	Runtime, TaskSpawner, CO_CORE_NAME_CO, CO_CORE_NAME_KEYSTORE, CO_CORE_NAME_MEMBERSHIP,
};
use anyhow::anyhow;
use async_trait::async_trait;
use co_actor::ActorHandle;
use co_core_co::{CoAction, Participant};
use co_core_keystore::{Key, KeyStoreAction};
use co_core_membership::{Membership, MembershipsAction};
use co_identity::PrivateIdentity;
use co_log::Log;
use co_network::{bitswap::NetworkBlockStorage, PeerProvider};
use co_primitives::{tags, BlockStorageSettings, CloneWithBlockStorageSettings, CoId};
use co_storage::{Algorithm, BlockStorageContentMapping, EncryptedBlockStorage, Secret};
use serde::{Deserialize, Serialize};
use std::{
	collections::{BTreeMap, BTreeSet},
	fmt::Debug,
	sync::Arc,
	time::Duration,
};

/// Shared CO Builder.
/// The Shared CO state is sptrend in an membership of an other CO (typicalle the Local CO).
pub struct SharedCoBuilder {
	parent: CoReducer,
	keystore_core_name: String,
	membership_core_name: String,
	#[cfg(feature = "pinning")]
	storage_core_name: String,
	membership: Membership,
	network: Option<(CoNetworkTaskSpawner, ActorHandle<ConnectionMessage>)>,
	initialize: bool,
	network_block_timeout: Duration,
}
impl SharedCoBuilder {
	pub fn new(parent: CoReducer, membership: Membership) -> Self {
		Self {
			parent,
			membership,
			membership_core_name: CO_CORE_NAME_MEMBERSHIP.to_owned(),
			keystore_core_name: CO_CORE_NAME_KEYSTORE.to_owned(),
			#[cfg(feature = "pinning")]
			storage_core_name: crate::CO_CORE_NAME_STORAGE.to_owned(),
			network: None,
			initialize: true,
			network_block_timeout: Duration::from_secs(30),
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

	pub fn with_network(self, network: Option<(CoNetworkTaskSpawner, ActorHandle<ConnectionMessage>)>) -> Self {
		Self { network, ..self }
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

	pub fn build_network_storage<P>(
		&self,
		peer_provider: P,
		(network, _): (CoNetworkTaskSpawner, ActorHandle<ConnectionMessage>),
		secret: Option<&co_primitives::Secret>,
		storage: CoStorage,
	) -> anyhow::Result<CoStorage>
	where
		P: PeerProvider + Clone + Send + Sync + 'static,
	{
		let local_peer_id = network.local_peer_id();
		let mut network_storage = NetworkBlockStorage::new(storage, network, peer_provider, self.network_block_timeout);
		let token = if let Some(shared_secret) = secret {
			CoToken::new(shared_secret, CoTokenParameters(local_peer_id, self.membership.id.clone()))?
				.to_bitswap_token()?
		} else {
			CoToken::new_unsigned(CoTokenParameters(local_peer_id, self.membership.id.clone())).to_bitswap_token()?
		};
		network_storage.set_tokens(vec![token]);
		Ok(CoStorage::new(network_storage))
	}

	pub fn build_peer_provider<I>(
		&self,
		(_, connections): (CoNetworkTaskSpawner, ActorHandle<ConnectionMessage>),
		identity: I,
	) -> impl PeerProvider + Clone + Send + Sync + 'static
	where
		I: PrivateIdentity + Debug + Send + Sync + Clone + 'static,
	{
		ConnectionsPeerProvider::new(self.membership.id.clone(), identity.identity().to_owned(), connections)
	}

	pub async fn build<I>(
		self,
		tasks: TaskSpawner,
		storage: ReducerStorage,
		runtime: Runtime,
		identity: I,
		core_resolver: DynamicCoreResolver<CoStorage>,
		date: impl CoDate,
	) -> Result<CoReducer, anyhow::Error>
	where
		I: PrivateIdentity + Debug + Send + Sync + Clone + 'static,
	{
		let encrypted_storage = storage.encrypted_storage().cloned();

		// network
		//  we want to layer the network storage before the encryption to we only send encrypted blocks over network
		//  `BASE <- NETWORK <- ENCRYPTION``
		let network_storage = if let Some(network) = &self.network {
			// get base storage
			let base_storage = if let Some(encrypted_storage) = storage.encrypted_storage() {
				encrypted_storage.storage().clone()
			} else {
				storage.storage().clone()
			};

			// create network storage
			let secret = self.secret().await?;
			let peer_provider = self.build_peer_provider(network.clone(), identity.clone());
			let network_storage =
				self.build_network_storage(peer_provider, network.clone(), secret.as_ref(), base_storage)?;

			// create encrypted storage which uses the network storage as base
			// note: it uses the same mapping as the instance itrhout networking
			let next_storage = if let Some(encrypted_storage) = storage.encrypted_storage() {
				let mut encrypted_storage = encrypted_storage.clone();
				encrypted_storage.set_storage(network_storage);
				CoStorage::new(encrypted_storage)
			} else {
				network_storage
			};

			// result
			Some(next_storage)
		} else {
			None
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
		let log =
			Log::new(self.membership.id.as_str().as_bytes().to_vec(), create_identity_resolver(), Default::default());

		// reducer
		let mut reducer_builder = ReducerBuilder::new(core_resolver, log).with_initialize(self.initialize);
		let parent_storage = self.parent.storage();
		for co_state in self.membership.state.iter() {
			let reducer_state = CoReducerState::from_co_state(&parent_storage, co_state).await?;

			// get (unencrypted) state/heads
			let reducer_state = reducer_state.to_internal(&co_storage).await;

			// load state/heads mappings from parent into our mapping
			if let Some(parent_mappings) = reducer_state.to_external_mapping(&parent_storage).await {
				co_storage.insert_mappings(parent_mappings).await;
			}

			// add to builder
			//  when we only have one state we assume its the latest
			if let Some((state, heads)) = reducer_state.some() {
				reducer_builder = reducer_builder.with_snapshot(state, heads);
			}
			// if self.membership.state.len() == 1 {
			// 	reducer_builder = reducer_builder.with_latest_state(state, heads);
			// } else {
			// 	reducer_builder = reducer_builder.with_snapshot(state, heads);
			// }
		}
		let mut reducer = reducer_builder.build(&co_storage, runtime.runtime(), date).await?;

		// push changes to all connectable peers
		if let Some((network, connections)) = &self.network {
			let publish = PushHeads::new(
				network.clone(),
				connections.clone(),
				tasks.clone(),
				self.membership.id.clone(),
				PrivateIdentity::boxed(identity.clone()),
				true,
			)?;
			reducer.add_change_handler(Box::new(publish));
		}

		// publish changes for every `NetworkCoHeads` setting
		if let Some((network, _)) = self.network {
			let publish = CoHeadsPublish::new(network, self.membership.id.clone(), true);
			reducer.add_change_handler(Box::new(publish));
		}

		// setup auto write to parent co
		let writer = MembershipWriter::new(
			self.membership.id.clone(),
			self.parent.clone(),
			self.membership_core_name,
			identity.clone(),
			encrypted_storage.clone(),
			self.membership.state.clone(),
		);
		reducer.add_change_handler(Box::new(writer));

		// setup auto write references to parent co
		#[cfg(feature = "pinning")]
		{
			let writer = crate::reducer::change::reference_writer::ReferenceWriteReducerChangedHandler::new(
				crate::reducer::change::reference_writer::ReferenceWriter::new(
					self.parent.dispatcher(&self.storage_core_name, identity.clone()),
					context.clone(),
					Some(crate::types::co_pinning_key::CoPinningKey::State.to_string(&self.membership.id)),
				),
				*reducer.state(),
			);
			reducer.add_change_handler(Box::new(writer));
		}

		// result
		let application_identifier = self
			.parent
			.handle()
			.tags()
			.string("application")
			.ok_or_else(|| anyhow!("Missing parent tag: application"))?
			.to_owned();
		Ok(CoReducer::spawn(
			application_identifier,
			self.membership.id,
			Some(self.parent.id().clone()),
			context.storage(false),
			tasks,
			runtime,
			reducer,
			context,
		)?)
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCo {
	pub id: CoId,
	pub name: String,
	pub algorithm: Option<Algorithm>,
}
impl CreateCo {
	pub fn generate(name: String) -> Self {
		CreateCo { id: uuid::Uuid::new_v4().to_string().into(), name, algorithm: Some(Default::default()) }
	}

	pub fn with_public(self) -> Self {
		CreateCo { algorithm: None, ..self }
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
	network_storage: Option<CoStorage>,
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
			if let Some(network_storage) = &self.network_storage {
				return network_storage.clone();
			}
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
			membership_core_name: CO_CORE_NAME_MEMBERSHIP.to_owned(),
			keystore_core_name: CO_CORE_NAME_KEYSTORE.to_owned(),
			#[cfg(feature = "pinning")]
			storage_core_name: crate::CO_CORE_NAME_STORAGE.to_owned(),
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
					let result_storage =
						EncryptedBlockStorage::new(storage.clone(), key.clone(), algorithm, Default::default());
					(CoStorage::new(result_storage.clone()), Some((result_storage, key_uri, key)))
				},
				None => (storage.clone_with_settings(BlockStorageSettings::new().with_detached()), None),
			};

		// log
		let log = Log::new(self.co.id.as_str().as_bytes().to_vec(), create_identity_resolver(), Default::default());

		// reducer
		let mut reducer = ReducerBuilder::new(CoCoreResolver::default(), log)
			.build(&co_storage, runtime.runtime(), date)
			.await?;

		// initialize
		let mut participants = BTreeMap::new();
		participants.insert(
			identity.identity().to_owned(),
			Participant {
				did: identity.identity().to_owned(),
				state: co_core_co::ParticipantState::Active,
				tags: tags!(),
			},
		);
		reducer
			.push(
				&co_storage,
				runtime.runtime(),
				&identity,
				CO_CORE_NAME_CO,
				&CoAction::Create {
					id: self.co.id.to_owned(),
					name: self.co.name.to_owned(),
					cores: Default::default(),
					participants,
					key: encrypted_storage.as_ref().map(|(_, key_uri, _)| key_uri.clone()),
				},
			)
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
			let reducer_context = Arc::new(SharedContext {
				id: self.co.id.clone(),
				encrypted_storage: _encrypted_storage.clone(),
				storage: storage.clone(),
				network_storage: None,
			});
			let writer = crate::reducer::change::reference_writer::ReferenceWriter::new(
				self.parent.dispatcher(&self.storage_core_name, identity.clone()),
				reducer_context,
				Some(crate::types::co_pinning_key::CoPinningKey::State.to_string(&self.co.id)),
			);
			writer
				.write(
					None,
					reducer_state.state().ok_or(anyhow::anyhow!("Expected state after create"))?,
					<<CoStorage as co_primitives::BlockStorage>::StoreParams as co_primitives::StoreParams>::MAX_BLOCK_SIZE,
				)
				.await?;
		}

		// result
		Ok(self.co.id)
	}
}
