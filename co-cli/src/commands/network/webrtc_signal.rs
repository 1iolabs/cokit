// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

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
	let network_settings = NetworkSettings::new()
		.with_force_new_peer_id(network_command.force_new_peer_id)
		.with_listen(command.listen.clone())
		.with_bootstraps(bootstrap)
		.with_added_external_addresses(command.external_address.clone())
		.with_relay(true)
		.with_mdns(false)
		.with_nat(true)
		.build()?;

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
	}

	// run until shutdown
	application.shutdown().cancelled().await;

	Ok(exitcode::OK)
}
