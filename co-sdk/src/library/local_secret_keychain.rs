// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::library::local_secret::LocalSecret;
use async_trait::async_trait;
use co_primitives::Secret;
use co_storage::Algorithm;

pub struct KeychainLocalSecret {
	service: String,
	user: String,
}
impl KeychainLocalSecret {
	pub fn new(service: String, user: String) -> Self {
		Self { service, user }
	}

	/// Get or create encryption key in OS Keychain.
	fn fetch_secret_keychain(service: &str, user: &str, allow_create: bool) -> Result<Secret, anyhow::Error> {
		let entry = keyring::Entry::new(service, user)?;
		let key_as_base64 = match entry.get_password() {
			Ok(p) => p,
			Err(keyring::Error::NoEntry) if allow_create => {
				// generate and set key
				let secret = Algorithm::default().generate_serect();
				let secret_base64 = multibase::encode(multibase::Base::Base64, secret.divulge());
				entry.set_password(&secret_base64)?;

				// fetch again to make sure the key has persisted
				return Self::fetch_secret_keychain(service, user, false);
			},
			Err(e) => return Err(e.into()),
		};
		Ok(Secret::new(multibase::decode(key_as_base64)?.1))
	}
}
#[async_trait]
impl LocalSecret for KeychainLocalSecret {
	async fn fetch(&self) -> Result<Secret, anyhow::Error> {
		Self::fetch_secret_keychain(&self.service, &self.user, true)
	}
}
