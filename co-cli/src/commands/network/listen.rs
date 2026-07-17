// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use super::Command as NetworkCommand;
use crate::{cli::Cli, library::cli_context::CliContext};
use anyhow::Result;
use co_core_membership::MembershipState;
use co_primitives::Did;
use co_sdk::{state, CoId, CoReducerFactory, NetworkSettings};
use exitcode::ExitCode;
use futures::{stream, StreamExt, TryStreamExt};
use multiaddr::Multiaddr;
use std::{future::ready, time::Duration};

/// Listen for connections.
///
/// Relay node (native + browser):
///
/// ```sh
/// co --base-path /var/lib/co-bootstrap --no-keychain network listen \
///   --listen /ip4/0.0.0.0/udp/5000/quic-v1,/ip6/::/udp/5000/quic-v1,/ip4/0.0.0.0/tcp/4001/ws \
///   --relay \
///   --external-address /dns4/bootstrap.1io.com/udp/5000/quic-v1 \
///   --external-address /dns4/bootstrap.1io.com/tcp/443/wss \
///   --no-mdns --no-bootstrap
/// ```
///
/// Browsers on `https://` pages require `wss`. Terminate TLS at a reverse proxy
/// on port 443, forward to the plain `/ws` listener, and advertise the public
/// `/wss` address with `--external-address`.
/// Example: `wss://bootstrap.1io.com:443` -> `ws://127.0.0.1:4001`.
/// Startup prints copy-paste `relay:` addresses for browser and native clients.
///
/// Keep `--no-bootstrap` on the bootstrap node. With a fresh keystore, its peer
/// ID differs from the compiled-in production bootstrap and it would otherwise
/// dial the old production node.
#[derive(Debug, Clone, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Command {
	/// CO ID to listen. By default uses all active COs.
	#[arg(long)]
	pub co: Option<Vec<String>>,

	/// Identites to listen to. By default uses all private identities.
	#[arg(long)]
	pub identity: Option<Vec<Did>>,

	/// Listen addresses (comma separated).
	#[arg(long, value_name = "MULTIADDR", value_delimiter = ',', default_values_t = default_listen())]
	pub listen: Vec<Multiaddr>,

	/// Bootstap addresses.
	///
	/// # Examples
	/// - `/dns4/bootstrap.1io.com/udp/5000/quic-v1/p2p/12D3KooWEinh2zCgGbJaDfepoiiPiBgFcysSMYSc1EQrgEEZi9aX`
	#[arg(long, value_name = "MULTIADDR", value_parser = parse_bootstrap, default_values_t = default_bootstrap(), conflicts_with = "no_bootstrap")]
	pub bootstrap: Vec<Multiaddr>,

	/// External address.
	///
	/// # Examples
	/// - `/dns4/bootstrap.1io.com/udp/5000/quic-v1`
	/// - `/dns4/bootstrap.1io.com/tcp/443/wss`
	#[arg(long, value_name = "MULTIADDR")]
	pub external_address: Vec<Multiaddr>,

	/// Do not use any bootstraps.
	#[arg(long)]
	pub no_bootstrap: bool,

	/// Enable the limited relay server for hole punching and circuit traffic.
	/// At least one public external address must be configured.
	#[arg(long, short, requires = "external_address")]
	pub relay: bool,

	/// Maximum bytes allowed per relay circuit (default: 128 KiB).
	#[arg(long, value_name = "BYTES", requires = "relay")]
	pub max_circuit_bytes: Option<u64>,

	/// Maximum duration in seconds per relay circuit (default: 120s).
	#[arg(long, value_name = "SECONDS", requires = "relay")]
	pub max_circuit_duration: Option<u64>,

	/// Disable mDNS protocol client.
	#[arg(long)]
	pub no_mdns: bool,

	/// Disable NAT protocol clients.
	#[arg(long)]
	pub no_nat: bool,
}

pub fn default_bootstrap() -> Vec<Multiaddr> {
	NetworkSettings::default().bootstrap.into_iter().collect()
}

pub fn default_listen() -> Vec<Multiaddr> {
	NetworkSettings::default().listen.into_iter().collect()
}

