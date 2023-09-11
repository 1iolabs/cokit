use crate::Reducer;

/// Store API which is `Sync + Clone`.
#[async_trait::async_trait]
pub trait SyncStoreApi<R>
where
	R: Reducer + Send + 'static,
{
	/// Dispatch action.
	async fn dispatch(&self, action: R::Action);

	/// Get clone of current state.
	async fn state(&self) -> R::State;
}
