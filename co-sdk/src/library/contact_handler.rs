// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use async_trait::async_trait;
use co_identity::DidCommHeader;
use co_primitives::Did;
use std::{
	fmt::{Debug, Formatter},
	sync::Arc,
};

/// Handler for incoming contact requests.
#[async_trait]
pub trait ContactHandler: Send + Sync + 'static {
	async fn handle_contact(&self, sender: &Did, header: &DidCommHeader) -> Result<(), anyhow::Error>;
}

#[derive(Clone)]
pub struct DynamicContactHandler(Arc<dyn ContactHandler>);
impl Debug for DynamicContactHandler {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.debug_tuple("DynamicContactHandler").finish()
	}
}
impl DynamicContactHandler {
	pub fn new(handler: impl ContactHandler) -> Self {
		Self(Arc::new(handler))
	}
}
#[async_trait]
impl ContactHandler for DynamicContactHandler {
	async fn handle_contact(&self, sender: &Did, header: &DidCommHeader) -> Result<(), anyhow::Error> {
		self.0.handle_contact(sender, header).await
	}
}
