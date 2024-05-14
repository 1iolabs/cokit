use super::{didcomm_jwe::didcomm_jwe_receive, into_didcomm_rs_header::from_didcomm_rs_header};
use crate::{DidCommHeader, IdentityResolver, ReceiveError};
use co_primitives::Secret;
use didcomm_rs::Message;

pub async fn didcomm_receive<R: IdentityResolver>(
	to_private_key: Secret,
	resolver: &R,
	incoming: &str,
) -> Result<(DidCommHeader, String), ReceiveError> {
	// try receive
	let message = match Message::receive(incoming, Some(to_private_key.divulge()), None, None) {
		Ok(message) => message,
		Err(didcomm_rs::Error::DidResolveFailed) => {
			// when the message is encrypted we need to resolve the encryptor did so we just forwad this to the special
			// method
			return didcomm_jwe_receive(to_private_key, resolver, incoming).await;
		},
		Err(err) => return Err(ReceiveError::Decrypt(err.into())),
	};

	// result
	Ok((
		from_didcomm_rs_header(message.get_didcomm_header().clone()),
		message.get_body().map_err(|e| ReceiveError::InvalidArgument(e.into()))?,
	))
}

#[cfg(test)]
mod tests {
	use crate::{
		library::{didcomm_jwe::didcomm_jwe, didcomm_jws::didcomm_jws, didcomm_receive::didcomm_receive},
		DidCommHeader, DidKeyIdentity, DidKeyIdentityResolver, Identity,
	};

	#[tokio::test]
	async fn jwe() {
		// create x25519 identities (protocol)
		let from = DidKeyIdentity::generate_x25519(Some(&vec![1; 32]));
		let to = DidKeyIdentity::generate_x25519(Some(&vec![2; 32]));

		// create
		let header = DidCommHeader {
			id: "test".to_owned(),
			from: Some(from.identity().to_owned()),
			to: vec![to.identity().to_owned()].into_iter().collect(),
			message_type: "test".to_owned(),
			..Default::default()
		};
		let message = didcomm_jwe(from.private_key_bytes(), to.public_key_bytes(), header, "null").unwrap();

		// receive
		let (receviced_header, receviced_body) =
			didcomm_receive(to.private_key_bytes(), &DidKeyIdentityResolver::new(), &message)
				.await
				.unwrap();
		assert_eq!("test", receviced_header.id);
		assert_eq!("null", receviced_body);
	}

	#[tokio::test]
	async fn jws() {
		// create Ed25519 identities (curve)
		let from = DidKeyIdentity::generate(Some(&vec![1; 32]));
		let to = DidKeyIdentity::generate(Some(&vec![2; 32]));

		// create
		let header = DidCommHeader {
			id: "test".to_owned(),
			from: Some(from.identity().to_owned()),
			to: vec![to.identity().to_owned()].into_iter().collect(),
			message_type: "test".to_owned(),
			..Default::default()
		};
		let message = didcomm_jws(from.private_key_bytes(), &from.public_key_bytes(), header, "null").unwrap();

		// receive
		let (receviced_header, receviced_body) =
			didcomm_receive(to.private_key_bytes(), &DidKeyIdentityResolver::new(), &message)
				.await
				.unwrap();
		assert_eq!("test", receviced_header.id);
		assert_eq!("null", receviced_body);
	}
}
