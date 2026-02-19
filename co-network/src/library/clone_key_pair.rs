// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use did_key::{Generate, KeyMaterial};

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
