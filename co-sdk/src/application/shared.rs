use super::{
	co_context::{CoPinningKey, ReducerStorage},
	identity::create_identity_resolver,
};
use crate::{
	find_membership,
	library::{
		connections_peer_provider::ConnectionsPeerProvider,
		push_heads::PushHeads,
		to_external_cid::{to_external_cid, to_external_cids},
	},
	reducer::{
		change::{
			membership_writer::MembershipWriter,
			reference_writer::{ReferenceWriteReducerChangedHandler, ReferenceWriter},
		},
		core_resolver::dynamic::DynamicCoreResolver,
	},
	services::{
		connections::ConnectionMessage,
		network::{CoHeadsPublish, CoNetworkTaskSpawner},
	},
	state::{find, query_core, QueryExt},
	types::{co_reducer::CoReducerContext, co_storage::CoBlockStorageContentMapping},
	CoCoreResolver, CoReducer, CoStorage, CoToken, CoTokenParameters, ReducerBuilder, Runtime, TaskSpawner,
	CO_CORE_NAME_CO, CO_CORE_NAME_KEYSTORE, CO_CORE_NAME_MEMBERSHIP, CO_CORE_NAME_STORAGE,
};
use anyhow::anyhow;
use async_trait::async_trait;
use cid::Cid;
use co_actor::ActorHandle;
use co_core_co::{CoAction, Participant};
use co_core_keystore::{Key, KeyStore, KeyStoreAction};
use co_core_membership::{CoState, Membership, MembershipsAction};
use co_core_storage::{PinStrategy, StorageAction};
use co_identity::PrivateIdentity;
use co_log::Log;
use co_network::{bitswap::NetworkBlockStorage, PeerProvider};
use co_primitives::{tags, CoId, KnownMultiCodec, MultiCodec, StoreParams, WeakCid};
use co_storage::{Algorithm, BlockStorage, BlockStorageContentMapping, EncryptedBlockStorage, Secret, StorageError};
use futures::{stream, StreamExt, TryStreamExt};
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
			storage_core_name: CO_CORE_NAME_STORAGE.to_owned(),
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

	pub fn with_storage_core_name(self, storage_core_name: String) -> Self {
		Self { storage_core_name, ..self }
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
			let (storage, key_store) = query_core::<KeyStore>(&self.keystore_core_name)
				.execute_reducer(&self.parent)
				.await?;
			let (_, key) = find(&storage, &key_store.keys, |(k, _)| k == key_reference)
				.await?
				.ok_or(anyhow::anyhow!("Shared key not found: {}", key_reference))?;
			let secret = match key.secret {
				co_core_keystore::Secret::SharedKey(sec) => Ok(sec),
				_ => Err(anyhow!("Invalid secret")),
			}?;
			Ok(Some(secret))
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
			create_identity_resolver(),
			context.storage(false),
			// latest_state.map(|(_, heads)| heads).unwrap_or_default(),
			Default::default(),
		);

		// reducer
		let mut reducer_builder = ReducerBuilder::new(core_resolver, log).with_initialize(self.initialize);
		for co_state in self.membership.state.iter() {
			// get (unencrypted) state/heads
			let state = context.to_internal_cid(co_state.state.into()).await?;
			let heads: BTreeSet<Cid> = stream::iter(co_state.heads.iter())
				.then(|cid| async { context.to_internal_cid(cid.cid()).await })
				.try_collect()
				.await?;

			// add to builder
			//  when we only have one state we assume its the latest
			reducer_builder = reducer_builder.with_snapshot(state, heads);
			// if self.membership.state.len() == 1 {
			// 	reducer_builder = reducer_builder.with_latest_state(state, heads);
			// } else {
			// 	reducer_builder = reducer_builder.with_snapshot(state, heads);
			// }
		}
		let mut reducer = reducer_builder.build(runtime.runtime()).await?;

		// push changes to all connectable peers
		if let Some((network, connections)) = &self.network {
			let mapping = encrypted_storage.as_ref().map(|e| e.content_mapping());
			let publish = PushHeads::new(
				network.clone(),
				connections.clone(),
				tasks,
				self.membership.id.clone(),
				PrivateIdentity::boxed(identity.clone()),
				mapping.clone(),
				true,
			)?;
			reducer.add_change_handler(Box::new(publish));
		}

		// publish changes for every `NetworkCoHeads` setting
		if let Some((network, _)) = self.network {
			let mapping = encrypted_storage.as_ref().map(|e| e.content_mapping());
			let publish = CoHeadsPublish::new(network, self.membership.id.clone(), mapping.clone(), true);
			reducer.add_change_handler(Box::new(publish));
		}

		// setup auto write to parent co
		let writer = MembershipWriter::new(
			self.membership.id.clone(),
			self.parent.clone(),
			self.membership_core_name,
			identity.clone(),
			encrypted_storage.clone(),
			self.membership
				.state
				.iter()
				.flat_map(|item| item.heads.iter().map(WeakCid::cid))
				.collect(),
		);
		reducer.add_change_handler(Box::new(writer));

		// setup auto write references to parent co
		let writer = ReferenceWriteReducerChangedHandler::new(
			ReferenceWriter::new(
				self.parent.dispatcher(&self.storage_core_name, identity.clone()),
				context.clone(),
				Some(CoPinningKey::State.to_string(&self.membership.id)),
			),
			*reducer.state(),
		);
		reducer.add_change_handler(Box::new(writer));

		// result
		Ok(CoReducer::new(self.membership.id, Some(self.parent.id().clone()), runtime, reducer, context))
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
			let co_heads = co.heads().await;
			if co_heads
				!= membership
					.state
					.iter()
					.flat_map(|item| item.heads.iter().map(WeakCid::cid))
					.collect::<BTreeSet<_>>()
			{
				tracing::info!(co = ?co.id(), from = ?co_heads, to = ?membership, "membership-update");
				for state in membership.state.iter() {
					// encryption mapping
					if let (Some(storage), Some(cid)) = (&self.encrypted_storage, &state.encryption_mapping) {
						storage.load_mapping(&cid).await?;
					}

					// snapshot
					let state_heads: BTreeSet<Cid> = state.heads.iter().map(WeakCid::cid).collect();
					co.insert_snapshot(state.state.into(), state_heads.clone()).await?;

					// load snapshot
					co.join(&state_heads).await?;
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

	fn content_mapping(&self) -> Option<CoBlockStorageContentMapping> {
		self.encrypted_storage
			.as_ref()
			.map(|storage| storage.content_mapping())
			.map(CoBlockStorageContentMapping::new)
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

	/// Map external [`Cid`] to internal [`Cid`].
	/// Internal means in context of the CO.
	/// If no mapping is needed/available return the original [`Cid`].
	async fn to_internal_cid(&self, cid: Cid) -> Result<Cid, StorageError> {
		match (&self.encrypted_storage, MultiCodec::from(&cid)) {
			(Some(storage), MultiCodec::Known(KnownMultiCodec::CoEncryptedBlock)) => {
				// make sure the block is available (network)
				if let Some(network_storage) = &self.network_storage {
					network_storage.get(&cid).await?;
				}

				// get unencrypted
				Ok(*storage.get_unencrypted(&cid).await?.cid())
			},
			_ => Ok(cid),
		}
	}

	/// Map internal [`Cid`] to external [`Cid`].
	/// External means NOT in context of the CO.
	/// If no mapping is needed/available return the original [`Cid`].
	async fn to_external_cid(&self, cid: Cid) -> Result<Cid, StorageError> {
		if let Some(encrypted_storage) = &self.encrypted_storage {
			Ok(encrypted_storage.content_mapping().to_plain(&cid).await.unwrap_or(cid))
		} else {
			Ok(cid)
		}
	}
}

pub struct SharedCoCreator {
	parent: CoReducer,
	keystore_core_name: String,
	membership_core_name: String,
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
			storage_core_name: CO_CORE_NAME_STORAGE.to_owned(),
		}
	}

	pub fn with_membership_core_name(self, membership_core_name: String) -> Self {
		Self { membership_core_name, ..self }
	}

	pub fn with_keystore_core_name(self, keystore_core_name: String) -> Self {
		Self { keystore_core_name, ..self }
	}

	pub fn with_storage_core_name(self, storage_core_name: String) -> Self {
		Self { storage_core_name, ..self }
	}

	/// TODO: Cleanup when something fails?
	pub async fn create<I>(self, storage: CoStorage, runtime: Runtime, identity: I) -> Result<CoId, anyhow::Error>
	where
		I: PrivateIdentity + Clone + Debug + Send + Sync + 'static,
	{
		// storage
		let (co_storage, encrypted_storage): (CoStorage, Option<(EncryptedBlockStorage<CoStorage>, String, Secret)>) =
			match self.co.algorithm {
				Some(algorithm) => {
					let key_uri = format!("urn:co:{}:{}", self.co.id, uuid::Uuid::new_v4());
					let key = algorithm.generate_serect();
					let result_storage =
						EncryptedBlockStorage::new(storage.clone(), key.clone(), algorithm, Default::default());
					(CoStorage::new(result_storage.clone()), Some((result_storage, key_uri, key)))
				},
				None => (storage.clone(), None),
			};

		// log
		let log = Log::new(
			self.co.id.as_str().as_bytes().to_vec(),
			create_identity_resolver(),
			co_storage,
			Default::default(),
		);

		// reducer
		let mut reducer = ReducerBuilder::new(CoCoreResolver::default(), log)
			.build(runtime.runtime())
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
		let mut state = reducer.state().ok_or(anyhow::anyhow!("Expected state after create"))?;
		let mut heads = reducer.heads().clone();

		// store key in parent co
		let (key_uri, encryption_mapping, encrypted_storage) =
			if let Some((encrypted_storage, key_uri, secret)) = encrypted_storage {
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
				(Some(key_uri), None, Some(encrypted_storage))
			} else {
				(None, None, None)
			};

		// store encrypted state/heads
		if let Some(encrypted_storage) = &encrypted_storage {
			let mapping = encrypted_storage.content_mapping();
			state = to_external_cid(&mapping, state).await;
			heads = to_external_cids(&mapping, heads).await;
		}

		// add membership to parent co
		let membership: Membership = Membership {
			id: self.co.id.to_owned(),
			did: identity.identity().to_owned(),
			state: BTreeSet::from([CoState {
				heads: heads.iter().map(Into::into).collect(),
				state: state.into(),
				encryption_mapping,
			}]),
			key: key_uri,
			membership_state: co_core_membership::MembershipState::Active,
			tags: tags!(),
		};
		self.parent
			.push(&identity, &self.membership_core_name, &MembershipsAction::Join(membership))
			.await?;

		// add pin to parent co
		let pin_state = StorageAction::PinCreate(
			CoPinningKey::State.to_string(&self.co.id),
			PinStrategy::Unlimited,
			Default::default(),
		);
		let pin_log = StorageAction::PinCreate(
			CoPinningKey::Log.to_string(&self.co.id),
			PinStrategy::Unlimited,
			Default::default(),
		);
		self.parent.push(&identity, &self.storage_core_name, &pin_log).await?;
		self.parent.push(&identity, &self.storage_core_name, &pin_state).await?;

		// pin initial state
		let reducer_context = Arc::new(SharedContext {
			id: self.co.id.clone(),
			encrypted_storage: encrypted_storage.clone(),
			storage: storage.clone(),
			network_storage: None,
		});
		let writer = ReferenceWriter::new(
			self.parent.dispatcher(&self.storage_core_name, identity.clone()),
			reducer_context,
			Some(CoPinningKey::State.to_string(&self.co.id)),
		);
		writer
			.write(None, state, <CoStorage as BlockStorage>::StoreParams::MAX_BLOCK_SIZE)
			.await?;

		// result
		Ok(self.co.id)
	}
}
