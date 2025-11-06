use crate::{didcomm, DidcommBehaviourProvider, NetworkTask, NetworkTaskSpawner};
use libp2p::{
	swarm::{NetworkBehaviour, SwarmEvent},
	PeerId, Swarm,
};
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
	pub async fn send<B, C, S>(
		spawner: S,
		peers: impl IntoIterator<Item = PeerId>,
		message: didcomm::EncodedMessage,
		timeout: Duration,
	) -> anyhow::Result<PeerId>
	where
		S: NetworkTaskSpawner<B, C> + Send + Sync + 'static,
		B: NetworkBehaviour + DidcommBehaviourProvider,
	{
		let (tx, rx) = tokio::sync::oneshot::channel();
		let task = Self { message, peers: peers.into_iter().collect(), sent: Some(tx) };
		spawner.spawn(task)?;
		Ok(tokio::time::timeout(timeout, rx).await???)
	}
}
impl<B, C> NetworkTask<B, C> for DidCommSendNetworkTask
where
	B: NetworkBehaviour + DidcommBehaviourProvider,
{
	fn execute(&mut self, swarm: &mut Swarm<B>, _context: &mut C) {
		for peer in &self.peers {
			swarm.behaviour_mut().didcomm_mut().send(peer, self.message.clone());
		}
	}

	fn on_swarm_event(
		&mut self,
		_swarm: &mut Swarm<B>,
		_context: &mut C,
		event: SwarmEvent<B::ToSwarm>,
	) -> Option<SwarmEvent<B::ToSwarm>> {
		if let Some(didcomm_event) = B::swarm_didcomm_event(&event) {
			match &didcomm_event {
				didcomm::Event::Sent { peer_id, message } => {
					// check the message before removing the peer as the peer may sent other message at same time
					if &self.message == message {
						if self.peers.remove(peer_id) {
							if let Some(sent) = Option::take(&mut self.sent) {
								sent.send(Ok(*peer_id)).ok();
							}
						}
					}
				},
				didcomm::Event::OutboundFailure { peer_id, error, message } => {
					if self.peers.is_empty() || Some(&self.message) == message.as_ref() {
						if self.peers.remove(peer_id) {
							if let Some(sent) = Option::take(&mut self.sent) {
								sent.send(Err(error.clone().into())).ok();
							}
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
