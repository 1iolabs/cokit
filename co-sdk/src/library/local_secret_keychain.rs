// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::library::local_secret::LocalSecret;
use async_trait::async_trait;
use co_primitives::Secret;
use co_storage::Algorithm;

pub struct KeychainLocalSecret {
	service: String,
	user: String,
	algorithm: Algorithm,
}
impl KeychainLocalSecret {
	pub fn new(service: String, user: String) -> Self {
		Self { service, user, algorithm: Algorithm::default() }
	}

	/// Get or create encryption key in OS Keychain.
	fn fetch_secret_keychain(
		service: &str,
		user: &str,
		algorithm: Algorithm,
		allow_create: bool,
	) -> Result<Secret, anyhow::Error> {
		let entry = keyring::Entry::new(service, user)?;
		let key_as_base64 = match entry.get_password() {
			Ok(p) => p,
			Err(keyring::Error::NoEntry) if allow_create => {
				// generate and set key
				let secret = algorithm.generate_serect();
				let secret_base64 = multibase::encode(multibase::Base::Base64, secret.divulge());
				entry.set_password(&secret_base64)?;

				// fetch again to make sure the key has persisted
				return Self::fetch_secret_keychain(service, user, algorithm, false);
			},
			Err(e) => return Err(e.into()),
		};
		Ok(Secret::new(multibase::decode(key_as_base64)?.1))
	}
}
#[async_trait]
impl LocalSecret for KeychainLocalSecret {
	async fn fetch(&self) -> Result<(Algorithm, Secret), anyhow::Error> {
		Ok((self.algorithm, Self::fetch_secret_keychain(&self.service, &self.user, self.algorithm, true)?))
	}
}
