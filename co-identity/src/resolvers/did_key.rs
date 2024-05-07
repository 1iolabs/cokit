use crate::{
	DidCommPrivateContext, DidCommPublicContext, Identity, IdentityResolver, IdentityResolverError, PrivateIdentity,
	SignError,
};
use anyhow::anyhow;
use async_trait::async_trait;
use co_primitives::{tags, Secret};
use did_key::{
	from_existing_key, generate, resolve, CoreSign, DIDCore, Ed25519KeyPair, KeyMaterial, PatchedKeyPair, X25519KeyPair,
};
use std::{fmt::Debug, sync::Arc};

#[derive(Clone)]
pub struct DidKeyIdentity {
	did: String,
	key: Arc<PatchedKeyPair>,
	private: bool,
}
impl DidKeyIdentity {
	/// Generate new identity.
	///
	/// # Arguments
	/// - `seed` - The seed usedt to genreate the identity. If `None` is passed it will be generated using `getrandom`
	///   crate.
	pub fn generate(seed: Option<&[u8]>) -> Self {
		Self::from_key(generate::<Ed25519KeyPair>(seed))
	}
	pub fn generate_x25519(seed: Option<&[u8]>) -> Self {
		Self::from_key(generate::<X25519KeyPair>(seed))
	}

	pub fn from_identity(identity: &str) -> Result<Self, anyhow::Error> {
		Self::try_from(identity)
	}

	pub fn from_key(key: PatchedKeyPair) -> Self {
		let private = !key.private_key_bytes().is_empty();
		Self { did: key.get_did_document(Default::default()).id, key: Arc::new(key), private }
	}

	pub fn from_bytes(bytes: &[u8]) -> Result<Self, anyhow::Error> {
		Self::try_from(bytes)
	}

	pub fn to_bytes(&self) -> &[u8] {
		self.identity().as_bytes()
	}

	// pub fn key(&self) -> Arc<PatchedKeyPair> {
	// 	self.key.clone()
	// }

	pub fn public_key_bytes(&self) -> Vec<u8> {
		self.key.as_ref().public_key_bytes()
	}

	pub fn private_key_bytes(&self) -> Secret {
		self.key.as_ref().private_key_bytes().into()
	}

	pub fn import(key: &co_core_keystore::Key) -> Result<Self, anyhow::Error> {
		match (key.tags.string("format"), &key.secret) {
			(Some("Ed25519"), co_core_keystore::Secret::PrivateKey(secret)) =>
				Ok(Self::from_key(from_existing_key::<Ed25519KeyPair>(&[], Some(secret.divulge())))),
			(Some("X25519"), co_core_keystore::Secret::PrivateKey(secret)) =>
				Ok(Self::from_key(from_existing_key::<X25519KeyPair>(&[], Some(secret.divulge())))),
			_ => Err(anyhow!("Invalid identity format or key")),
		}
	}

	pub fn export(&self) -> Result<co_core_keystore::Key, anyhow::Error> {
		Ok(co_core_keystore::Key {
			description: "did:key identitiy".to_owned(),
			name: self.identity().to_owned(),
			tags: tags!("type": "co-identity", "format": "Ed25519"), // TODO: detect format alg
			uri: self.identity().to_owned(),
			secret: co_core_keystore::Secret::PrivateKey(self.key.private_key_bytes().into()),
		})
	}
}
impl Debug for DidKeyIdentity {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("DidKeyIdentity")
			.field("did", &self.did)
			.field("public_key", &format_args!("{:02X?}", self.key.public_key_bytes()))
			.finish()
	}
}
impl TryFrom<&[u8]> for DidKeyIdentity {
	type Error = anyhow::Error;

	fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
		Ok(Self::from_key(resolve(std::str::from_utf8(value)?).map_err(|e| anyhow!("resolve failed: {:?}", e))?))
	}
}
impl TryFrom<&str> for DidKeyIdentity {
	type Error = anyhow::Error;

	fn try_from(value: &str) -> Result<Self, Self::Error> {
		Ok(Self::from_key(resolve(value).map_err(|e| anyhow!("resolve failed: {:?}", e))?))
	}
}
impl Identity for DidKeyIdentity {
	fn identity(&self) -> &str {
		&self.did
	}

	fn public_key(&self) -> Option<Vec<u8>> {
		// Some(self.key.public_key_bytes())
		None
	}

	fn verify(&self, signature: &[u8], data: &[u8], public_key: Option<&[u8]>) -> bool {
		// if key is provided verify its our key
		if let Some(key) = public_key {
			if key != &self.key.public_key_bytes() {
				return false
			}
		}

		// verify signature
		self.key.verify(data, signature).is_ok()
	}

	fn didcomm_public(&self) -> Option<DidCommPublicContext> {
		Some(DidCommPublicContext::new(self.identity().to_owned(), self.key.public_key_bytes()))
	}
}
impl PrivateIdentity for DidKeyIdentity {
	fn sign(&self, data: &[u8]) -> Result<Vec<u8>, SignError> {
		if !self.private {
			return Err(SignError::Unauthorized);
		}
		Ok(self.key.sign(data))
	}

	fn didcomm_private(&self) -> Option<DidCommPrivateContext> {
		Some(DidCommPrivateContext::new(self.identity().to_owned(), self.key.private_key_bytes().into()))
	}
}

pub struct DidKeyIdentityResolver {}
impl DidKeyIdentityResolver {
	pub fn new() -> DidKeyIdentityResolver {
		Self {}
	}
}
#[async_trait]
impl IdentityResolver for DidKeyIdentityResolver {
	async fn resolve(
		&self,
		identity: &str,
		public_key: Option<&[u8]>,
	) -> Result<Box<dyn Identity + Send + Sync>, IdentityResolverError> {
		if identity.starts_with("did:key:") {
			if let Ok(did_key_identity) = DidKeyIdentity::try_from(identity) {
				if match (public_key, did_key_identity.public_key()) {
					(Some(a), Some(b)) => a == b,
					_ => true,
				} {
					return Ok(Box::new(did_key_identity));
				}
			}
		}
		Err(IdentityResolverError::NotFound)
	}
}

#[cfg(test)]
mod tests {
	use crate::{DidKeyIdentity, Identity, PrivateIdentity};

	#[test]
	fn it_should_sign_and_verfiy() {
		let data = "hello world".as_bytes();
		let identity = DidKeyIdentity::generate(None);
		let signature = identity.sign(data).unwrap();
		assert!(identity.verify(signature.as_slice(), data, None));
	}

	#[test]
	fn it_should_sign_and_verfiy_with_public_key() {
		let data = "hello world".as_bytes();
		let identity = DidKeyIdentity::generate(None);
		let public_key = identity.public_key();
		let signature = identity.sign(data).unwrap();
		assert!(identity.verify(signature.as_slice(), data, public_key.as_ref().map(|k| k.as_slice())));
	}
}
