use crate::{
	application::shared::SharedCoBuilder, library::shared_membership::shared_membership,
	types::co_reducer_factory::CoReducerFactoryError, ApplicationMessage, CoReducer, CoStorage, Cores,
};
use anyhow::anyhow;
use co_actor::ActorHandle;
use co_core_membership::Membership;
use co_primitives::CoId;
use co_storage::{EncryptedBlockStorage, EncryptionReferenceMode, StorageError};
use std::time::Duration;

/// Reducer storage implementations.
#[derive(Debug, Clone)]
pub enum ReducerStorage {
	Default(CoStorage),
	Encrypted(CoStorage, EncryptedBlockStorage<CoStorage>),
}
impl ReducerStorage {
	pub fn storage(&self) -> &CoStorage {
		match self {
			ReducerStorage::Default(storage) => storage,
			ReducerStorage::Encrypted(storage, _encrypted) => storage,
		}
	}

	pub fn encrypted_storage(&self) -> Option<&EncryptedBlockStorage<CoStorage>> {
		match self {
			ReducerStorage::Default(_) => None,
			ReducerStorage::Encrypted(_, encrypted) => Some(encrypted),
		}
	}

	pub(crate) async fn from_id(
		handle: ActorHandle<ApplicationMessage>,
		storage: CoStorage,
		parent: CoReducer,
		id: CoId,
		key_request_timeout: Duration,
	) -> Result<ReducerStorage, CoReducerFactoryError> {
		let membership = shared_membership(&parent, &id, None)
			.await?
			.ok_or(CoReducerFactoryError::CoNotFound(id, anyhow!("No active membership")))?;
		Ok(Self::from_membership(handle, &storage, &parent, membership, key_request_timeout)
			.await
			.map_err(|e| CoReducerFactoryError::Other(e.into()))?)
	}

	pub(crate) async fn from_membership(
		handle: ActorHandle<ApplicationMessage>,
		storage: &CoStorage,
		parent: &CoReducer,
		membership: Membership,
		key_request_timeout: Duration,
	) -> Result<ReducerStorage, StorageError> {
		let builder =
			SharedCoBuilder::new(parent.clone(), membership.clone()).with_key_request_timeout(key_request_timeout);
		let secret = builder.secret(Some(handle)).await?;
		Ok(match secret {
			Some(secret) => {
				let builtin_cores = Cores::default()
					.built_in_native_mapping()
					.into_iter()
					.map(|(cid, _)| cid)
					.collect();
				let encrypted_storage =
					EncryptedBlockStorage::new(storage.clone(), secret.into(), Default::default(), Default::default())
						.with_encryption_reference_mode(EncryptionReferenceMode::DisallowExcept(builtin_cores));
				for state in membership.state {
					if let Some(encryption_mapping) = &state.encryption_mapping {
						encrypted_storage.load_mapping(encryption_mapping).await?;
					}
				}
				ReducerStorage::Encrypted(CoStorage::new(encrypted_storage.clone()), encrypted_storage)
			},
			None => ReducerStorage::Default(storage.clone()),
		})
	}
}
impl AsRef<CoStorage> for ReducerStorage {
	fn as_ref(&self) -> &CoStorage {
		self.storage()
	}
}
