use co_sdk::{Application, ApplicationBuilder, NetworkSettings};
use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
pub struct CoApplicationSettings {
	pub identifier: String,
	pub path: Option<PathBuf>,
	pub network_force_new_peer_id: bool,
	pub network: bool,
	pub no_keychain: bool,
	pub no_log: bool,
}
impl CoApplicationSettings {
	pub fn new(identifier: &str) -> Self {
		CoApplicationSettings { identifier: identifier.into(), ..Default::default() }
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

pub async fn application(settings: CoApplicationSettings) -> Application {
	let identifier = settings.identifier;
	let mut builder = match settings.path {
		Some(path) => ApplicationBuilder::new_with_path(identifier, path),
		None => ApplicationBuilder::new(identifier),
	};
	if settings.no_keychain {
		builder = builder.without_keychain()
	}
	let mut application = builder.with_bunyan_logging(None).build().await.expect("application");

	// network
	if settings.network {
		application
			.create_network(NetworkSettings::new().with_force_new_peer_id(settings.network_force_new_peer_id))
			.await
			.expect("network");
	}
	application.clone()
}
