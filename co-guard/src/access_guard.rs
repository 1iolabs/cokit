// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use async_trait::async_trait;
use co_primitives::CoId;
use std::{
	fmt::{Debug, Formatter},
	sync::Arc,
};

/// Application-defined guard for granting CO key access to non-participants.
#[async_trait]
pub trait AccessGuard: Send + Sync + 'static {
	async fn check_access(&self, co: &CoId, requester: &str) -> Result<bool, anyhow::Error>;
}

#[derive(Clone)]
pub struct DynamicAccessGuard(Arc<dyn AccessGuard>);
impl Debug for DynamicAccessGuard {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.debug_tuple("DynamicAccessGuard").finish()
	}
}
impl DynamicAccessGuard {
	pub fn new(guard: impl AccessGuard) -> Self {
		Self(Arc::new(guard))
	}
}
#[async_trait]
impl AccessGuard for DynamicAccessGuard {
	async fn check_access(&self, co: &CoId, requester: &str) -> Result<bool, anyhow::Error> {
		self.0.check_access(co, requester).await
	}
}
