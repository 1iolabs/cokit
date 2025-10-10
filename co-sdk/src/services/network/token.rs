use co_network::bitswap::Token;
use co_primitives::{CoId, KnownMultiCodec, MultiCodec, Secret};
use hmac::{Hmac, Mac};
use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use sha2::Sha256;

/// Canonical Token Parameters.
#[derive(Debug, Serialize, Deserialize)]
pub struct CoTokenParameters(pub PeerId, pub CoId);

/// Signed Token.
#[derive(Debug, Serialize, Deserialize)]
pub struct CoToken {
	#[serde(rename = "b")]
	pub body: CoTokenParameters,
	#[serde(rename = "s")]
	pub signature: Vec<u8>,
	#[serde(rename = "a")]
	pub algorithm: String,
}
impl CoToken {
	pub fn new(secret: &Secret, body: CoTokenParameters) -> Result<Self, anyhow::Error> {
		let mut mac = Hmac::<Sha256>::new_from_slice(secret.divulge())?;
		serde_ipld_dagcbor::to_writer(&mut mac, &body)?;
		let result = mac.finalize();
		Ok(Self { body, signature: result.into_bytes().to_vec(), algorithm: "HS256".to_owned() })
	}

	pub fn new_unsigned(body: CoTokenParameters) -> Self {
		Self { body, signature: [].to_vec(), algorithm: "".to_owned() }
	}

	pub fn is_unsigned(&self) -> bool {
		self.signature.is_empty()
	}

	/// Verify token.
	/// If local_peer is supplied we also allow our token.
	pub fn verify(&self, secret: &Secret, remote_peer: &PeerId, local_peer: Option<&PeerId>) -> bool {
		// speedup: fail immediately if the token peer is not the remote peer
		let token_peer = if &self.body.0 == remote_peer {
			remote_peer
		} else if local_peer == Some(&self.body.0) {
			local_peer.unwrap()
		} else {
			tracing::trace!(?remote_peer, ?local_peer, token_peer = ?self.body.0, "bitswap-token-peer-invalid");
			return false;
		};
		match self.algorithm.as_str() {
			"HS256" => Self::new(secret, CoTokenParameters(*token_peer, self.body.1.clone()))
				.map(|token| &token.signature == &self.signature)
				.unwrap_or(false),
			_ => false,
		}
	}

	pub fn to_bytes(&self) -> Result<Vec<u8>, anyhow::Error> {
		Ok(serde_ipld_dagcbor::to_vec(&self)?)
	}

	pub fn from_bytes(bytes: &[u8]) -> Result<Self, anyhow::Error> {
		Ok(serde_ipld_dagcbor::from_slice(bytes)?)
	}

	pub fn to_bitswap_token(&self) -> Result<Token, anyhow::Error> {
		Ok(Token(KnownMultiCodec::DagCbor.into(), self.to_bytes()?))
	}

	pub fn from_bitswap_token(token: &Token) -> Result<Self, anyhow::Error> {
		match MultiCodec::from(token.0) {
			MultiCodec::Known(KnownMultiCodec::DagCbor) => Ok(Self::from_bytes(&token.1)?),
			_ => Err(anyhow::anyhow!("Unsupported token multicode")),
		}
	}
}

#[cfg(test)]
mod tests {
	use crate::CoToken;
	use co_primitives::Secret;
	use libp2p::PeerId;

	#[test]
	fn smoke() {
		let peer = PeerId::random();
		let secret: Secret = co_storage::Secret::generate(32).into();
		let token = CoToken::new(&secret, crate::CoTokenParameters(peer, "test".into())).unwrap();
		assert!(token.verify(&secret, &peer, None));
	}
}
