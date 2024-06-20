use super::{didcomm_jwe::didcomm_jwe_receive, into_didcomm_rs_header::from_didcomm_rs_header};
use crate::{DidCommHeader, IdentityResolver, ReceiveError};
use co_primitives::Secret;
use didcomm_rs::Message;

pub async fn didcomm_receive<R: IdentityResolver>(
	to_key_agreement_private_key: Option<Secret>,
	resolver: &R,
	incoming: &str,
) -> Result<(DidCommHeader, String), ReceiveError> {
	// try receive
	let message =
		match Message::receive(incoming, to_key_agreement_private_key.as_ref().map(|key| key.divulge()), None, None) {
			Ok(message) => message,
			Err(didcomm_rs::Error::DidResolveFailed) => {
				// when the message is encrypted we need to resolve the encryptor did so we just forwad this to the
				// special method
				return didcomm_jwe_receive(
					to_key_agreement_private_key
						.ok_or(ReceiveError::InvalidArgument(anyhow::anyhow!("No private key")))?,
					resolver,
					incoming,
				)
				.await;
			},
			Err(err) => return Err(ReceiveError::UnknownFormat(err.into())),
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
		let from = DidKeyIdentity::generate_x25519(Some(&[1; 32]));
		let to = DidKeyIdentity::generate_x25519(Some(&[2; 32]));

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
			didcomm_receive(Some(to.private_key_bytes()), &DidKeyIdentityResolver::new(), &message)
				.await
				.unwrap();
		assert_eq!("test", receviced_header.id);
		assert_eq!("null", receviced_body);
	}

	#[tokio::test]
	async fn jws() {
		// create Ed25519 identities (curve)
		let from = DidKeyIdentity::generate(Some(&[1; 32]));
		let to = DidKeyIdentity::generate(Some(&[2; 32]));

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
			didcomm_receive(Some(to.private_key_bytes()), &DidKeyIdentityResolver::new(), &message)
				.await
				.unwrap();
		assert_eq!("test", receviced_header.id);
		assert_eq!("null", receviced_body);
	}

	#[test]
	fn test_parse_didcomm_rs_message_without_to_field() {
		// {"typ":"application/didcomm-plain+json","id":"a66d96f6-4a1f-45dc-842d-b0b9b6096cd5","type":"co-heads/1.0.0","
		// from":null,"created_time":1716995635,"expires_time":1716995755,"body":{"h":["test",[{"/":"
		// bafyr4ieq523r6dklo2ff2re6gabvx2377tgxluri4wzy5du4x6vadxp25e"}]]}}
		let payload = "eyJ0eXAiOiJhcHBsaWNhdGlvbi9kaWRjb21tLXBsYWluK2pzb24iLCJpZCI6ImE2NmQ5NmY2LTRhMWYtNDVkYy04NDJkLWIwYjliNjA5NmNkNSIsInR5cGUiOiJjby1oZWFkcy8xLjAuMCIsImZyb20iOm51bGwsImNyZWF0ZWRfdGltZSI6MTcxNjk5NTYzNSwiZXhwaXJlc190aW1lIjoxNzE2OTk1NzU1LCJib2R5Ijp7ImgiOlsidGVzdCIsW3siLyI6ImJhZnlyNGllcTUyM3I2ZGtsbzJmZjJyZTZnYWJ2eDIzNzd0Z3hsdXJpNHd6eTVkdTR4NnZhZHhwMjVlIn1dXX19";
		let data = multibase::Base::Base64Url.decode(payload).unwrap();
		serde_json::from_slice::<didcomm_rs::Message>(&data).unwrap();
	}
}
