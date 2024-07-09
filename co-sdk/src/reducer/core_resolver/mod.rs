use async_trait::async_trait;
use co_runtime::{ExecuteError, RuntimePool};
use co_storage::StorageError;
use libipld::Cid;

use crate::ReducerChangeContext;

pub mod co;
pub mod dynamic;
pub mod epic;
pub mod log;
pub mod membership;
pub mod single;

#[async_trait]
pub trait CoreResolver<S> {
	/// Apply action to root state.
	///
	/// This execute operation has to be deterministic I.E. is not allowed to introduce nont deterministic values into the core.
	/// This also implies that usage of `context` parameter is only intendet for side-effects.
	///
	/// When this method is called the Reducer is write locked, so every side-effect which accesses the reducer has to be executed out-of-band (queued, spawned).
	async fn execute(
		&self,
		storage: &S,
		runtime: &RuntimePool,
		context: &ReducerChangeContext,
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

	/// The core reported an error while executing.
	#[error("Execute core failed: {0}")]
	Execute(String, #[source] ExecuteError),

	/// A core resolver middleware reported an error (usually before or after executing).
	#[error("Execute core failed: {0}")]
	Middleware(#[source] anyhow::Error),
}
