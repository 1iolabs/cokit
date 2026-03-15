// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use super::Command as NetworkCommand;
use crate::{cli::Cli, library::cli_context::CliContext};
use anyhow::Result;
use co_core_membership::MembershipState;
use co_primitives::Did;
use co_sdk::{state, CoId, CoReducerFactory, NetworkSettings};
use exitcode::ExitCode;
use futures::{stream, StreamExt, TryStreamExt};
use multiaddr::Multiaddr;
use std::future::ready;

#[derive(Debug, Clone, clap::Args)]
pub struct Command {
	/// CO ID to listen. By default uses all active COs.
	#[arg(long)]
	pub co: Option<Vec<String>>,

	/// Identites to listen to. By default uses all private identities.
	#[arg(long)]
	pub identity: Option<Vec<Did>>,

	/// Listen address.
	#[arg(long, value_name = "MULTIADDR", default_value_t = default_listen())]
	pub listen: Multiaddr,

	/// Bootstap addresses.
	///
	/// # Examples
	/// - `/dns4/bootstrap.1io.com/udp/5000/quic-v1/p2p/12D3KooWEinh2zCgGbJaDfepoiiPiBgFcysSMYSc1EQrgEEZi9aX`
	#[arg(long, value_name = "MULTIADDR", value_parser = parse_bootstrap, default_values_t = default_bootstrap(), conflicts_with = "no_bootstrap")]
	pub bootstrap: Vec<Multiaddr>,

	/// External address.
	///
	/// # Examples
	/// - `/dns4/bootstrap.1io.com/upd/5000`
	#[arg(long, value_name = "MULTIADDR")]
	pub external_address: Vec<Multiaddr>,

	/// Do not use any bootstraps.
	#[arg(long)]
	pub no_bootstrap: bool,

	/// Enable relay server.
	/// A (public) external address needs to be configured to enable it.
	/// The relay is limited and only used for holepunching (DCUtR).
	#[arg(long, short, requires = "external_address")]
	pub relay: bool,

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

pub fn default_listen() -> Multiaddr {
	NetworkSettings::default().listen
}

pub fn parse_bootstrap(str: &str) -> Result<Multiaddr, anyhow::Error> {
	let addr: Multiaddr = str.parse()?;
	NetworkSettings::default().with_bootstrap(addr.clone()).build()?;
	Ok(addr)
}

pub async fn command(
	context: &CliContext,
	cli: &Cli,
	network_command: &NetworkCommand,
	command: &Command,
) -> Result<ExitCode, anyhow::Error> {
	// setting
	let network_settings = NetworkSettings::new()
		.with_force_new_peer_id(network_command.force_new_peer_id)
		.with_listen(command.listen.clone())
		.with_bootstraps(if !command.no_bootstrap { command.bootstrap.clone() } else { Default::default() })
		.with_added_external_addresses(command.external_address.clone())
		.with_relay(command.relay)
		.with_mdns(!command.no_mdns)
		.with_nat(!command.no_nat)
		.build()?;

	// application and network
	let mut application = context.application(cli).await;
	application.create_network(network_settings).await?;

	// verbose
	if cli.verbose > 0 {
		if let Some(network) = application.context().network().await {
			// peer-id
			let peer_id = network.local_peer_id();
			println!("peer-id: {}", peer_id);

			// listeners
			let listeners = network.listeners(true, false).await?;
			for listener in listeners {
				println!("listen: {}", listener);
			}
		}
	}

	// network
	if let Some(network) = application.co().network().await {
		network.didcontact_subscribe_default()?;
	}

	// COs
	// TODO: watch local co
	// TODO: https://gitlab.1io.com/1io/co-sdk/-/issues/52
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
