// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

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
