use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
pub struct CoSettings {
	pub identifier: Option<String>,
	pub path: Option<PathBuf>,
	pub network_force_new_peer_id: bool,
	pub network: bool,
}
impl CoSettings {
	pub fn with_path(self, path: &str) -> Self {
		Self { path: Some(path.into()), ..self }
	}

	pub fn with_identifier(self, identifier: &str) -> Self {
		Self { identifier: Some(identifier.into()), ..self }
	}

	pub fn with_network(self, force_new_peer_id: bool) -> Self {
		Self { network: true, network_force_new_peer_id: force_new_peer_id, ..self }
	}
}
