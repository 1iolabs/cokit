// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

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
