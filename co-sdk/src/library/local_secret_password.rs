// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

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
