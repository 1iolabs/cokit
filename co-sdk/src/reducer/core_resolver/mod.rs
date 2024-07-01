use async_trait::async_trait;
use co_runtime::{ExecuteError, RuntimePool};
use co_storage::StorageError;
use libipld::Cid;

pub mod co;
pub mod single;

#[async_trait]
pub trait CoreResolver<S> {
	/// Apply action to root state.
	async fn execute(
		&self,
		storage: &S,
		runtime: &RuntimePool,
		state: &Option<Cid>,
		action: &Cid,
	) -> Result<Option<Cid>, CoreResolverError>;
}

#[derive(Debug, thiserror::Error)]
pub enum CoreResolverError {
	/// Storage error.
	#[error("Storage error")]
	Storage(#[from] StorageError),

	/// Invalid arguemnt (action) supplied to the resolver.
	#[error("Invalid argument")]
	InvalidArgument(#[from] anyhow::Error),

	/// The core referenced by the action can not be found.
	#[error("Core not found: {0}")]
	CoreNotFound(String),

	/// The core referenced by the action can not be found.
	#[error("Execute core failed: {0}")]
	Execute(String, #[source] ExecuteError),
}
