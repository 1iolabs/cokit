use super::into_didcomm_rs_header::{from_didcomm_rs_header, into_didcomm_rs_header};
use crate::{DidCommHeader, DidKeyIdentity, Identity, ReceiveError, SignError};
use co_primitives::Secret;
use didcomm_rs::{
	crypto::{CryptoAlgorithm, SignatureAlgorithm},
	Jwe, Message,
};

/// Create a encrypted JWE envelope.
///
/// This follows the recommendation to generate a new one-time signing DID just for this single call.
/// (See didcomm-messaging / message-header / from).
///
/// Envelope: `authcrypt(plaintext)`
/// Media Type: `application/didcomm-encrypted+json`
/// See: https://identity.foundation/didcomm-messaging/spec/#message-headers
pub fn didcomm_jwe(
	from_private_key: Secret,
	to_public_key: Vec<u8>,
	header: DidCommHeader,
	body: &str,
) -> Result<String, SignError> {
	let message = Message::new()
		.didcomm_header(into_didcomm_rs_header(header))
		.body(body)
		.map_err(|e| SignError::Other(e.into()))?;
	let signer = DidKeyIdentity::generate(None);
	// println!("signer: {}", signer.identity());
	let result = message
		//.from(signer.identity())
		.as_flat_jwe(&CryptoAlgorithm::XC20P, Some(to_public_key.clone()))
		.kid(&hex::encode(signer.public_key_bytes()))
		.seal_signed(
			from_private_key.divulge(),
			Some(vec![Some(to_public_key.clone())]),
			SignatureAlgorithm::EdDsa,
			signer.private_key_bytes().divulge(),
		)
		// .seal(from_private_key.divulge(), Some(vec![Some(to_public_key)]))
		.map_err(|e| SignError::Other(e.into()))?;
	Ok(result)
}

pub fn didcomm_jwe_receive(to_private_key: Secret, incoming: &str) -> Result<(DidCommHeader, String), ReceiveError> {
	let jwe: Jwe = serde_json::from_str(&incoming).map_err(|e| ReceiveError::UnknownFormat(e.into()))?;

	// we expect the jwe signed with a one-time key
	let skid = jwe.get_skid().ok_or_else(|| ReceiveError::MissingSigningKeyId)?;
	// println!("skid: {}", skid);

	// we only support did:key: as signing key
	let skid_identity = DidKeyIdentity::from_identity(&skid).map_err(|e| ReceiveError::InvalidSigningKeyId(e))?;

	// try recv
	let message =
		Message::receive(incoming, Some(to_private_key.divulge()), Some(skid_identity.public_key_bytes()), None)
			.map_err(|e| ReceiveError::Decrypt(e.into()))?;

	// result
	Ok((
		from_didcomm_rs_header(message.get_didcomm_header().clone()),
		message.get_body().map_err(|e| ReceiveError::InvalidArgument(e.into()))?,
	))
}

#[cfg(test)]
mod tests {
	use super::{didcomm_jwe, didcomm_jwe_receive};
	use crate::{DidCommHeader, DidKeyIdentity, Identity};

	#[test]
	fn smoke() {
		// let alice_key = did_key::from_existing_key::<did_key::X25519KeyPair>(
		// 	&[],
		// 	Some(
		// 		bs58::decode("6QN8DfuN9hjgHgPvLXqgzqYE3jRRGRrmJQZkd5tL8paR")
		// 			.into_vec()
		// 			.unwrap()
		// 			.as_ref(),
		// 	),
		// );
		// let bob_key = did_key::from_existing_key::<did_key::X25519KeyPair>(
		// 	&[],
		// 	Some(
		// 		bs58::decode("HBTcN2MrXNRj9xF9oi8QqYyuEPv3JLLjQKuEgW9oxVKP")
		// 			.into_vec()
		// 			.unwrap()
		// 			.as_ref(),
		// 	),
		// );
		// println!("bob: {}", bob_key.fingerprint());
		// println!("bob: {:?}", bob_key.private_key_bytes());
		// println!("bob-public: {:?}", bob_key.public_key_bytes());
		// println!("alice: {:?}", alice_key.private_key_bytes());
		// println!("alice-public: {:?}", alice_key.public_key_bytes());
		// let f = did_key::from_existing_key::<did_key::X25519KeyPair>(
		// 	bs58::decode("6MkiTBz1ymuepAQ4HEHYSF1H8quG5GLVVQR3djdX3mDooWp")
		// 		.into_vec()
		// 		.unwrap()
		// 		.as_ref(),
		// 	None,
		// );
		// let f = did_key::resolve("did:key:z6MkiTBz1ymuepAQ4HEHYSF1H8quG5GLVVQR3djdX3mDooWp").unwrap();
		// println!("f: {}", f.fingerprint());
		// println!("f: {:?}", f.public_key_bytes());

		// let from = DidKeyIdentity::from_key(alice_key);
		// let to = DidKeyIdentity::from_key(bob_key);
		let from = DidKeyIdentity::generate_x25519(Some(&vec![1; 32]));
		let to = DidKeyIdentity::generate_x25519(Some(&vec![2; 32]));
		let other = DidKeyIdentity::generate(Some(&vec![3; 32]));
		// println!("from: {}", from.identity());
		// println!("to: {}", to.identity());

		// create
		let header = DidCommHeader {
			id: "test".to_owned(),
			from: Some(from.identity().to_owned()),
			to: vec![to.identity().to_owned()].into_iter().collect(),
			message_type: "test".to_owned(),
			..Default::default()
		};
		let message = didcomm_jwe(from.private_key_bytes(), to.public_key_bytes(), header, "null").unwrap();
		// println!("message({}): {}", message.len(), message); // 2462

		// receive
		let (receviced_header, receviced_body) = didcomm_jwe_receive(to.private_key_bytes(), &message).unwrap();
		assert_eq!("test", receviced_header.id);
		assert_eq!("null", receviced_body);

		// receive other
		let received_other = didcomm_jwe_receive(other.private_key_bytes(), &message);
		assert!(received_other.is_err());
	}
}
