use crate::{
	library::{
		didcomm_jwe::{didcomm_jwe, didcomm_jwe_receive},
		didcomm_jws::didcomm_jws,
		didcomm_receive::didcomm_receive,
	},
	DidCommHeader, IdentityResolver, ReceiveError, SignError,
};
use co_primitives::{Did, Secret};

pub struct DidCommPrivateContext {
	did: Did,
	private_key: Secret,
	public_key: Vec<u8>,
}
impl DidCommPrivateContext {
	pub fn new(did: Did, private_key: Secret, public_key: Vec<u8>) -> Self {
		Self { did, private_key, public_key }
	}

	pub fn did(&self) -> Did {
		self.did.clone()
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
		didcomm_jws(self.private_key.clone(), &self.public_key, header, body)
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
			header.to.insert(to.did());
		}
		// if !header.to.contains(&to.did) {
		// 	return Err(SignError::InvalidArgument(anyhow!("header must contain recipent: {}", to.did)));
		// }
		didcomm_jwe(self.private_key.clone(), to.public_key.clone(), header, body)
	}

	pub async fn jwe_receive<R: IdentityResolver>(
		&self,
		resolver: &R,
		incoming: &str,
	) -> Result<(DidCommHeader, String), ReceiveError> {
		didcomm_jwe_receive(self.private_key.clone(), resolver, incoming).await
	}

	pub async fn receive<R: IdentityResolver>(
		&self,
		resolver: &R,
		incoming: &str,
	) -> Result<(DidCommHeader, String), ReceiveError> {
		didcomm_receive(self.private_key.clone(), resolver, incoming).await
	}
}

pub struct DidCommPublicContext {
	did: Did,
	public_key: Vec<u8>,
}
impl DidCommPublicContext {
	pub fn new(did: Did, public_key: Vec<u8>) -> Self {
		Self { did, public_key }
	}

	pub fn did(&self) -> Did {
		self.did.clone()
	}

	pub fn public_key_bytes(&self) -> Vec<u8> {
		self.public_key.clone()
	}
}
