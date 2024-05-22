use co_network::{DidcommBehaviourProvider, GossipsubBehaviourProvider, HeadsLayerBehaviourProvider, NetworkTask};
use co_primitives::{CoId, NetworkCoHeads};
use libipld::Cid;
use libp2p::{swarm::NetworkBehaviour, PeerId, Swarm};
use std::collections::BTreeSet;

#[derive(Debug)]
pub enum HeadsRequest {
	Subscribe { network: NetworkCoHeads, co: CoId },
	Unsubscribe { network: NetworkCoHeads, co: CoId },
	Heads { co: CoId, heads: BTreeSet<Cid>, peers: BTreeSet<PeerId> },
	PublishHeads { network: NetworkCoHeads, co: CoId, heads: BTreeSet<Cid> },
}

pub struct HeadsRequestNetworkTask {
	request: Option<HeadsRequest>,
}
impl HeadsRequestNetworkTask {
	pub fn new(request: HeadsRequest) -> Self {
		Self { request: Some(request) }
	}
}
impl<B, C> NetworkTask<B, C> for HeadsRequestNetworkTask
where
	B: NetworkBehaviour + GossipsubBehaviourProvider + DidcommBehaviourProvider,
	C: HeadsLayerBehaviourProvider,
{
	fn execute(&mut self, swarm: &mut Swarm<B>, context: &mut C) {
		let behaviour = context.heads_mut();
		match Option::take(&mut self.request) {
			Some(HeadsRequest::Subscribe { network, co }) => match behaviour.subscribe(swarm, &network, &co) {
				Ok(true) => {
					tracing::debug!(?co, "co-subscribe");
				},
				Ok(_) => {},
				Err(err) => {
					tracing::warn!(?co, ?err, "co-subscribe-failed");
				},
			},
			Some(HeadsRequest::Unsubscribe { network, co }) => match behaviour.unsubscribe(swarm, &network, &co) {
				Ok(true) => {
					tracing::debug!(?co, "co-unsubscribe");
				},
				Ok(_) => {},
				Err(err) => {
					tracing::warn!(?co, ?err, "co-unsubscribe-failed");
				},
			},
			Some(HeadsRequest::Heads { co, heads, peers }) =>
				match behaviour.heads(swarm, &co, heads, peers.into_iter()) {
					Ok(_) => {
						tracing::debug!(?co, "co-request-heads");
					},
					Err(err) => {
						tracing::warn!(?co, ?err, "co-request-heads-failed");
					},
				},
			Some(HeadsRequest::PublishHeads { network, co, heads }) =>
				match behaviour.publish(swarm, &network, &co, &heads) {
					Ok(_) => {
						tracing::debug!(?co, "co-publish-heads");
					},
					Err(err) => {
						tracing::warn!(?co, ?err, "co-publish-heads-failed");
					},
				},
			None => {},
		}
	}
}
