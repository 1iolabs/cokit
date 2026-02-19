// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::{Jwk, VerificationMethod};

pub fn from_did_key_verification_method(
	vm: did_key::VerificationMethod,
	key: Option<did_key::KeyFormat>,
) -> VerificationMethod {
	let mut result = VerificationMethod {
		id: vm.id,
		method_type: vm.key_type.into(),
		controller: vm.controller,
		public_key_jwk: None,
		public_key_multibase: None,
		public_key_base58: None,
		public_key_hex: None,
	};
	if let Some(key) = key.or(vm.public_key) {
		match key {
			did_key::KeyFormat::Base58(v) => {
				result.public_key_base58 = Some(v);
			},
			did_key::KeyFormat::Multibase(v) => {
				result.public_key_multibase = std::str::from_utf8(&v).ok().map(|f| f.to_owned());
			},
			did_key::KeyFormat::JWK(v) => {
				result.public_key_jwk =
					Some(Jwk { kid: v.key_id, kty: v.key_type, crv: v.curve, x: v.x, y: v.y, d: v.d });
			},
		}
	}
	result
}
