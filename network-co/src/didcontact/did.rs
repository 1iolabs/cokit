use did_key::{Fingerprint, Generate, KeyMaterial};
use did_url::DID;
use uuid::Uuid;

#[derive(Debug, Clone, thiserror::Error)]
pub enum ResolveError {
	/// Invalid URI format.
	#[error("Invalid URI format.")]
	InvalidUri,

	/// The method has not been implemented.
	#[error("The method has not been implemented.")]
	UnsupportedMethod,

	/// The method reported an error while resolving the DID Document.
	#[error("The method reported an error while resolving the DID Document.")]
	Resolve,
}

pub enum ResolveResult {
	Key(did_key::PatchedKeyPair),
}

/// Resolve `did` rendenzvoud point string, if one.
pub async fn resolve(did: &str) -> Result<ResolveResult, ResolveError> {
	let uri = DID::parse(did).map_err(|_e| ResolveError::InvalidUri)?;
	match uri.method() {
		"key" => {
			let result = did_key::resolve(did).map_err(|e| {
				// log
				tracing::warn!(err = ?e, "did-resolve-failed");

				// err
				ResolveError::Resolve
			})?;
			Ok(ResolveResult::Key(result))
		},
		_ => Err(ResolveError::UnsupportedMethod),
	}
}

pub fn clone_key_pair(value: &did_key::KeyPair) -> did_key::KeyPair {
	match value {
		did_key::KeyPair::Ed25519(key) => {
			did_key::KeyPair::Ed25519(did_key::Ed25519KeyPair::from_secret_key(key.private_key_bytes().as_slice()))
		},
		did_key::KeyPair::X25519(key) => {
			did_key::KeyPair::X25519(did_key::X25519KeyPair::from_secret_key(key.private_key_bytes().as_slice()))
		},
		did_key::KeyPair::P256(key) => {
			did_key::KeyPair::P256(did_key::P256KeyPair::from_secret_key(key.private_key_bytes().as_slice()))
		},
		did_key::KeyPair::Bls12381G1G2(key) => did_key::KeyPair::Bls12381G1G2(
			did_key::Bls12381KeyPairs::from_secret_key(key.private_key_bytes().as_slice()),
		),
		did_key::KeyPair::Secp256k1(key) => {
			did_key::KeyPair::Secp256k1(did_key::Secp256k1KeyPair::from_secret_key(key.private_key_bytes().as_slice()))
		},
	}
}
