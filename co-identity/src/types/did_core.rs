// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use anyhow::anyhow;
use serde::{Deserialize, Serialize};

/// Verification Method.
/// See: https://www.w3.org/TR/did-core/#verification-methods
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct VerificationMethod {
	/// The value of the id property for a verification method MUST be a string that conforms to the rules in Section
	/// 3.2 DID URL Syntax.
	pub id: String,

	/// The value of the type property MUST be a string that references exactly one verification method type. In order
	/// to maximize global interoperability, the verification method type SHOULD be registered in the DID Specification
	/// Registries [DID-SPEC-REGISTRIES].
	#[serde(rename = "type")]
	pub method_type: VerificationMethodTypes,

	/// The value of the controller property MUST be a string that conforms to the rules in 3.1 DID Syntax.
	pub controller: String,

	/// The publicKeyJwk property is OPTIONAL. If present, the value MUST be a map representing a JSON Web Key that
	/// conforms to [RFC7517]. The map MUST NOT contain "d", or any other members of the private information class as
	/// described in Registration Template. It is RECOMMENDED that verification methods that use JWKs [RFC7517] to
	/// represent their public keys use the value of kid as their fragment identifier. It is RECOMMENDED that JWK kid
	/// values are set to the public key fingerprint [RFC7638]. See the first key in Example 13 for an example of a
	/// public key with a compound key identifier.
	#[serde(alias = "publicKeyJwk")]
	pub public_key_jwk: Option<Jwk>,

	/// The publicKeyMultibase property is OPTIONAL. This feature is non-normative. If present, the value MUST be a
	/// string representation of a [MULTIBASE] encoded public key.
	#[serde(alias = "publicKeyMultibase")]
	pub public_key_multibase: Option<String>,

	/// This property is deprecated in favor of publicKeyMultibase or publicKeyJwk. It's generally expected that this
	/// term will still be used in older suites and therefore needs be supported for legacy compatibility, but is
	/// expected to not be used for newly defined suites.
	#[serde(rename = "publicKeyBase58")]
	pub public_key_base58: Option<String>,

	/// This property is deprecated in favor of publicKeyMultibase or publicKeyJwk. It's generally expected that this
	/// term will still be used in older suites and therefore needs be supported for legacy compatibility, but is
	/// expected to not be used for newly defined suites.
	#[serde(rename = "publicKeyHex")]
	pub public_key_hex: Option<String>,
}
impl VerificationMethod {
	pub fn public_key_bytes(&self) -> Result<Vec<u8>, anyhow::Error> {
		if let Some(jwk) = &self.public_key_jwk {
			match jwk.kty.as_str() {
				// See: https://datatracker.ietf.org/doc/html/draft-ietf-jose-cfrg-curves-06#section-2
				"OKP" => {
					if let Some(public_key) = &jwk.x {
						Ok(multibase::Base::Base64Url.decode(public_key)?)
					} else if let Some(_private_key) = &jwk.d {
						// Ok(multibase::Base::Base64Url.decode(private_key)?)
						Err(anyhow!("Unsupported JWK: no public key found."))
					} else {
						Err(anyhow!("Unsupported JWK: no keys found."))
					}
				},
				other => Err(anyhow!("Unsupported JWK: kty: {}", other)),
			}
		} else if let Some(s) = &self.public_key_multibase {
			Ok(multibase::decode(s)?.1)
		} else if let Some(s) = &self.public_key_base58 {
			Ok(multibase::Base::Base58Btc.decode(s)?)
		} else if let Some(s) = &self.public_key_hex {
			Ok((0..s.len())
				.step_by(2)
				.map(|i| u8::from_str_radix(&s[i..i + 2], 16))
				.collect::<Result<Vec<_>, _>>()?)
		} else {
			Err(anyhow!("No public key."))
		}
	}
}

/// JSON Web Key (JWK)
///
/// See:
/// - https://datatracker.ietf.org/doc/html/rfc7517
/// - https://www.rfc-editor.org/rfc/rfc7638
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Default)]
pub struct Jwk {
	/// The "kid" (key ID) parameter is used to match a specific key.  This
	/// is used, for instance, to choose among a set of keys within a JWK Set
	/// during key rollover.  The structure of the "kid" value is
	/// unspecified.  When "kid" values are used within a JWK Set, different
	/// keys within the JWK Set SHOULD use distinct "kid" values.  (One
	/// example in which different keys might use the same "kid" value is if
	/// they have different "kty" (key type) values but are considered to be
	/// equivalent alternatives by the application using them.)  The "kid"
	/// value is a case-sensitive string.  Use of this member is OPTIONAL.
	/// When used with JWS or JWE, the "kid" value is used to match a JWS or
	/// JWE "kid" Header Parameter value.
	///
	/// See: https://datatracker.ietf.org/doc/html/rfc7517#section-4.5
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub kid: Option<String>,

