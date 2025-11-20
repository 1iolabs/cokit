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
	#[arg(long, value_name = "MULTIADDR", value_parser = parse_bootstrap, default_values_t = default_bootstrap())]
	pub bootstrap: Vec<Multiaddr>,

	/// Enable relay server (Limited for DCUtR).
	#[arg(short)]
	pub relay: bool,
}

fn default_bootstrap() -> Vec<Multiaddr> {
	NetworkSettings::default().bootstrap.into_iter().collect()
}

fn default_listen() -> Multiaddr {
	NetworkSettings::default().listen
}

fn parse_bootstrap(str: &str) -> Result<Multiaddr, anyhow::Error> {
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
		.with_bootstraps(command.bootstrap.iter().cloned())
		.with_relay(command.relay)
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
		network.didcontact_subscribe_default().await?;
	}

	// COs
	// TODO: watch local co
	// TODO: https://gitlab.1io.com/1io/co-sdk/-/issues/52
	let cos: Vec<CoId> = match &command.co {
		Some(dids) => dids.iter().map(|id| CoId::from(id)).collect(),
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
