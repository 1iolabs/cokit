use async_trait::async_trait;
use cid::Cid;

/// A minimal trait to dispatch (push) actions into an reducer/core.
/// Concrete implementations are pre-configured with identity and core informations.
#[async_trait]
pub trait CoDispatch<A>: Sync + Send {
	async fn dispatch(&self, action: &A) -> Result<Option<Cid>, anyhow::Error>;
}
