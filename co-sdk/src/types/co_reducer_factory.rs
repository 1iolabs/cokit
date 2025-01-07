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
pub enum CoReducerFactoryError {
	#[error("CO not found: {0:?}")]
	CoNotFound(CoId),

	#[error("Create CO failed: {0:?}")]
	Create(CoId, #[source] anyhow::Error),

	#[error("CO failed")]
	Other(#[from] anyhow::Error),

	#[error("CO actor error")]
	Actor(#[from] ActorError),
}
