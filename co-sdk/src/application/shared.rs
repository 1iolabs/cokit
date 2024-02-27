use super::identity::create_identity_resolver;
use crate::{
	drivers::network::{subscribe::Publish, CoNetworkTaskSpawner},
	library::co_peer_provider::CoPeerProvider,
	types::co_storage::CoBlockStorageContentMapping,
	CoCoreResolver, CoReducer, CoStorage, Reducer, ReducerBuilder, ReducerChangedHandler, Runtime, CO_CORE_NAME_CO,
	CO_CORE_NAME_KEYSTORE, CO_CORE_NAME_MEMBERSHIP,
};
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
use std::fmt::Debug;

/// Shared CO Builder.
/// The Shared CO state is sptrend in an membership of an other CO (typicalle the Local CO).
pub struct SharedCoBuilder {
	parent: CoReducer,
	keystore_core_name: String,
	membership_core_name: String,
	membership: Membership,
	network: Option<CoNetworkTaskSpawner>,
	initialize: bool,
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
		I: PrivateIdentity + Debug + Send + Sync + 'static,
	{
		// storage
		let (storage, encrypted_storage): (CoStorage, Option<EncryptedBlockStorage<CoStorage>>) =
			match &self.membership.key {
				// encrypted
				Some(key_reference) => {
					let key_store: co_core_keystore::KeyStore = self.parent.state(&self.keystore_core_name).await?;
					let key = key_store
						.shared_key(key_reference)
						.ok_or(anyhow::anyhow!("Shared key not found: {}", key_reference))?;
					let mut result_storage =
						EncryptedBlockStorage::new(storage, Secret::new(key.divulge().to_vec()), Default::default());
					if let Some(mapping) = &self.membership.encryption_mapping {
						result_storage.load_mapping(mapping).await?;
					}
					(CoStorage::new(result_storage.clone()), Some(result_storage))
				},
				// plain
				None => (storage, None),
			};

		// network
		let storage = if let Some(network) = &self.network {
			let mut network_storage = NetworkBlockStorage::new(storage.clone(), network.clone());
			network_storage.set_peers(CoPeerProvider::new(storage, None));
			if let Some(encrypted) = &encrypted_storage {
				network_storage.set_mapping(encrypted.content_mapping());
			}
			CoStorage::new(network_storage)
		} else {
			storage
		};

		// log
		let log = Log::new(
			self.membership.id.as_str().as_bytes().to_vec(),
			create_identity_resolver(),
			storage,
			self.membership.heads.clone(),
		);

		// reducer
		let mut reducer = ReducerBuilder::new(CoCoreResolver::default(), log)
			.with_initialize(self.initialize)
			.with_latest_state(self.membership.state, self.membership.heads.clone())
			.build(runtime.runtime())
			.await?;

		// publish changes
		if let Some(network) = self.network {
			let mapping = encrypted_storage.as_ref().map(|e| e.content_mapping());
			let publish = Publish::new(network, self.membership.id.clone(), mapping, true);
			reducer.add_change_handler(Box::new(publish));
		}

		// mapping
		let mapping = encrypted_storage
			.as_ref()
			.map(|e| e.content_mapping())
			.map(CoBlockStorageContentMapping::new);

		// setup auto write to parent co
		let writer = MembershipWriter {
			id: self.membership.id.clone(),
			parent: self.parent,
			membership_core_name: self.membership_core_name,
			identity,
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
	async fn on_state_changed(&mut self, reducer: &Reducer<CoStorage, CoCoreResolver>) -> Result<(), anyhow::Error> {
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
