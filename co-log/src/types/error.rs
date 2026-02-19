// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::EntryError;
use co_identity::IdentityResolverError;
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

	#[error("Internal error")]
	Internal(#[source] anyhow::Error),
}
