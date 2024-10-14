use crate::drivers::network::{
	tasks::{didcomm_send::DidCommSendNetworkTask, discovery_connect::DiscoveryConnectNetworkTask},
	CoNetworkTaskSpawner,
};
use co_network::{didcomm::EncodedMessage, discovery::Discovery};
use futures::Stream;
use libp2p::PeerId;
use std::time::Duration;

/// Try to connect and send message.
#[deprecated]
pub fn connect_and_send(
	network: CoNetworkTaskSpawner,
	message: EncodedMessage,
	networks: impl IntoIterator<Item = Discovery> + Send + 'static,
	timeout: Duration,
) -> impl Stream<Item = anyhow::Result<PeerId>> + Send + 'static {
	async_stream::stream! {
		let connect_peers = DiscoveryConnectNetworkTask::connect_with_timeout(network.clone(), networks, timeout);
		for await peer in connect_peers {
			if let Ok(peer) = peer {
				let send = DidCommSendNetworkTask::send(
					network.clone(),
					[peer],
					message.clone(),
					timeout,
				).await;
				yield send.map(|_| peer);
			}
		}
	}
}
