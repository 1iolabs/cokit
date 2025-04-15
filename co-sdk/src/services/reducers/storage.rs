use crate::{
	application::shared::SharedCoBuilder, library::shared_membership::shared_membership,
	types::co_reducer_factory::CoReducerFactoryError, CoReducer, CoStorage, Cores,
};
use co_core_membership::Membership;
use co_primitives::CoId;
use co_storage::{EncryptedBlockStorage, EncryptionReferenceMode, StorageError};

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
		storage: CoStorage,
		parent: CoReducer,
		id: CoId,
	) -> Result<ReducerStorage, CoReducerFactoryError> {
		let membership = shared_membership(&parent, &id, None)
			.await?
			.ok_or(CoReducerFactoryError::CoNotFound(id))?;
		Ok(Self::from_membership(&storage, &parent, membership)
			.await
			.map_err(|e| CoReducerFactoryError::Other(e.into()))?)
	}

	pub(crate) async fn from_membership(
		storage: &CoStorage,
		parent: &CoReducer,
		membership: Membership,
	) -> Result<ReducerStorage, StorageError> {
		let builder = SharedCoBuilder::new(parent.clone(), membership.clone());
		let secret = builder.secret().await?;
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