pub fn parse_bootstrap(str: &str) -> Result<Multiaddr, anyhow::Error> {
	let addr: Multiaddr = str.parse()?;
	NetworkSettings::default().with_bootstrap(addr.clone()).build()?;
	Ok(addr)
}

fn build_network_settings(force_new_peer_id: bool, command: &Command) -> Result<NetworkSettings> {
	let mut settings = NetworkSettings::new()
		.with_force_new_peer_id(force_new_peer_id)
		.with_listens(command.listen.clone())
		.with_bootstraps(if !command.no_bootstrap { command.bootstrap.clone() } else { Default::default() })
		.with_added_external_addresses(command.external_address.clone())
		.with_relay(command.relay)
		.with_mdns(!command.no_mdns)
		.with_nat(!command.no_nat);

	if let Some(bytes) = command.max_circuit_bytes {
		settings = settings.with_max_circuit_bytes(bytes);
	}
	if let Some(seconds) = command.max_circuit_duration {
		settings = settings.with_max_circuit_duration(Duration::from_secs(seconds));
	}

	settings.build()
}

fn network_info_lines(command: &Command, verbose: bool, peer_id: &str, listeners: &[Multiaddr]) -> Vec<String> {
	let mut lines = Vec::new();

	if command.relay {
		lines.push(format!("peer-id: {peer_id}"));
		lines.extend(
			command
				.external_address
				.iter()
				.map(|external| format!("relay: {external}/p2p/{peer_id}")),
		);
	} else if verbose {
		lines.push(format!("peer-id: {peer_id}"));
	}

	if verbose {
		lines.extend(listeners.iter().map(|listener| format!("listen: {listener}")));
	}

	lines
}

pub async fn command(
	context: &CliContext,
	cli: &Cli,
	network_command: &NetworkCommand,
	command: &Command,
) -> Result<ExitCode, anyhow::Error> {
	// setting
	let network_settings = build_network_settings(network_command.force_new_peer_id, command)?;

	// application and network
	let mut application = context.application(cli).await;
	application.create_network(network_settings).await?;

	let verbose = cli.log.verbose > 0;
	if command.relay || verbose {
		if let Some(network) = application.context().network().await {
			let peer_id = network.local_peer_id().to_string();
			let listeners = if verbose {
				network.listeners(true, false).await?.into_iter().collect::<Vec<_>>()
			} else {
				Vec::new()
			};

			for line in network_info_lines(command, verbose, &peer_id, &listeners) {
				println!("{line}");
			}
		}
	}

	// network
	if let Some(network) = application.co().network().await {
		network.didcontact_subscribe_default()?;
	}

	// COs
	// TODO: watch local co
	// TODO: https://gitlab.1io.com/1io/cokit/-/issues/52
	let cos: Vec<CoId> = match &command.co {
		Some(dids) => dids.iter().map(CoId::from).collect(),
		None => {
			let local_co = application.local_co_reducer().await?;
			let co_context = application.co();
			state::memberships(local_co.storage(), local_co.reducer_state().await.co())
				.try_filter(|(_, _, _, membership_state)| ready(*membership_state == MembershipState::Active))
				.map_ok(|membership| membership.0)
				.then(move |id| async move {
					match id {
						Ok(id) => {
							let co = co_context.try_co_reducer(&id).await?;
							let (_storage, co_state) = co.co().await?;
							if co_state.network.is_empty() {
								Ok(None)
							} else {
								Ok(Some(id))
							}
						},
						Err(err) => Err(Into::<anyhow::Error>::into(err)),
					}
				})
				.filter_map(|id| async move {
					match id {
						Ok(None) => None,
						Ok(Some(id)) => Some(Ok(id)),
						Err(e) => Some(Err(e)),
					}
				})
				.try_collect()
				.await?
		},
	};
	let _cos = stream::iter(cos)
		.then(|co| async { application.co_reducer(co).await })
		.try_filter_map(|item| ready(Ok(item)))
		.try_collect::<Vec<_>>()
		.await?;

	// listen forever
	application.shutdown().cancelled().await;

	// result
	Ok(exitcode::OK)
}

#[cfg(test)]
mod tests {
	use super::Command;
	use clap::{error::ErrorKind, Parser};
	use multiaddr::Multiaddr;
	use std::time::Duration;

