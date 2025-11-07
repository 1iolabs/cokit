use crate::{
	library::network_discovery::network_discovery,
	services::{
		connections::{
			actor::ConnectionsContext, ConnectAction, ConnectedAction, ConnectionAction, ConnectionState,
			DisconnectReason, DisconnectedAction,
		},
		network::DiscoveryConnectNetworkTask,
	},
	types::network_task::NetworkTaskSpawner,
};
use co_actor::{Actions, Epic};
use co_identity::PrivateIdentityResolver;
use co_primitives::{Did, Network};
use futures::{Stream, StreamExt, TryStreamExt};

pub struct ConnectEpic();
impl ConnectEpic {
	pub fn new() -> Self {
		Self()
	}
}
impl Epic<ConnectionAction, ConnectionState, ConnectionsContext> for ConnectEpic {
	fn epic(
		&mut self,
		_actions: &Actions<ConnectionAction, ConnectionState, ConnectionsContext>,
		message: &ConnectionAction,
		_state: &ConnectionState,
		context: &ConnectionsContext,
	) -> Option<impl Stream<Item = Result<ConnectionAction, anyhow::Error>> + 'static> {
		match message {
			ConnectionAction::Connect(ConnectAction { from, network }) => {
				Some(connect(context.clone(), from.clone(), network.clone()).map({
					let network = network.clone();
					move |item| match item {
						Ok(action) => Ok(action),
						Err(err) => Ok(ConnectionAction::Disconnected(DisconnectedAction {
							network: network.clone(),
							reason: DisconnectReason::Failure(err.to_string()),
						})),
					}
				}))
			},
			_ => None,
		}
	}
}

fn connect(
	context: ConnectionsContext,
	from: Did,
	network: Network,
) -> impl Stream<Item = Result<ConnectionAction, anyhow::Error>> + 'static {
	async_stream::try_stream! {
		// discovery
		let from_identity = context.private_identity_resolver.resolve_private(&from).await?;
		let discovery = network_discovery(Some(&context.identity_resolver), context.network.local_peer_id(), &from_identity, [network.clone()], []).try_collect().await?;

		// connect
		let (task, peers) = DiscoveryConnectNetworkTask::new(discovery);

		// spawn
		context.network.spawn(task)?;

		// yield
		for await peer in peers {
			yield ConnectionAction::Connected(ConnectedAction { network: network.clone(), result: peer.map_err(|err| err.to_string()) });
		}
	}
}
