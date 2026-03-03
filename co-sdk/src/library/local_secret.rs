// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use async_trait::async_trait;
use co_primitives::Secret;
use co_storage::Algorithm;
use std::{
	fmt::{Debug, Formatter},
	sync::Arc,
};

#[async_trait]
pub trait LocalSecret: Send + Sync + 'static {
	async fn fetch(&self) -> Result<(Algorithm, Secret), anyhow::Error>;
}

pub struct MemoryLocalSecret {
	algorithm: Algorithm,
	secret: co_storage::Secret,
}
impl MemoryLocalSecret {
	pub fn generate() -> Self {
		let algorithm = Algorithm::default();
		Self { secret: algorithm.generate_serect(), algorithm }
	}
}
#[async_trait]
impl LocalSecret for MemoryLocalSecret {
	async fn fetch(&self) -> Result<(Algorithm, Secret), anyhow::Error> {
		Ok((self.algorithm, self.secret.clone().into()))
	}
}

#[derive(Clone)]
pub struct DynamicLocalSecret(Arc<dyn LocalSecret>);
impl Debug for DynamicLocalSecret {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.debug_tuple("DynamicLocalSecret").finish()
	}
}
impl DynamicLocalSecret {
	pub fn new(secret: impl LocalSecret) -> Self {
		Self(Arc::new(secret))
	}
}
#[async_trait]
impl LocalSecret for DynamicLocalSecret {
	async fn fetch(&self) -> Result<(Algorithm, Secret), anyhow::Error> {
		self.0.fetch().await
	}
}