	#[derive(Parser)]
	struct Wrapper {
		#[command(flatten)]
		command: Command,
	}

	#[test]
	fn circuit_limits_require_relay() {
		for arguments in [["test", "--max-circuit-bytes", "1048576"], ["test", "--max-circuit-duration", "600"]] {
			let error = Wrapper::try_parse_from(arguments)
				.err()
				.expect("a circuit limit without --relay must fail");
			assert_eq!(error.kind(), ErrorKind::MissingRequiredArgument);
		}
	}

	#[test]
	fn relay_circuit_limits_map_to_network_settings() {
		let wrapper = Wrapper::parse_from([
			"test",
			"--relay",
			"--external-address",
			"/dns4/bootstrap.1io.com/tcp/443/wss",
			"--max-circuit-bytes",
			"1048576",
			"--max-circuit-duration",
			"600",
		]);

		assert_eq!(wrapper.command.max_circuit_bytes, Some(1_048_576));
		assert_eq!(wrapper.command.max_circuit_duration, Some(600));

		let settings = super::build_network_settings(false, &wrapper.command).expect("valid network settings");
		assert_eq!(settings.max_circuit_bytes, Some(1_048_576));
		assert_eq!(settings.max_circuit_duration, Some(Duration::from_secs(600)));
	}

	#[test]
	fn listen_accepts_quic_and_websocket_addresses() {
		let wrapper = Wrapper::parse_from([
			"test",
			"--listen",
			"/ip4/0.0.0.0/udp/5000/quic-v1,/ip6/::/udp/5000/quic-v1,/ip4/0.0.0.0/tcp/4001/ws",
		]);

		assert_eq!(wrapper.command.listen.len(), 3);
		assert!(wrapper
			.command
			.listen
			.contains(&"/ip4/0.0.0.0/udp/5000/quic-v1".parse().unwrap()));
		assert!(wrapper.command.listen.contains(&"/ip6/::/udp/5000/quic-v1".parse().unwrap()));
		assert!(wrapper.command.listen.contains(&"/ip4/0.0.0.0/tcp/4001/ws".parse().unwrap()));
	}

	#[test]
	fn network_info_lines_cover_relay_and_verbose_modes() {
		let relay = Wrapper::parse_from([
			"test",
			"--relay",
			"--external-address",
			"/dns4/bootstrap.1io.com/udp/5000/quic-v1",
			"--external-address",
			"/dns4/bootstrap.1io.com/tcp/443/wss",
		]);
		let peer_id = "12D3KooWRelayPeer";
		let listeners: Vec<Multiaddr> = vec!["/ip4/0.0.0.0/tcp/4001/ws".parse().unwrap()];

		assert_eq!(
			super::network_info_lines(&relay.command, false, peer_id, &[]),
			vec![
				format!("peer-id: {peer_id}"),
				format!("relay: /dns4/bootstrap.1io.com/udp/5000/quic-v1/p2p/{peer_id}"),
				format!("relay: /dns4/bootstrap.1io.com/tcp/443/wss/p2p/{peer_id}"),
			],
		);
		assert_eq!(
			super::network_info_lines(&relay.command, true, peer_id, &listeners),
			vec![
				format!("peer-id: {peer_id}"),
				format!("relay: /dns4/bootstrap.1io.com/udp/5000/quic-v1/p2p/{peer_id}"),
				format!("relay: /dns4/bootstrap.1io.com/tcp/443/wss/p2p/{peer_id}"),
				"listen: /ip4/0.0.0.0/tcp/4001/ws".to_owned(),
			],
		);

		let non_relay = Wrapper::parse_from(["test"]);
		assert!(super::network_info_lines(&non_relay.command, false, peer_id, &listeners).is_empty());
		assert_eq!(
			super::network_info_lines(&non_relay.command, true, peer_id, &listeners),
			vec![format!("peer-id: {peer_id}"), "listen: /ip4/0.0.0.0/tcp/4001/ws".to_owned(),],
		);
	}

	#[test]
	fn listen_defaults_to_dual_stack() {
		let wrapper = Wrapper::parse_from(["test"]);
		assert_eq!(wrapper.command.listen.len(), 2);
	}
}
