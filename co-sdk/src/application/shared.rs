use super::identity::create_identity_resolver;
use crate::{
	drivers::network::{publish::CoHeadsPublish, CoNetworkTaskSpawner},
	library::{co_peer_provider::CoPeerProvider, co_state::CoState, push_heads::PushHeads},
	state::find,
	types::co_storage::CoBlockStorageContentMapping,
	CoCoreResolver, CoReducer, CoStorage, Reducer, ReducerBuilder, ReducerChangedContext, ReducerChangedHandler,
	Runtime, CO_CORE_NAME_CO, CO_CORE_NAME_KEYSTORE, CO_CORE_NAME_MEMBERSHIP,
};
use anyhow::anyhow;
use async_trait::async_trait;
use co_core_co::CoAction;
use co_core_keystore::{Key, KeyStoreAction};
use co_core_membership::{Membership, MembershipsAction};
use co_identity::PrivateIdentity;
use co_log::Log;
use co_network::NetworkBlockStorage;
use co_primitives::{tags, CoId};
use co_storage::{Algorithm, EncryptedBlockStorage, Secret};
use serde::{Deserialize, Serialize};
use std::{fmt::Debug, time::Duration};

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

	pub async fn build<I>(self, storage: CoStorage, runtime: Runtime, identity: I) -> Result<CoReducer, anyhow::Error>
	where
		I: PrivateIdentity + Debug + Send + Sync + Clone + 'static,
	{
		// storage
		let (storage, encrypted_storage): (CoStorage, Option<EncryptedBlockStorage<CoStorage>>) =
			match &self.membership.key {
				// encrypted
				Some(key_reference) => {
					let key_store: co_core_keystore::KeyStore = self.parent.state(&self.keystore_core_name).await?;
					let (_, key) = find(&self.parent.storage(), &key_store.keys, |(k, _)| k == key_reference)
						.await?
						.ok_or(anyhow::anyhow!("Shared key not found: {}", key_reference))?;
					let secret = match key.secret {
						co_core_keystore::Secret::SharedKey(sec) => Ok(sec),
						_ => Err(anyhow!("Invalid secret")),
					}?;
					let mut result_storage =
						EncryptedBlockStorage::new(storage, Secret::new(secret.divulge().to_vec()), Default::default());
					if let Some(mapping) = &self.membership.encryption_mapping {
						result_storage.load_mapping(mapping).await?;
					}
					(CoStorage::new(result_storage.clone()), Some(result_storage))
				},
				// plain
				None => (storage, None),
			};

		// network
		let (storage, co_state) = if let Some(network) = &self.network {
			let co_state = CoState::default();
			let peer_provider = CoPeerProvider::new(
				network.clone(),
				create_identity_resolver(),
				identity.clone(),
				storage.clone(),
				self.membership.id.clone(),
				co_state.clone(),
			);
			let mut network_storage =
				NetworkBlockStorage::new(storage.clone(), network.clone(), peer_provider, self.network_block_timeout);
			if let Some(encrypted) = &encrypted_storage {
				network_storage.set_mapping(encrypted.content_mapping());
			}
			(CoStorage::new(network_storage), Some(co_state))
		} else {
			(storage, None)
		};

		// log
		let log = Log::new(
			self.membership.id.as_str().as_bytes().to_vec(),
			create_identity_resolver(),
			storage.clone(),
			self.membership.heads.clone(),
		);

		// reducer
		let mut reducer = ReducerBuilder::new(CoCoreResolver::default(), log)
			.with_initialize(self.initialize)
			.with_latest_state(self.membership.state, self.membership.heads.clone())
			.build(runtime.runtime())
			.await?;

		// push changes to all connectable peers
		if let Some(network) = &self.network {
			let mapping = encrypted_storage.as_ref().map(|e| e.content_mapping());
			let peer_provider = CoPeerProvider::new(
				network.clone(),
				create_identity_resolver(),
				identity.clone(),
				storage.clone(),
				self.membership.id.clone(),
				co_state.clone().unwrap(),
			);
			let publish = PushHeads::new(
				network.clone(),
				self.membership.id.clone(),
				identity.clone(),
				peer_provider,
				mapping.clone(),
				true,
			);
			reducer.add_change_handler(Box::new(publish));
		}

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

		// mapping
		let mapping = encrypted_storage
			.as_ref()
			.map(|e| e.content_mapping())
			.map(CoBlockStorageContentMapping::new);

		// setup auto write to parent co
		let writer = MembershipWriter {
			id: self.membership.id.clone(),
			parent: self.parent.clone(),
			membership_core_name: self.membership_core_name,
			identity: identity.clone(),
			encrypted_storage,
		};
		reducer.add_change_handler(Box::new(writer));

		// result
		Ok(CoReducer::new(self.membership.id, runtime, reducer, mapping))
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
		let (storage, encrypted_storage): (CoStorage, Option<(EncryptedBlockStorage<CoStorage>, Secret)>) =
			match self.co.algorithm {
				Some(algorithm) => {
					let key = algorithm.generate_serect();
					let result_storage = EncryptedBlockStorage::new(storage, key.clone(), algorithm);
					(CoStorage::new(result_storage.clone()), Some((result_storage, key)))
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
		reducer
			.push(
				runtime.runtime(),
				&identity,
				CO_CORE_NAME_CO,
				&CoAction::Create {
					id: self.co.id.to_owned(),
					name: self.co.name.to_owned(),
					cores: Default::default(),
					participants: Default::default(),
				},
			)
			.await?;
		let state = reducer.state().ok_or(anyhow::anyhow!("Expected state after create"))?;

		// store key in parent co
		let (key, encryption_mapping) = if let Some((encrypted_storage, secret)) = encrypted_storage {
			let key_uri = format!("urn:co:{}:{}", self.co.id, uuid::Uuid::new_v4());
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
			key,
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
impl<I> ReducerChangedHandler<CoStorage, CoCoreResolver> for MembershipWriter<I>
where
	I: PrivateIdentity + Debug + Send + Sync,
{
	async fn on_state_changed(
		&mut self,
		reducer: &Reducer<CoStorage, CoCoreResolver>,
		_context: ReducerChangedContext,
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
						state: state.clone(),
						heads: reducer.heads().clone(),
						encryption_mapping: mapping,
					},
				)
				.await?;
		}
		Ok(())
	}
}
