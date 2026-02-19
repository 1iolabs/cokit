// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::CoReducer;
use async_trait::async_trait;
use co_actor::ActorError;
use co_primitives::CoId;

#[async_trait]
pub trait CoReducerFactory {
	/// Get instance of CoReducer.
	/// Returns None if `co` membership could not be found.
	/// TODO: Refactor to own error type and remove option.
	async fn co_reducer(&self, co: &CoId) -> Result<Option<CoReducer>, anyhow::Error>;

	async fn try_co_reducer(&self, co: &CoId) -> Result<CoReducer, CoReducerFactoryError>;
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CoReducerFactoryError {
	#[error("CO not found: {0:?}")]
	CoNotFound(CoId, #[source] anyhow::Error),

	#[error("Create CO failed: {0:?}")]
	Create(CoId, #[source] anyhow::Error),

	#[error("CO failed")]
	Other(#[from] anyhow::Error),

	#[error("CO actor error")]
	Actor(#[from] ActorError),

	#[error("CO create pending")]
	Pending,

	#[error("CO not initialized yet")]
	WouldCreate,
}

pub trait CoReducerFactoryResultExt<T> {
	/// Return None for pending/uninitialized COs.
	fn opt(self) -> Result<Option<T>, CoReducerFactoryError>;
}
impl<T> CoReducerFactoryResultExt<T> for Result<T, CoReducerFactoryError> {
	fn opt(self) -> Result<Option<T>, CoReducerFactoryError> {
		match self {
			Err(CoReducerFactoryError::Pending) => Ok(None),
			Err(CoReducerFactoryError::WouldCreate) => Ok(None),
			Ok(value) => Ok(Some(value)),
			Err(err) => Err(err),
		}
	}
}
