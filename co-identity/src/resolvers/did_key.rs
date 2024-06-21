use crate::{
	library::from_did_key_verification_method::from_did_key_verification_method,
	types::didcomm::context::DidCommContext, DidCommPrivateContext, DidCommPublicContext, Identity, IdentityBox,
	IdentityResolver, IdentityResolverError, PrivateIdentity, SignError,
};
use anyhow::anyhow;
use async_trait::async_trait;
use co_primitives::{tags, Network, Secret};
use did_key::{
	from_existing_key, generate, resolve, CoreSign, DIDCore, Ed25519KeyPair, KeyMaterial, PatchedKeyPair, X25519KeyPair,
};
use std::{collections::BTreeSet, fmt::Debug, sync::Arc};

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
			(Some("Ed25519"), co_core_keystore::Secret::PrivateKey(secret)) => {
				Ok(Self::from_key(from_existing_key::<Ed25519KeyPair>(&[], Some(secret.divulge()))))
			},
			(Some("X25519"), co_core_keystore::Secret::PrivateKey(secret)) => {
				Ok(Self::from_key(from_existing_key::<X25519KeyPair>(&[], Some(secret.divulge()))))
			},
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
				return false;
			}
		}

		// verify signature
		self.key.verify(data, signature).is_ok()
	}

	fn didcomm_public(&self) -> Option<DidCommPublicContext> {
		let doc = self
			.key
			.get_did_document(did_key::Config { use_jose_format: false, serialize_secrets: false });
		let verfication_method = from_did_key_verification_method(doc.verification_method.first()?.clone(), None);
		let key_aggrements = doc.key_agreement?;
		let key_aggrement_id = key_aggrements.first()?;
		let key_agreement = from_did_key_verification_method(
			doc.verification_method
				.iter()
				.find(|item| &item.id == key_aggrement_id)?
				.clone(),
			None,
		);
		Some(DidCommPublicContext::new(self.identity().to_owned(), verfication_method, key_agreement))
	}

	fn networks(&self) -> BTreeSet<Network> {
		Default::default()
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
		let public = self.didcomm_public()?;
		let doc = self
			.key
			.get_did_document(did_key::Config { use_jose_format: false, serialize_secrets: true });
		let verfication_method_private = doc.verification_method.iter().find_map(|vm| {
			if vm.id == public.verification_method().id && vm.private_key.is_some() {
				from_did_key_verification_method(vm.clone(), vm.private_key.clone())
					.public_key_bytes()
					.ok()
			} else {
				None
			}
		})?;
		let key_agreement_private = doc.verification_method.iter().find_map(|vm| {
			if vm.id == public.key_agreement().id && vm.private_key.is_some() {
				from_did_key_verification_method(vm.clone(), vm.private_key.clone())
					.public_key_bytes()
					.ok()
			} else {
				None
			}
		})?;
		Some(DidCommPrivateContext::new(public, verfication_method_private.into(), key_agreement_private.into()))
	}
}

#[derive(Debug, Clone)]
pub struct DidKeyIdentityResolver {}
impl Default for DidKeyIdentityResolver {
	fn default() -> Self {
		Self::new()
	}
}
impl DidKeyIdentityResolver {
	pub fn new() -> DidKeyIdentityResolver {
		Self {}
	}
}
#[async_trait]
impl IdentityResolver for DidKeyIdentityResolver {
	async fn resolve(&self, identity: &str) -> Result<IdentityBox, IdentityResolverError> {
		if identity.starts_with("did:key:") {
			if let Ok(did_key_identity) = DidKeyIdentity::try_from(identity) {
				return Ok(IdentityBox::new(did_key_identity));
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
		assert!(identity.verify(signature.as_slice(), data, public_key.as_deref()));
	}
}
