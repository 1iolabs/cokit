use crate::{
	library::{
		didcomm_jwe::{didcomm_jwe, didcomm_jwe_receive},
		didcomm_jws::didcomm_jws,
	},
	DidCommHeader, ReceiveError, SignError,
};
use co_primitives::{Did, Secret};

pub struct DidCommPrivateContext {
	did: Did,
	private_key: Secret,
}
impl DidCommPrivateContext {
	pub fn new(did: Did, private_key: Secret) -> Self {
		Self { did, private_key }
	}

	pub fn did(&self) -> Did {
		self.did.clone()
	}

	pub fn jws(&self, body: &str) -> Result<String, SignError> {
		didcomm_jws(self.private_key.clone(), body)
	}

	pub fn jwe(&self, to: &DidCommPublicContext, header: DidCommHeader, body: &str) -> Result<String, SignError> {
		if header.to.contains(&to.did) {
			return Err(SignError::InvalidArgument);
		}
		didcomm_jwe(self.private_key.clone(), to.public_key.clone(), header, body)
	}

	pub fn jwe_receive(&self, incoming: &str) -> Result<(DidCommHeader, String), ReceiveError> {
		didcomm_jwe_receive(self.private_key.clone(), incoming)
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
}
