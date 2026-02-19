// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::{keystore_fetch, CoReducer};
use anyhow::anyhow;
use co_core_keystore::Key;
use co_identity::PrivateIdentity;
use co_network::Keypair;
use co_primitives::tags;
use std::fmt::Debug;

pub async fn local_keypair_fetch<I>(
	identifier: &str,
	local_co: &CoReducer,
	identity: &I,
	force_new_peer_id: bool,
) -> Result<Keypair, anyhow::Error>
where
	I: PrivateIdentity + Debug + Clone + Send + Sync + 'static,
{
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
				secret: co_core_keystore::Secret::PrivateKey(keypair.to_protobuf_encoding().unwrap().into()),
				tags: tags!(),
			}
		},
		force_new_peer_id,
	)
	.await?;
	match key.secret {
		co_core_keystore::Secret::PrivateKey(p) => Ok(Keypair::from_protobuf_encoding(p.divulge())?),
		_ => Err(anyhow!("Expected private key: {}", key.uri)),
	}
}
