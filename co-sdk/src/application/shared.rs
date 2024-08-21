use super::identity::create_identity_resolver;
use crate::{
	drivers::network::{publish::CoHeadsPublish, CoNetworkTaskSpawner},
	find_membership,
	library::{
		co_peer_provider::CoPeerProvider,
		co_state::CoState,
		override_peer_provider::{OverridePeerProvider, Overrides},
		push_heads::PushHeads,
	},
	reducer::core_resolver::dynamic::DynamicCoreResolver,
	state::find,
	types::{co_reducer::CoReducerContext, co_storage::CoBlockStorageContentMapping},
	CoCoreResolver, CoReducer, CoStorage, CoToken, CoTokenParameters, Reducer, ReducerBuilder, ReducerChangeContext,
	ReducerChangedHandler, Runtime, TaskSpawner, CO_CORE_NAME_CO, CO_CORE_NAME_KEYSTORE, CO_CORE_NAME_MEMBERSHIP,
};
use anyhow::anyhow;
use async_trait::async_trait;
use co_core_co::{CoAction, Participant};
use co_core_keystore::{Key, KeyStoreAction};
use co_core_membership::{Membership, MembershipsAction};
use co_identity::PrivateIdentity;
use co_log::Log;
use co_network::{bitswap::NetworkBlockStorage, PeerProvider};
use co_primitives::{tags, CoId, KnownMultiCodec, MultiCodec};
use co_storage::{Algorithm, BlockStorageContentMapping, EncryptedBlockStorage, Secret, StorageError};
use futures::{stream, StreamExt, TryStreamExt};
use libipld::Cid;
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
	membership: Membership,
	network: Option<CoNetworkTaskSpawner>,
	initialize: bool,
	network_block_timeout: Duration,
	network_overrides: Option<Overrides>,
}
impl SharedCoBuilder {
	pub fn new(parent: CoReducer, membership: Membership) -> Self {
		Self {
			parent,
			membership,
			membership_core_name: CO_CORE_NAME_MEMBERSHIP.to_owned(),
			keystore_core_name: CO_CORE_NAME_KEYSTORE.to_owned(),
			network: None,
			initialize: true,
			network_block_timeout: Duration::from_secs(30),
			network_overrides: Default::default(),
		}
	}

	pub fn with_membership_core_name(self, membership_core_name: String) -> Self {
		Self { membership_core_name, ..self }
	}

	pub fn with_keystore_core_name(self, keystore_core_name: String) -> Self {
		Self { keystore_core_name, ..self }
	}

	pub fn with_network(self, network: Option<CoNetworkTaskSpawner>) -> Self {
		Self { network, ..self }
	}

	pub fn with_initialize(self, initialize: bool) -> Self {
		Self { initialize, ..self }
	}

	pub fn with_network_overrides(self, network_overrides: Option<Overrides>) -> Self {
		Self { network_overrides, ..self }
	}

