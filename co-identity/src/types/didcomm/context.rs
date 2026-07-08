// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::{
	library::{
		didcomm_anoncrypt::{didcomm_anoncrypt_receive, didcomm_anoncrypt_to_public_key},
		didcomm_jwe::{didcomm_jwe, didcomm_jwe_receive},
		didcomm_jws::didcomm_jws,
		didcomm_receive::didcomm_receive,
	},
	DidCommHeader, IdentityResolver, ReceiveError, SignError, VerificationMethod,
};
use co_primitives::{Did, Secret};

pub trait DidCommContext {
	fn did(&self) -> &Did;
	fn key_agreement(&self) -> &VerificationMethod;
	fn verification_method(&self) -> &VerificationMethod;
}

pub struct DidCommPublicContext {
	did: Did,
	verification_method: VerificationMethod,
	key_agreement: VerificationMethod,
}
impl DidCommPublicContext {
	pub fn new(did: Did, verification_method: VerificationMethod, key_agreement: VerificationMethod) -> Self {
		Self { did, verification_method, key_agreement }
	}
}
impl DidCommPublicContext {
	/// Create an anoncrypt JWE envelope to this public DIDComm context.
	///
	/// No sender private key is required or used. The plaintext `from` header is removed during
	/// packing so the recipient cannot recover the real sender identity from this envelope.
	pub fn anoncrypt(&self, header: DidCommHeader, body: Option<&str>) -> Result<String, SignError> {
		if !header.to.contains(&self.did) {
			return Err(SignError::InvalidArgument(anyhow::anyhow!("header must contain recipent: to: {}", self.did)));
		}
		didcomm_anoncrypt_to_public_key(
			self.key_agreement.public_key_bytes().map_err(SignError::InvalidArgument)?,
			header,
			body,
		)
	}
}
impl DidCommContext for DidCommPublicContext {
	fn did(&self) -> &Did {
		&self.did
	}

	fn key_agreement(&self) -> &VerificationMethod {
		&self.key_agreement
	}

	fn verification_method(&self) -> &VerificationMethod {
		&self.verification_method
	}
}

pub struct DidCommPrivateContext {
	public: DidCommPublicContext,
	verification_method_private_key: Secret,
	key_agreement_private_key: Secret,
}
impl DidCommPrivateContext {
	pub fn new(
		public: DidCommPublicContext,
		verification_method_private_key: Secret,
		key_agreement_private_key: Secret,
	) -> Self {
		Self { public, verification_method_private_key, key_agreement_private_key }
	}

	/// Create JWS message envelope.
	///
	/// # DID Comm
	/// - Envelope: `signed(plaintext)`
	/// - Media Type: `application/didcomm-signed+json`
	///
	/// # Arguments
	/// - `body` - JSON String.
	pub fn jws(&self, header: DidCommHeader, body: &str) -> Result<String, SignError> {
		didcomm_jws(
			self.verification_method_private_key.clone(),
			&self
				.verification_method()
				.public_key_bytes()
				.map_err(SignError::InvalidArgument)?,
			header,
			body,
		)
	}

	/// Create JWE message envelope.
	///
	/// # DID Comm
	/// - Envelope: `authcrypt(plaintext)`
	/// - Media Type: `application/didcomm-encrypted+json`
	///
	/// # Arguments
	/// - `body` - JSON String.
	pub fn jwe(&self, to: &DidCommPublicContext, header: DidCommHeader, body: &str) -> Result<String, SignError> {
		if !header.to.contains(&to.did) {
			return Err(SignError::InvalidArgument(anyhow::anyhow!("header must contain recipent: to: {}", to.did)));
		}
		didcomm_jwe(
			self.key_agreement_private_key.clone(),
			to.key_agreement.public_key_bytes().map_err(SignError::InvalidArgument)?,
			header,
			body,
		)
	}

	/// Deprecated compatibility convenience for anoncrypt packing.
	///
	/// Prefer `DidCommPublicContext::anoncrypt`; anoncrypt does not need this private sender context.
	#[deprecated(note = "use DidCommPublicContext::anoncrypt; anoncrypt does not need a sender private key")]
	pub fn anoncrypt(
		&self,
		to: &DidCommPublicContext,
		header: DidCommHeader,
		body: Option<&str>,
	) -> Result<String, SignError> {
		to.anoncrypt(header, body)
	}

	pub fn anoncrypt_receive(&self, incoming: &str) -> Result<(DidCommHeader, Option<String>), ReceiveError> {
		didcomm_anoncrypt_receive(self.key_agreement_private_key.clone(), incoming)
	}

	pub async fn jwe_receive<R: IdentityResolver>(
		&self,
		resolver: &R,
		incoming: &str,
	) -> Result<(DidCommHeader, String), ReceiveError> {
		didcomm_jwe_receive(self.key_agreement_private_key.clone(), resolver, incoming).await
	}

	pub async fn receive<R: IdentityResolver>(
		&self,
		resolver: &R,
		incoming: &str,
	) -> Result<(DidCommHeader, String), ReceiveError> {
		didcomm_receive(Some(self.key_agreement_private_key.clone()), resolver, incoming).await
	}
}
impl DidCommContext for DidCommPrivateContext {
	fn did(&self) -> &Did {
		self.public.did()
	}

	fn key_agreement(&self) -> &VerificationMethod {
		self.public.key_agreement()
	}

	fn verification_method(&self) -> &VerificationMethod {
		self.public.verification_method()
	}
}

#[cfg(test)]
mod tests {
	use crate::{DidKeyIdentity, Identity, PrivateIdentity};

	#[test]
	fn anoncrypt_can_be_packed_from_recipient_public_context_without_sender_private_key() {
		let to = DidKeyIdentity::generate(Some(&[21; 32]));
		let recipient = to.try_didcomm_public().unwrap();

		let header = crate::DidCommHeader {
			id: "recipient-public-anoncrypt".to_owned(),
			to: vec![to.identity().to_owned()].into_iter().collect(),
			message_type: "test".to_owned(),
			..Default::default()
		};

		let sealed = recipient.anoncrypt(header, Some("\"payload\"")).unwrap();
		let (received_header, body) = to.try_didcomm_private().unwrap().anoncrypt_receive(&sealed).unwrap();

		assert_eq!(received_header.id, "recipient-public-anoncrypt");
		assert_eq!(body.as_deref(), Some("\"payload\""));
	}
}
