use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
pub struct CoSettings {
	pub identifier: String,
	pub path: Option<PathBuf>,
	pub network_force_new_peer_id: bool,
	pub network: bool,
	pub no_keychain: bool,
	pub no_log: bool,
}
impl CoSettings {
	pub fn new(identifier: &str) -> Self {
		CoSettings { identifier: identifier.into(), ..Default::default() }
	}

	pub fn with_path(self, path: &str) -> Self {
		Self { path: Some(path.into()), ..self }
	}

	pub fn with_network(self, force_new_peer_id: bool) -> Self {
		Self { network: true, network_force_new_peer_id: force_new_peer_id, ..self }
	}

	pub fn without_keychain(self) -> Self {
		Self { no_keychain: true, ..self }
	}
}
