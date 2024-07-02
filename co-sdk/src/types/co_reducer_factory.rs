use crate::CoReducer;
use async_trait::async_trait;
use co_primitives::CoId;

#[async_trait]
pub trait CoReducerFactory {
	/// Get instance of CoReducer.
	/// Returns None if `co` membership could not be found.
	/// TODO: Refactor to own error type and remove option.
	async fn co_reducer(&self, co: &CoId) -> Result<Option<CoReducer>, anyhow::Error>;
}
