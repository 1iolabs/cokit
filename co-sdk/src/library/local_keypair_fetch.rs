use crate::{keystore_fetch, CoReducer};
use anyhow::anyhow;
use co_core_keystore::Key;
use co_identity::PrivateIdentity;
use co_primitives::tags;
use libp2p::identity::Keypair;

pub async fn local_keypair_fetch<I: PrivateIdentity + Send + Sync>(
	identifier: &str,
	local_co: &CoReducer,
	identity: &I,
	force_new_peer_id: bool,
) -> Result<Keypair, anyhow::Error> {
	let uri = format!("urn:local:{}:peer-id", identifier);
	let key = keystore_fetch(
		local_co,
		identity,
		&uri,
		|| {
			let keypair = Keypair::generate_ed25519();
			Key {
				uri: uri.clone(),
				description: "co libp2p device peer-id".to_owned(),
				name: "co peer id".to_owned(),
				secret: co_core_keystore::Secret::PrivateKey(keypair.to_protobuf_encoding().unwrap()),
				tags: tags!(),
			}
		},
		force_new_peer_id,
	)
	.await?;
	Ok(match key.secret {
		co_core_keystore::Secret::PrivateKey(p) => Ok(Keypair::from_protobuf_encoding(&p)?),
		_ => Err(anyhow!("Expected private key: {}", key.uri)),
	}?)
}