	/// Read (latest) secret from parent CO.
	pub async fn secret(&self) -> anyhow::Result<Option<co_primitives::Secret>> {
		if let Some(key_reference) = &self.membership.key {
			let key_store: co_core_keystore::KeyStore = self.parent.state(&self.keystore_core_name).await?;
			let (_, key) = find(&self.parent.storage(), &key_store.keys, |(k, _)| k == key_reference)
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
		network: CoNetworkTaskSpawner,
		secret: Option<&co_primitives::Secret>,
		storage: CoStorage,
	) -> anyhow::Result<CoStorage>
	where
		P: PeerProvider + Send + Sync + 'static,
	{
		let local_peer_id = network.local_peer_id();
		let mut network_storage = NetworkBlockStorage::new(storage, network, peer_provider, self.network_block_timeout);
		if let Some(shared_secret) = secret {
			let token = CoToken::new(shared_secret, CoTokenParameters(local_peer_id, self.membership.id.clone()))?
				.to_bitswap_token()?;
			network_storage.set_tokens(vec![token]);
		}
		Ok(CoStorage::new(network_storage))
	}

	pub async fn build_encrypted_storage(
		&self,
		secret: &co_primitives::Secret,
		storage: CoStorage,
	) -> anyhow::Result<EncryptedBlockStorage<CoStorage>> {
		let result_storage =
			EncryptedBlockStorage::new(storage, Secret::new(secret.divulge().to_vec()), Default::default());
		if let Some(mapping) = &self.membership.encryption_mapping {
			result_storage.load_mapping(mapping).await?;
		}
		Ok(result_storage)
	}

	pub fn build_peer_provider<I>(
		&self,
		network: CoNetworkTaskSpawner,
		identity: I,
		co_state: CoState,
	) -> impl PeerProvider + Send + Sync + 'static
	where
		I: PrivateIdentity + Debug + Send + Sync + Clone + 'static,
	{
		OverridePeerProvider::new(
			self.network_overrides.clone().unwrap_or_default(),
			CoPeerProvider::new(network, create_identity_resolver(), identity, self.membership.id.clone(), co_state),
			self.membership.id.clone(),
		)
	}

	pub async fn build_storage(&self, storage: CoStorage) -> Result<CoStorage, anyhow::Error> {
		match self.secret().await? {
			Some(secret) => Ok(CoStorage::new(self.build_encrypted_storage(&secret, storage).await?)),
			None => Ok(storage),
		}
	}

	pub async fn build<I>(
		self,
		tasks: TaskSpawner,
		storage: CoStorage,
		runtime: Runtime,
		identity: I,
		core_resolver: DynamicCoreResolver<CoStorage>,
	) -> Result<CoReducer, anyhow::Error>
	where
		I: PrivateIdentity + Debug + Send + Sync + Clone + 'static,
	{
		// secret
		let secret = self.secret().await?;

		// network
		let (storage, co_state) = if let Some(network) = &self.network {
			let co_state = CoState::default();
			let peer_provider = self.build_peer_provider(network.clone(), identity.clone(), co_state.clone());
			(
				self.build_network_storage(peer_provider, network.clone(), secret.as_ref(), storage.clone())?,
				Some(co_state),
			)
		} else {
			(storage, None)
		};

		// encryption
		let (storage, encrypted_storage) = match &secret {
			// encrypted
			Some(secret) => {
				let encrypted_storage = self.build_encrypted_storage(secret, storage).await?;
				(CoStorage::new(encrypted_storage.clone()), Some(encrypted_storage))
			},
			// plain
			None => (storage, None),
		};

		// context
		let context = SharedContext { encrypted_storage: encrypted_storage.clone(), id: self.membership.id.clone() };

		// get (unencrypted) state/heads
		let state = context.to_internal_cid(self.membership.state).await?;
		let heads: BTreeSet<Cid> = stream::iter(self.membership.heads.iter())
			.then(|cid| async { context.to_internal_cid(*cid).await })
			.try_collect()
			.await?;

		// explicitly update co state so the network has it available when initialize
		if let Some(co_state) = &co_state {
			co_state.write(&storage, state.into(), true).await;
		}

		// log
		let log = Log::new(
			self.membership.id.as_str().as_bytes().to_vec(),
			create_identity_resolver(),
			storage.clone(),
			heads.clone(),
		);

		// reducer
		let mut reducer = ReducerBuilder::new(core_resolver, log)
			.with_initialize(self.initialize)
			.with_latest_state(state, heads.clone())
			.build(runtime.runtime())
			.await?;

		// push changes to all connectable peers
		let co_state = if let Some(network) = &self.network {
			let co_state = co_state.unwrap_or_default();
			let mapping = encrypted_storage.as_ref().map(|e| e.content_mapping());
			let peer_provider = self.build_peer_provider(network.clone(), identity.clone(), co_state.clone());
			let publish = PushHeads::new(
				network.clone(),
				tasks,
				self.membership.id.clone(),
				identity.clone(),
				peer_provider,
				mapping.clone(),
				true,
			);
			reducer.add_change_handler(Box::new(publish));
			Some(co_state)
		} else {
			None
		};

		// publish changes for every `NetworkCoHeads` setting
		if let Some(network) = self.network {
			let mapping = encrypted_storage.as_ref().map(|e| e.content_mapping());
			let publish = CoHeadsPublish::new(network, self.membership.id.clone(), mapping.clone(), true);
			reducer.add_change_handler(Box::new(publish));
		}

		// update co state token
		if let Some(co_state) = co_state {
			reducer.add_change_handler(Box::new(co_state));
		}

		// setup auto write to parent co
		let writer = MembershipWriter {
			id: self.membership.id.clone(),
			parent: self.parent.clone(),
			membership_core_name: self.membership_core_name,
			identity: identity.clone(),
			encrypted_storage: encrypted_storage.clone(),
		};
		reducer.add_change_handler(Box::new(writer));

		// result
		Ok(CoReducer::new(self.membership.id, Some(self.parent.id().clone()), runtime, reducer, Arc::new(context)))
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

struct SharedContext {
	id: CoId,
	encrypted_storage: Option<EncryptedBlockStorage<CoStorage>>,
}
impl SharedContext {
	/// Update `co` membership if necessary.
	async fn update_membership(&self, parent: CoReducer, co: CoReducer) -> Result<(), anyhow::Error> {
		if let Some(membership) = find_membership(&parent, co.id()).await? {
			let co_heads = co.heads().await;
			if co_heads != membership.heads {
				tracing::info!(co = ?co.id(), from = ?co_heads, to = ?membership, "membership-update");

				// encryption mapping
				if let (Some(storage), Some(cid)) = (&self.encrypted_storage, &membership.encryption_mapping) {
					storage.load_mapping(&cid).await?;
				}

				// snapshot
				co.insert_snapshot(membership.state, membership.heads.clone()).await?;

				// load snapshot
				co.join(&membership.heads).await?;
			}
		}
		Ok(())
	}
}
#[async_trait]
impl CoReducerContext for SharedContext {
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
				Ok(*storage.get_unencrypted(&cid, false).await?.cid())
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
	co: CreateCo,
}
impl SharedCoCreator {
	pub fn new(parent: CoReducer, co: CreateCo) -> Self {
		Self {
			parent,
			co,
			membership_core_name: CO_CORE_NAME_MEMBERSHIP.to_owned(),
			keystore_core_name: CO_CORE_NAME_KEYSTORE.to_owned(),
		}
	}

	pub fn with_membership_core_name(self, membership_core_name: String) -> Self {
		Self { membership_core_name, ..self }
	}

	pub fn with_keystore_core_name(self, keystore_core_name: String) -> Self {
		Self { keystore_core_name, ..self }
	}

	/// TODO: Cleanup when something fails?
	pub async fn create<I>(self, storage: CoStorage, runtime: Runtime, identity: I) -> Result<CoId, anyhow::Error>
	where
		I: PrivateIdentity + Debug + Send + Sync + 'static,
	{
		// storage
		let (storage, encrypted_storage): (CoStorage, Option<(EncryptedBlockStorage<CoStorage>, String, Secret)>) =
			match self.co.algorithm {
				Some(algorithm) => {
					let key_uri = format!("urn:co:{}:{}", self.co.id, uuid::Uuid::new_v4());
					let key = algorithm.generate_serect();
					let result_storage = EncryptedBlockStorage::new(storage, key.clone(), algorithm);
					(CoStorage::new(result_storage.clone()), Some((result_storage, key_uri, key)))
				},
				None => (storage, None),
			};

		// log
		let log =
			Log::new(self.co.id.as_str().as_bytes().to_vec(), create_identity_resolver(), storage, Default::default());

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
		let state = reducer.state().ok_or(anyhow::anyhow!("Expected state after create"))?;

		// store key in parent co
		let (key_uri, encryption_mapping) = if let Some((encrypted_storage, key_uri, secret)) = encrypted_storage {
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
			(Some(key_uri), encrypted_storage.flush_mapping().await?)
		} else {
			(None, None)
		};

		// add membership to parent co
		let membership: Membership = Membership {
			id: self.co.id.to_owned(),
			did: identity.identity().to_owned(),
			heads: reducer.heads().clone(),
			state,
			encryption_mapping,
			key: key_uri,
			membership_state: co_core_membership::MembershipState::Active,
			tags: tags!(),
		};
		self.parent
			.push(&identity, &self.membership_core_name, &MembershipsAction::Join(membership))
			.await?;

		// result
		Ok(self.co.id)
	}
}

struct MembershipWriter<I> {
	/// The membership CO UUID.
	id: CoId,
	/// The membership DID.
	// did: Did,
	parent: CoReducer,
	membership_core_name: String,
	identity: I,
	encrypted_storage: Option<EncryptedBlockStorage<CoStorage>>,
}
#[async_trait]
impl<I> ReducerChangedHandler<CoStorage, DynamicCoreResolver<CoStorage>> for MembershipWriter<I>
where
	I: PrivateIdentity + Debug + Send + Sync,
{
	async fn on_state_changed(
		&mut self,
		reducer: &Reducer<CoStorage, DynamicCoreResolver<CoStorage>>,
		_context: ReducerChangeContext,
	) -> Result<(), anyhow::Error> {
		if let Some(state) = reducer.state() {
			let mapping = match &self.encrypted_storage {
				Some(storage) => storage.flush_mapping().await?,
				None => None,
			};

			// update
			self.parent
				.push(
					&self.identity,
					&self.membership_core_name,
					&MembershipsAction::Update {
						id: self.id.to_owned(),
						state: *state,
						heads: reducer.heads().clone(),
						encryption_mapping: mapping,
					},
				)
				.await?;
		}
		Ok(())
	}
}
