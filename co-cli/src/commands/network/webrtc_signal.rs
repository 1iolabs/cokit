// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use super::Command as NetworkCommand;
use crate::{
	cli::Cli,
	commands::network::listen::{default_bootstrap, parse_bootstrap},
	library::cli_context::CliContext,
};
use anyhow::Result;
use co_sdk::NetworkSettings;
use exitcode::ExitCode;
use multiaddr::Multiaddr;
use std::time::Duration;

/// Run a relay node for browser WebRTC peers.
#[derive(Debug, Clone, clap::Args)]
pub struct Command {
	/// Listen address (WebSocket for browser connectivity).
	#[arg(long, value_name = "MULTIADDR", default_value = "/ip4/0.0.0.0/tcp/4001/ws")]
	pub listen: Multiaddr,

	/// External address that browsers use to reach this relay.
	///
	/// # Examples
	/// - `/ip4/127.0.0.1/tcp/4001/ws`
	/// - `/dns4/bootstrap.1io.com/tcp/4001/ws`
	#[arg(long, value_name = "MULTIADDR", required = true)]
	pub external_address: Vec<Multiaddr>,

	/// Bootstrap addresses.
	#[arg(long, value_name = "MULTIADDR", value_parser = parse_bootstrap, default_values_t = default_bootstrap(), conflicts_with = "no_bootstrap")]
	pub bootstrap: Vec<Multiaddr>,

	/// Do not use default bootstraps.
	#[arg(long)]
	pub no_bootstrap: bool,

	/// Maximum bytes allowed per relay circuit (default: 128 KiB).
	#[arg(long, value_name = "BYTES")]
	pub max_circuit_bytes: Option<u64>,

	/// Maximum duration in seconds per relay circuit (default: 120s).
	#[arg(long, value_name = "SECONDS")]
	pub max_circuit_duration: Option<u64>,
}

pub async fn command(
	context: &CliContext,
	cli: &Cli,
	network_command: &NetworkCommand,
	command: &Command,
) -> Result<ExitCode, anyhow::Error> {
	// settings: relay enabled, mdns disabled, nat enabled
	let bootstrap = if command.no_bootstrap {
		command.bootstrap.clone()
	} else {
		let mut bs: Vec<Multiaddr> = NetworkSettings::default().bootstrap.into_iter().collect();
		bs.extend(command.bootstrap.clone());
		bs
	};
	let mut network_settings = NetworkSettings::new()
		.with_force_new_peer_id(network_command.force_new_peer_id)
		.with_listen(command.listen.clone())
		.with_bootstraps(bootstrap)
		.with_added_external_addresses(command.external_address.clone())
		.with_relay(true)
		.with_mdns(false)
		.with_nat(true);
	if let Some(bytes) = command.max_circuit_bytes {
		network_settings = network_settings.with_max_circuit_bytes(bytes);
	}
	if let Some(seconds) = command.max_circuit_duration {
		network_settings = network_settings.with_max_circuit_duration(Duration::from_secs(seconds));
	}
	let network_settings = network_settings.build()?;

	// application and network
	let mut application = context.application(cli).await;
	application.create_network(network_settings).await?;

	// print relay info
	if let Some(network) = application.context().network().await {
		let peer_id = network.local_peer_id();
		println!("peer-id: {peer_id}");

		let listeners = network.listeners(true, true).await?;
		for listener in &listeners {
			println!("listen: {listener}");
		}

		// print relay multiaddrs for browser configuration
		for external in &command.external_address {
			println!("relay: {external}/p2p/{peer_id}");
		}

		// subscribe to gossipsub topics so the relay participates in the mesh
		// and can forward messages between browser peers
		network.subscribe_gossip_topic("co-contact").await?;
	}

	// run until shutdown
	application.shutdown().cancelled().await;

	Ok(exitcode::OK)
}
