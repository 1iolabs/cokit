use crate::ReducerChangeContext;
use async_trait::async_trait;
use cid::Cid;
use co_runtime::{ExecuteError, RuntimeContext, RuntimePool};
use co_storage::StorageError;

pub mod change;
pub mod co;
pub mod dynamic;
pub mod epic;
pub mod flush_overlay;
pub mod log;
pub mod membership;
pub mod overlay;
pub mod reference;
pub mod single;

#[async_trait]
pub trait CoreResolver<S> {
	/// Apply action to root state.
	///
	/// This execute operation has to be deterministic I.E. is not allowed to introduce not deterministic values into
	/// the core. This also implies that usage of `context` parameter is only intendet for side-effects.
	/// If the implementation executes extra actions (to `state`) these have to be determinisitc as they are not
	/// reflected in the heads. If the action is deterministic there is also no need to store it because it gets
	/// recomputed every time this resolver executes.
	///
	/// When this method is called the Reducer is write locked, so every side-effect which accesses the reducer has to
	/// be executed out-of-band (queued, spawned).
	async fn execute(
		&self,
		storage: &S,
		runtime: &RuntimePool,
		context: &ReducerChangeContext,
		state: &Option<Cid>,
		action: &Cid,
	) -> Result<RuntimeContext, CoreResolverError>;
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
