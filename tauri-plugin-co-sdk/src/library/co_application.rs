// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::library::cli::Cli;
use clap::Parser;
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

	/// Create `CoApplicationSettings` from command line args.
	pub fn cli(identifier: &str) -> Self {
		let mut cli = Cli::parse();
		if cli.instance_id.is_none() {
			cli.instance_id = Some(identifier.to_owned());
		}
		cli.into()
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

pub async fn start_application(settings: CoApplicationSettings) -> Result<Application, anyhow::Error> {
	let identifier = settings.instance_id;
	let mut builder = match settings.base_path {
		Some(path) => ApplicationBuilder::new_with_path(identifier, path),
		None => ApplicationBuilder::new(identifier),
	};
	if settings.no_keychain {
		builder = builder.without_keychain()
	}
	let mut application = builder.with_bunyan_logging(None).build().await?;

	// network
	if settings.network {
		application
			.create_network(NetworkSettings::new().with_force_new_peer_id(settings.force_new_peer_id))
			.await?;
	}

	Ok(application.clone())
}
