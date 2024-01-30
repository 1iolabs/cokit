use crate::{EntryError, IdentityResolverError};
use co_storage::StorageError;

#[derive(Debug, thiserror::Error)]
pub enum LogError {
	#[error("Storage error")]
	Storage(#[from] StorageError),

	#[error("Entry error")]
	Entry(#[from] EntryError),

	#[error("Identity resolver error")]
	IdentityResolver(#[from] IdentityResolverError),

	#[error("Invalid argument")]
	InvalidArgument(#[from] anyhow::Error),
}
