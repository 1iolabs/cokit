use crate::{
	actor::Epic,
	drivers::network::tasks::discovery_connect::DiscoveryConnectNetworkTask,
	library::network_discovery::network_discovery,
	plugins::connections::{ConnectionAction, ConnectionState, DisconnectReason, NetworkWithContext},
	CoContext,
};
use co_identity::PrivateIdentityResolver;
use co_network::NetworkTaskSpawner;
use co_primitives::{Did, Network};
use futures::{Stream, StreamExt, TryStreamExt};

pub struct ConnectEpic {}
impl Epic<ConnectionAction, ConnectionState, CoContext> for ConnectEpic {
	fn epic(
		&mut self,
		message: &ConnectionAction,
		_state: &ConnectionState,
		context: &CoContext,
	) -> Option<impl Stream<Item = Result<ConnectionAction, anyhow::Error>> + 'static> {
		match message {
			ConnectionAction::Connect(NetworkWithContext { from, network }) => {
				Some(connect(context.clone(), from.clone(), network.clone()).map({
					let network = network.clone();
					move |item| match item {
						Ok(action) => Ok(action),
						Err(err) => Ok(ConnectionAction::Disconnected(network.clone(), DisconnectReason::Failure(err))),
					}
				}))
			},
			_ => None,
		}
	}
}

fn connect(
	context: CoContext,
	from: Did,
	network: Network,
) -> impl Stream<Item = Result<ConnectionAction, anyhow::Error>> + 'static {
	async_stream::try_stream! {
		// network
		let spawner = match context.network().await {
			Some(v) => v,
			None => {
				yield ConnectionAction::Disconnected(network.clone(), DisconnectReason::NoNetwork);
				return;
			},
		};

		// discovery
		let identity_resolver = context.identity_resolver().await?;
		let from_identity = context.private_identity_resolver().await?.resolve_private(&from).await?;
		let discovery = network_discovery(Some(&identity_resolver), &from_identity, [network.clone()], []).try_collect().await?;

		// connect
		let (task, peers) = DiscoveryConnectNetworkTask::new(discovery);

		// spawn
		spawner.spawn(task)?;

		// yield
		for await peer in peers {
			yield ConnectionAction::Connected(network.clone(), peer.map_err(Into::into));
		}
	}
}
