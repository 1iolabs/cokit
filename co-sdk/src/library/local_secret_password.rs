// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::LocalSecret;
use anyhow::anyhow;
use async_trait::async_trait;
use co_primitives::Secret;
use co_storage::Algorithm;

pub const PASSWORD_DERIVATION: &str = "co 2026-03-02T10:05:29Z password derivation v1";

pub struct PasswordLocalSecret {
	algorithm: Algorithm,
	secret: Secret,
}
impl PasswordLocalSecret {
	pub fn new(password: impl Into<String>, algorithm: Algorithm) -> Result<Self, anyhow::Error> {
		let password = password.into();
		let derived = co_storage::Secret::new(password.into_bytes()).derive_serect(PASSWORD_DERIVATION);
		if algorithm.key_size() != derived.divulge().len() {
			return Err(anyhow!("Invalid key size: {} != {} bytes", derived.divulge().len(), algorithm.key_size()));
		}
		Ok(Self { algorithm, secret: derived.into() })
	}
}
#[async_trait]
impl LocalSecret for PasswordLocalSecret {
	async fn fetch(&self) -> Result<(Algorithm, Secret), anyhow::Error> {
		Ok((self.algorithm, self.secret.clone()))
	}
}
