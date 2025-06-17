use super::Command as NetworkCommand;
use crate::{cli::Cli, library::cli_context::CliContext};
use anyhow::Result;
use co_core_membership::MembershipState;
use co_primitives::{DagSetExt, Did};
use co_sdk::{state, CoId, CoReducerFactory, NetworkSettings};
use exitcode::ExitCode;
use futures::{stream, StreamExt, TryStreamExt};
use std::future::ready;

#[derive(Debug, Clone, clap::Args)]
pub struct Command {
	/// CO ID to listen.
	#[arg(long)]
	pub co: Option<Vec<String>>,

	/// Identites to listen to.
	#[arg(long)]
	pub identity: Option<Vec<Did>>,

	/// Listen address (multiaddr like /ip4/0.0.0.0/udp/0/quic-v1).
	#[arg(long, value_name = "MULTIADDR")]
	pub listen: Option<String>,
}

pub async fn command(
	context: &CliContext,
	cli: &Cli,
	network_command: &NetworkCommand,
	command: &Command,
) -> Result<ExitCode, anyhow::Error> {
	// setting
	let mut network_settings =
		NetworkSettings { force_new_peer_id: network_command.force_new_peer_id, ..Default::default() };
	if let Some(listen) = &command.listen {
		network_settings = network_settings.with_listen_from_string(listen)?;
	}

	// application and network
	let mut application = context.application(cli).await;
	application.create_network(network_settings).await?;

	// verbose
	if cli.verbose > 0 {
		if let Some(network) = application.context().network_tasks().await {
			// peer-id
			let peer_id = network.local_peer_id();
			println!("peer-id: {}", peer_id);

			// listeners
			let listeners = network.listeners().await?;
			for listener in listeners {
				println!("listen: {}", listener);
			}
		}
	}

	// COs
	// TODO: watch local co
	// TODO: https://gitlab.1io.com/1io/co-sdk/-/issues/52
	let cos: Vec<CoId> = match &command.identity {
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
