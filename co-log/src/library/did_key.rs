use crate::Identity;
use anyhow::anyhow;
use did_key::{generate, resolve, CoreSign, Ed25519KeyPair, Fingerprint, KeyMaterial, PatchedKeyPair};

pub struct DidKeyIdentity {
	did: String,
	key: PatchedKeyPair,
}
impl DidKeyIdentity {
	pub fn generate(seed: Option<&[u8]>) -> Self {
		Self::from_key(generate::<Ed25519KeyPair>(seed))
	}

	fn from_key(key: PatchedKeyPair) -> Self {
		Self { did: key.fingerprint(), key }
	}

	pub fn from_bytes(bytes: &[u8]) -> Result<Self, anyhow::Error> {
		Self::try_from(bytes)
	}

	pub fn to_bytes(&self) -> &[u8] {
		self.identity().as_bytes()
	}
}
impl TryFrom<&[u8]> for DidKeyIdentity {
	type Error = anyhow::Error;

	fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
		Ok(Self::from_key(resolve(std::str::from_utf8(value)?).map_err(|e| anyhow!("resolve failed: {:?}", e))?))
	}
}
impl Identity for DidKeyIdentity {
	fn identity(&self) -> &str {
		&self.did
	}

	fn public_key(&self) -> Option<Vec<u8>> {
		Some(self.key.public_key_bytes())
	}

	fn sign(&self, data: &[u8]) -> Vec<u8> {
		self.key.sign(data)
	}

	fn verify(&self, signature: &[u8], data: &[u8], public_key: Option<&[u8]>) -> bool {
		// if key is provided verifgy its our key
		if let Some(key) = public_key {
			if key != &self.key.private_key_bytes() {
				return false
			}
		}

		// verify signature
		self.key.verify(data, signature).is_ok()
	}
}
