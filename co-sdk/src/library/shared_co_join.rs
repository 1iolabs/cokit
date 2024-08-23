use crate::{
	application::identity::create_identity_resolver, drivers::network::CoNetworkTaskSpawner, state, CoCoreResolver,
	CoReducer, CoStorage, CoToken, CoTokenParameters, ReducerBuilder, CO_CORE_NAME_CO, CO_CORE_NAME_KEYSTORE,
	CO_CORE_NAME_MEMBERSHIP,
};
use anyhow::anyhow;
use co_core_co::Co;
use co_core_keystore::{Key, KeyStoreAction};
use co_core_membership::{Membership, MembershipsAction};
use co_identity::{IdentityBox, PrivateIdentity};
use co_log::Log;
use co_network::{bitswap::NetworkBlockStorage, StaticPeerProvider};
use co_primitives::{tags, CoId, Network, Secret};
use co_runtime::RuntimePool;
use co_storage::{BlockStorage, EncryptedBlockStorage};
use futures::{stream, StreamExt, TryStreamExt};
use libipld::Cid;
use libp2p::PeerId;
use std::{collections::BTreeSet, fmt::Debug, time::Duration};

/// Join Shared COs that are currently unknown.
///
/// TODO: Implement consensus validation.
/// TODO: Implement _identites.
/// TODO: Implement _network.
#[derive(Debug, Clone)]
pub struct SharedCoJoin {
	id: CoId,

	keystore_core_name: String,
	membership_core_name: String,

	/// Known participants.
	_identites: Vec<IdentityBox>,

	/// Networks where to CO can be accessed.
	_network: BTreeSet<Network>,

	/// Trusted peers.
	/// We know from some other trusted source that there peers are participants.
	peers: BTreeSet<PeerId>,