	/// The "kty" (key type) parameter identifies the cryptographic algorithm
	/// family used with the key, such as "RSA" or "EC".  "kty" values should
	/// either be registered in the IANA "JSON Web Key Types" registry
	/// established by [JWA] or be a value that contains a Collision-
	/// Resistant Name.  The "kty" value is a case-sensitive string.  This
	/// member MUST be present in a JWK.
	///
	/// A list of defined "kty" values can be found in the IANA "JSON Web Key
	/// Types" registry established by [JWA]; the initial contents of this
	/// registry are the values defined in Section 6.1 of [JWA].
	///
	/// The key type definitions include specification of the members to be
	/// used for those key types.  Members used with specific "kty" values
	/// can be found in the IANA "JSON Web Key Parameters" registry
	/// established by Section 8.1.
	///
	/// See: https://datatracker.ietf.org/doc/html/rfc7517#section-4.1
	pub kty: String,

	/// Curve.
	pub crv: String,

	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub x: Option<String>,

	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub y: Option<String>,

	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub d: Option<String>,
}

/// These are values to be used for the type in a verification method object.
/// See: https://www.w3.org/TR/did-spec-registries/#verification-method-types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(into = "String", from = "String")]
#[non_exhaustive]
pub enum VerificationMethodTypes {
	JsonWebKey2020,
	EcdsaSecp256k1VerificationKey2019,
	Ed25519VerificationKey2018,
	Bls12381G1Key2020,
	Bls12381G2Key2020,
	PgpVerificationKey2021,
	RsaVerificationKey2018,
	X25519KeyAgreementKey2019,
	EcdsaSecp256k1RecoveryMethod2020,
	VerifiableCondition2021,
	Other(String),
}
impl AsRef<str> for VerificationMethodTypes {
	fn as_ref(&self) -> &str {
		match self {
			VerificationMethodTypes::JsonWebKey2020 => "JsonWebKey2020",
			VerificationMethodTypes::EcdsaSecp256k1VerificationKey2019 => "EcdsaSecp256k1VerificationKey2019",
			VerificationMethodTypes::Ed25519VerificationKey2018 => "Ed25519VerificationKey2018",
			VerificationMethodTypes::Bls12381G1Key2020 => "Bls12381G1Key2020",
			VerificationMethodTypes::Bls12381G2Key2020 => "Bls12381G2Key2020",
			VerificationMethodTypes::PgpVerificationKey2021 => "PgpVerificationKey2021",
			VerificationMethodTypes::RsaVerificationKey2018 => "RsaVerificationKey2018",
			VerificationMethodTypes::X25519KeyAgreementKey2019 => "X25519KeyAgreementKey2019",
			VerificationMethodTypes::EcdsaSecp256k1RecoveryMethod2020 => "EcdsaSecp256k1RecoveryMethod2020",
			VerificationMethodTypes::VerifiableCondition2021 => "VerifiableCondition2021",
			VerificationMethodTypes::Other(s) => s.as_str(),
		}
	}
}
impl From<VerificationMethodTypes> for String {
	fn from(val: VerificationMethodTypes) -> Self {
		val.as_ref().to_owned()
	}
}
impl From<String> for VerificationMethodTypes {
	fn from(value: String) -> Self {
		VerificationMethodTypes::from(value.as_str())
	}
}
impl From<&str> for VerificationMethodTypes {
	fn from(value: &str) -> Self {
		match value {
			"JsonWebKey2020" => VerificationMethodTypes::JsonWebKey2020,
			"EcdsaSecp256k1VerificationKey2019" => VerificationMethodTypes::EcdsaSecp256k1VerificationKey2019,
			"Ed25519VerificationKey2018" => VerificationMethodTypes::Ed25519VerificationKey2018,
			"Bls12381G1Key2020" => VerificationMethodTypes::Bls12381G1Key2020,
			"Bls12381G2Key2020" => VerificationMethodTypes::Bls12381G2Key2020,
			"PgpVerificationKey2021" => VerificationMethodTypes::PgpVerificationKey2021,
			"RsaVerificationKey2018" => VerificationMethodTypes::RsaVerificationKey2018,
			"X25519KeyAgreementKey2019" => VerificationMethodTypes::X25519KeyAgreementKey2019,
			"EcdsaSecp256k1RecoveryMethod2020" => VerificationMethodTypes::EcdsaSecp256k1RecoveryMethod2020,
			"VerifiableCondition2021" => VerificationMethodTypes::VerifiableCondition2021,
			other => VerificationMethodTypes::Other(other.to_string()),
		}
	}
}
