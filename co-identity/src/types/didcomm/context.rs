use crate::{
	library::{
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
		let mut header = header;
		if !header.to.contains(&to.did) {
			header.to.insert(to.did().to_owned());
		}
		// if !header.to.contains(&to.did) {
		// 	return Err(SignError::InvalidArgument(anyhow!("header must contain recipent: {}", to.did)));
		// }
		didcomm_jwe(
			self.key_agreement_private_key.clone(),
			to.key_agreement
				.public_key_bytes()
				.map_err(SignError::InvalidArgument)?,
			header,
			body,
		)
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