	heads: BTreeSet<Cid>,
	state: Option<Cid>,
	key: Option<Secret>,
}
impl SharedCoJoin {
	pub fn new(id: CoId) -> Self {
		Self {
			id,
			peers: Default::default(),
			heads: Default::default(),
			key: Default::default(),
			_identites: Default::default(),
			_network: Default::default(),
			state: Default::default(),
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

	pub fn with_trusted_peer(self, value: PeerId) -> Self {
		let mut peers = self.peers;
		peers.insert(value);
		Self { peers, ..self }
	}

	pub fn with_encryption(self, secret: Secret) -> Self {
		Self { key: Some(secret), ..self }
	}

	pub fn with_heads(self, heads: BTreeSet<Cid>, state: Option<Cid>) -> Self {
		Self { heads, state, ..self }
	}

	#[tracing::instrument(skip(network, storage, runtime))]
	pub async fn join<I>(
		&self,
		runtime: &RuntimePool,
		network: Option<CoNetworkTaskSpawner>,
		storage: CoStorage,
		parent: CoReducer,
		identity: I,
	) -> Result<(), SharedCoJoinError>
	where
		I: PrivateIdentity + Debug + Clone + Send + Sync + 'static,
	{
		if self.heads.is_empty() {
			return Err(SharedCoJoinError::NoHeads);
		}

		// storage
		let (storage, encrypted_storage) = {
			if let Some(key) = &self.key {
				let encrypted_storage =
					EncryptedBlockStorage::new(storage, key.clone().into(), Default::default(), Default::default());
				(CoStorage::new(encrypted_storage.clone()), Some(encrypted_storage))
			} else {
				(storage, None)
			}
		};

		// network
		let storage = if let Some(network) = network {
			if self.peers.is_empty() {
				return Err(SharedCoJoinError::InsufficentPeers);
			}
			let peer_provider = StaticPeerProvider::new(self.peers.clone());
			let mut network_storage =
				NetworkBlockStorage::new(storage.clone(), network.clone(), peer_provider, Duration::from_secs(1));
			if let Some(encrypted) = &encrypted_storage {
				network_storage.set_mapping(encrypted.content_mapping());
			}
			if let Some(shared_secret) = &self.key {
				let token = CoToken::new(shared_secret, CoTokenParameters(network.local_peer_id(), self.id.clone()))
					.map_err(SharedCoJoinError::Network)?
					.to_bitswap_token()
					.map_err(SharedCoJoinError::Network)?;
				network_storage.set_tokens(vec![token]);
			}
			CoStorage::new(network_storage)
		} else {
			storage
		};

		// fetch state/heads to use mapped cids internally
		let state = if let Some(state) = self.state {
			let state_block = storage.get(&state).await.map_err(|e| SharedCoJoinError::Join(e.into()))?;
			Some(*state_block.cid())
		} else {
			self.state
		};
		let heads: BTreeSet<_> = stream::iter(self.heads.clone())
			.then(|cid| {
				let heads_storage = storage.clone();
				async move {
					heads_storage
						.get(&cid)
						.await
						.map_err(|e| SharedCoJoinError::Join(e.into()))
						.map(|block| *block.cid())
				}
			})
			.try_collect()
			.await?;

		// log
		let log =
			Log::new(self.id.as_str().as_bytes().to_vec(), create_identity_resolver(), storage.clone(), heads.clone());

		// reducer
		let mut reducer_builder = ReducerBuilder::new(CoCoreResolver::default(), log);
		if let Some(state) = state {
			reducer_builder = reducer_builder.with_latest_state(state, heads.clone());
		}
		let reducer = reducer_builder.build(runtime).await.map_err(SharedCoJoinError::Reducer)?;
		let state = reducer.state().ok_or(SharedCoJoinError::NoState)?;

		// store: key
		//  by using the URI from the active key
		let (key_uri, encryption_mapping) = if let Some(key) = &self.key {
			if let Some(encrypted_storage) = encrypted_storage {
				let co: Co = state::core_state_or_default(reducer.log().storage(), state.into(), CO_CORE_NAME_CO)
					.await
					.map_err(|e| SharedCoJoinError::Join(e.into()))?;
				if let Some(key_data) = co.keys.as_ref().and_then(|keys| keys.first()) {
					let key_store_key = Key {
						uri: key_data.id.clone(),
						name: format!("co ({})", co.name),
						description: "".to_owned(),
						secret: co_core_keystore::Secret::SharedKey(key.clone()),
						tags: tags!(),
					};
					parent
						.push(&identity, &self.keystore_core_name, &KeyStoreAction::Set(key_store_key))
						.await
						.map_err(SharedCoJoinError::Join)?;
					(
						Some(key_data.id.clone()),
						encrypted_storage
							.flush_mapping()
							.await
							.map_err(|e| SharedCoJoinError::Join(e.into()))?,
					)
				} else {
					return Err(SharedCoJoinError::Join(anyhow!("No keys found")));
				}
			} else {
				(None, None)
			}
		} else {
			(None, None)
		};

		// store: membership
		let membership = Membership {
			id: self.id.to_owned(),
			did: identity.identity().to_owned(),
			heads: reducer.heads().clone(),
			state,
			encryption_mapping,
			key: key_uri,
			membership_state: co_core_membership::MembershipState::Active,
			tags: tags!(),
		};
		tracing::trace!(?membership, "join-membership");
		parent
			.push(&identity, &self.membership_core_name, &MembershipsAction::Join(membership))
			.await
			.map_err(SharedCoJoinError::Join)?;

		// done
		Ok(())
	}
}

#[derive(Debug, thiserror::Error)]
pub enum SharedCoJoinError {
	#[error("Insufficent peers to connect")]
	InsufficentPeers,

	#[error("No heads")]
	NoHeads,

	#[error("Reducer failed")]
	Reducer(#[source] anyhow::Error),

	/// Network error.
	#[error("Network failed")]
	Network(#[source] anyhow::Error),

	/// No state could be computed. This indicates corruption or an empty CO which can not be joined.
	#[error("No state")]
	NoState,

	/// Joining failed because the membership/key could not be stored in the parent CO.
	#[error("Join failed")]
	Join(#[source] anyhow::Error),
}
