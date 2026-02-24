// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use async_trait::async_trait;
use co_primitives::Secret;
use co_storage::Algorithm;

#[async_trait]
pub trait LocalSecret {
	async fn fetch(&self) -> Result<Secret, anyhow::Error>;
}

pub struct MemoryLocalSecret {
	secret: co_storage::Secret,
}
impl MemoryLocalSecret {
	pub fn new() -> Self {
		Self { secret: Algorithm::default().generate_serect() }
	}
}
#[async_trait]
impl LocalSecret for MemoryLocalSecret {
	async fn fetch(&self) -> Result<Secret, anyhow::Error> {
		Ok(self.secret.clone().into())
	}
}
