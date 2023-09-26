use libp2p::identity::Keypair;
use std::{
	io::{Error, ErrorKind},
	path::PathBuf,
};
use tokio::fs::{read, write};
use tracing::info;

/// Read or generate and persist key-pair from/to file.
pub async fn local_key(local_key_path: Option<PathBuf>, force_new_peer_id: bool) -> Result<Keypair, Error> {
	if let Some(local_key_path) = local_key_path {
		// force?
		if force_new_peer_id {
			return persist(generate(), local_key_path).await
		}

		// read or generate and persist
		match read(local_key_path.clone()).await {
			Ok(data) => {
				info!(?local_key_path, "loading-key-file");
				Ok(Keypair::from_protobuf_encoding(data.as_slice()).map_err(|e| Error::new(ErrorKind::Other, e))?)
			},
			Err(e) => match e.kind() {
				ErrorKind::NotFound => persist(generate(), local_key_path).await,
				_ => Err(e),
			},
		}
	} else {
		Ok(generate())
	}
}

fn generate() -> Keypair {
	Keypair::generate_ed25519()
}

async fn persist(key: Keypair, local_key_path: PathBuf) -> Result<Keypair, Error> {
	info!(?local_key_path, "writing-key-file");
	write(local_key_path, key.to_protobuf_encoding().map_err(|e| Error::new(ErrorKind::Other, e))?).await?;
	Ok(key)
}
