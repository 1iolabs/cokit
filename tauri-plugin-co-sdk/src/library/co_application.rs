use co_sdk::{Application, ApplicationBuilder, NetworkSettings};
use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
pub struct CoApplicationSettings {
	pub instance_id: String,
	pub base_path: Option<PathBuf>,
	pub force_new_peer_id: bool,
	pub network: bool,
	pub no_keychain: bool,
	pub no_log: bool,
}
impl CoApplicationSettings {
	pub fn new(identifier: &str) -> Self {
		CoApplicationSettings { instance_id: identifier.into(), ..Default::default() }
	}

	pub fn with_path(self, path: &str) -> Self {
		Self { base_path: Some(path.into()), ..self }
	}

	pub fn with_network(self, force_new_peer_id: bool) -> Self {
		Self { network: true, force_new_peer_id, ..self }
	}

	pub fn without_keychain(self) -> Self {
		Self { no_keychain: true, ..self }
	}
}

pub async fn application(settings: CoApplicationSettings) -> Application {
	let identifier = settings.instance_id;
	let mut builder = match settings.base_path {
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
			.create_network(NetworkSettings { force_new_peer_id: settings.force_new_peer_id, ..Default::default() })
			.await
			.expect("network");
	}
	application.clone()
}
