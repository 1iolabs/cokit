// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{
	didcomm,
	network::{Behaviour, Context, NetworkEvent},
	types::network_task::{NetworkTask, NetworkTaskSpawner},
};
use libp2p::{swarm::SwarmEvent, PeerId, Swarm};
use std::{collections::BTreeSet, time::Duration};

/// Handle received heads from the network within the application.
/// This structure essentially joins the received heads into the respective co reducer.
#[derive(Debug)]
pub struct DidCommSendNetworkTask {
	message: didcomm::EncodedMessage,
	peers: BTreeSet<PeerId>,
	sent: Option<tokio::sync::oneshot::Sender<anyhow::Result<PeerId>>>,
}
impl DidCommSendNetworkTask {
	/// Send DIDComm message to a peer.
	/// Resolves as soon the message could be sent to one of the specified peers.
	pub async fn send<S>(
		spawner: S,
		peers: impl IntoIterator<Item = PeerId>,
		message: didcomm::EncodedMessage,
		timeout: Duration,
	) -> anyhow::Result<PeerId>
	where
		S: NetworkTaskSpawner<Behaviour, Context> + Send + Sync + 'static,
	{
		let (tx, rx) = tokio::sync::oneshot::channel();
		let task = Self { message, peers: peers.into_iter().collect(), sent: Some(tx) };
		spawner.spawn(task)?;
		crate::compat::timeout(timeout, rx).await??
	}
}
impl NetworkTask<Behaviour, Context> for DidCommSendNetworkTask {
	fn execute(&mut self, swarm: &mut Swarm<Behaviour>, _context: &mut Context) {
		for peer in &self.peers {
			swarm.behaviour_mut().didcomm.send(peer, self.message.clone());
		}
	}

	fn on_swarm_event(
		&mut self,
		_swarm: &mut Swarm<Behaviour>,
		_context: &mut Context,
		event: SwarmEvent<NetworkEvent>,
	) -> Option<SwarmEvent<NetworkEvent>> {
		if let SwarmEvent::Behaviour(NetworkEvent::Didcomm(didcomm_event)) = &event {
			match &didcomm_event {
				didcomm::Event::Sent { peer_id, message } => {
					// check the message before removing the peer as the peer may sent other message at same time
					if &self.message == message && self.peers.remove(peer_id) {
						if let Some(sent) = Option::take(&mut self.sent) {
							sent.send(Ok(*peer_id)).ok();
						}
					}
				},
				didcomm::Event::OutboundFailure { peer_id, error, message } => {
					if (self.peers.is_empty() || Some(&self.message) == message.as_ref()) && self.peers.remove(peer_id)
					{
						if let Some(sent) = Option::take(&mut self.sent) {
							sent.send(Err(error.clone().into())).ok();
						}
					}
				},
				_ => {},
			}
		}
		Some(event)
	}

	fn is_complete(&mut self) -> bool {
		self.sent.as_ref().map(|i| i.is_closed()).unwrap_or_default()
	}
}

// #[derive(Debug, Error)]
// pub enum DidCommSendError
// {
// 	#[error("Message send timeout")]
// 	Timeout,
// 	#[error("Network connect timeout")]
// 	NetworkTimeout,
// 	#[error("Network connect timeout")]
// 	Other(#[source] anyhow::Error),
// }
