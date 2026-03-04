// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model
// training or retention—approved secure tools may process solely for internal use.

use async_trait::async_trait;
use co_primitives::CoId;
use std::{
	fmt::{Debug, Formatter},
	sync::Arc,
};

/// Application-defined policy for granting CO key access to non-participants.
#[async_trait]
pub trait CoAccessPolicy: Send + Sync + 'static {
	async fn check_access(&self, co: &CoId, requester: &str) -> Result<bool, anyhow::Error>;
}

#[derive(Clone)]
pub struct DynamicCoAccessPolicy(Arc<dyn CoAccessPolicy>);
impl Debug for DynamicCoAccessPolicy {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.debug_tuple("DynamicCoAccessPolicy").finish()
	}
}
impl DynamicCoAccessPolicy {
	pub fn new(policy: impl CoAccessPolicy) -> Self {
		Self(Arc::new(policy))
	}
}
#[async_trait]
impl CoAccessPolicy for DynamicCoAccessPolicy {
	async fn check_access(&self, co: &CoId, requester: &str) -> Result<bool, anyhow::Error> {
		self.0.check_access(co, requester).await
	}
}
