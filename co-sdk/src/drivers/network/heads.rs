use crate::CoReducerFactory;
use co_network::{heads, HeadsBehaviourProvider, NetworkTask};
use co_primitives::CoId;
use libipld::Cid;
use libp2p::{
	swarm::{NetworkBehaviour, SwarmEvent},
	PeerId, Swarm,
};
use std::collections::BTreeSet;

/// Handle received heads from the network within the application.
/// This structure essentially joins the received heads into the repective co reducer.
pub struct ReceivedHeadsNetworkTask<F> {
	co_factory: F,
}
impl<F> ReceivedHeadsNetworkTask<F> {
	pub fn new(co_factory: F) -> Self {
		Self { co_factory }
	}
}
impl<F, B> NetworkTask<B> for ReceivedHeadsNetworkTask<F>
where
	F: CoReducerFactory + Clone + Send + Sync + 'static,
	B: NetworkBehaviour + HeadsBehaviourProvider<Event = B::ToSwarm>,
{
	fn execute(&mut self, _swarm: &mut Swarm<B>) {}

	fn on_swarm_event(
		&mut self,
		_swarm: &mut Swarm<B>,
		event: SwarmEvent<B::ToSwarm>,
	) -> Option<SwarmEvent<B::ToSwarm>> {
		// handle
		match B::heads_event(&event) {
			Some(heads::Event::ReceivedHeads { co, heads, peer_id, response }) => {
				let co_id = co.to_owned();
				let heads = heads.to_owned();
				let co_factory = self.co_factory.clone();
				let peer_id = peer_id.clone();
				let response = *response;
				tokio::spawn(async move {
					match co_factory.co_reducer(&co_id).await {
						Ok(Some(co)) => match co.join(heads).await {
							Ok(update) => {
								tracing::debug!(co = ?co_id, update, "co-heads");

								// send response?
								if response && peer_id.is_some() {}
							},
							Err(err) => {
								tracing::warn!(co = ?co_id, ?err, reason = "join", "co-heads-failure");
							},
						},
						Ok(None) => {
							tracing::warn!(co = ?co_id, reason = "not-found", "co-heads-failure");
						},
						Err(err) => {
							tracing::warn!(co = ?co_id, ?err, reason = "factory", "co-heads-failure");
						},
					};
				});
			},
			_ => {},
		}

		// forward
		Some(event)
	}

	fn is_complete(&mut self) -> bool {
		false
	}
}

#[derive(Debug)]
pub enum HeadsRequest {
	Subscribe { co: CoId },
	Unsubscribe { co: CoId },
	Heads { co: CoId, heads: BTreeSet<Cid>, peers: BTreeSet<PeerId> },
	PublishHeads { co: CoId, heads: BTreeSet<Cid> },
}

pub struct HeadsRequestNetworkTask {
	request: Option<HeadsRequest>,
}
impl HeadsRequestNetworkTask {
	pub fn new(request: HeadsRequest) -> Self {
		Self { request: Some(request) }
	}
}
impl<B> NetworkTask<B> for HeadsRequestNetworkTask
where
	B: NetworkBehaviour + HeadsBehaviourProvider<Event = B::ToSwarm>,
{
	fn execute(&mut self, swarm: &mut Swarm<B>) {
		let behaviour = swarm.behaviour_mut().heads_mut();
		match Option::take(&mut self.request) {
			Some(HeadsRequest::Subscribe { co }) => match behaviour.subscribe(&co) {
				Ok(true) => {
					tracing::debug!(?co, "co-subscribe");
				},
				Ok(_) => {},
				Err(err) => {
					tracing::warn!(?co, ?err, "co-subscribe-failed");
				},
			},
			Some(HeadsRequest::Unsubscribe { co }) => match behaviour.unsubscribe(&co) {
				Ok(true) => {
					tracing::debug!(?co, "co-unsubscribe");
				},
				Ok(_) => {},
				Err(err) => {
					tracing::warn!(?co, ?err, "co-unsubscribe-failed");
				},
			},
			Some(HeadsRequest::Heads { co, heads, peers }) => match behaviour.heads(&co, heads, peers.into_iter()) {
				Ok(_) => {
					tracing::debug!(?co, "co-request-heads");
				},
				Err(err) => {
					tracing::warn!(?co, ?err, "co-request-heads-failed");
				},
			},
			Some(HeadsRequest::PublishHeads { co, heads }) => match behaviour.publish_heads(&co, heads) {
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
