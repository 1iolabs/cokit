// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use async_trait::async_trait;
use cid::Cid;

/// A minimal trait to dispatch (push) actions into an reducer/core.
/// Concrete implementations are pre-configured with identity and core informations.
#[async_trait]
pub trait CoDispatch<A>: Sync + Send {
	async fn dispatch(&mut self, action: &A) -> Result<Option<Cid>, anyhow::Error>;
}

pub struct DynamicCoDispatch<A> {
	dispatch: Box<dyn CoDispatch<A> + 'static>,
}
impl<A> DynamicCoDispatch<A>
where
	A: Send + Sync + 'static,
{
	pub fn new(dispatch: impl CoDispatch<A> + 'static) -> Self {
		Self { dispatch: Box::new(dispatch) }
	}
}
#[async_trait]
impl<A> CoDispatch<A> for DynamicCoDispatch<A>
where
	A: Send + Sync + 'static,
{
	async fn dispatch(&mut self, action: &A) -> Result<Option<Cid>, anyhow::Error> {
		self.dispatch.dispatch(action).await
	}
}
