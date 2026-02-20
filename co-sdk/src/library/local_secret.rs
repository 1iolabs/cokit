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
