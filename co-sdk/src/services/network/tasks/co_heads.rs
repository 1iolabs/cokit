use co_network::{DidcommBehaviourProvider, GossipsubBehaviourProvider, HeadsLayerBehaviourProvider, NetworkTask};
use co_primitives::NetworkCoHeads;
use cid::Cid;
use libp2p::{swarm::NetworkBehaviour, Swarm};
use std::collections::BTreeSet;

#[derive(Debug)]
pub enum CoHeadsRequest {
	Subscribe { network: NetworkCoHeads },
	Unsubscribe { network: NetworkCoHeads },
	PublishHeads { network: NetworkCoHeads, heads: BTreeSet<Cid> },
}

#[derive(Debug)]
pub struct CoHeadsNetworkTask {
	request: Option<CoHeadsRequest>,
}
impl CoHeadsNetworkTask {
	pub fn new(request: CoHeadsRequest) -> Self {
		Self { request: Some(request) }
	}
}
impl<B, C> NetworkTask<B, C> for CoHeadsNetworkTask
where
	B: NetworkBehaviour + GossipsubBehaviourProvider + DidcommBehaviourProvider,
	C: HeadsLayerBehaviourProvider,
{
	fn execute(&mut self, swarm: &mut Swarm<B>, context: &mut C) {
		let behaviour = context.heads_mut();
		match Option::take(&mut self.request) {
			Some(CoHeadsRequest::Subscribe { network }) => match behaviour.subscribe(swarm, &network) {
				Ok(true) => {
					tracing::debug!(co = ?network.id, "co-subscribe");
				},
				Ok(_) => {},
				Err(err) => {
					tracing::warn!(co = ?network.id, ?err, "co-subscribe-failed");
				},
			},
			Some(CoHeadsRequest::Unsubscribe { network }) => match behaviour.unsubscribe(swarm, &network) {
				Ok(true) => {
					tracing::debug!(co = ?network.id, "co-unsubscribe");
				},
				Ok(_) => {},
				Err(err) => {
					tracing::warn!(co = ?network.id, ?err, "co-unsubscribe-failed");
				},
			},
			Some(CoHeadsRequest::PublishHeads { network, heads }) => match behaviour.publish(swarm, &network, &heads) {
				Ok(_) => {
					tracing::debug!(co = ?network.id, "co-publish-heads");
				},
				Err(err) => {
					tracing::warn!(co = ?network.id, ?err, "co-publish-heads-failed");
				},
			},
			None => {},
		}
	}
}
