use crate::CoReducerFactory;
use co_network::{heads, HeadsLayerBehaviourProvider, NetworkTask};
use libp2p::{
	swarm::{NetworkBehaviour, SwarmEvent},
	Swarm,
};

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
impl<F, B, C> NetworkTask<B, C> for ReceivedHeadsNetworkTask<F>
where
	F: CoReducerFactory + Clone + Send + Sync + 'static,
	B: NetworkBehaviour,
	C: HeadsLayerBehaviourProvider<Event = <B as NetworkBehaviour>::ToSwarm>,
{
	fn execute(&mut self, _swarm: &mut Swarm<B>, _context: &mut C) {}

	fn on_swarm_event(
		&mut self,
		_swarm: &mut Swarm<B>,
		_context: &mut C,
		event: SwarmEvent<B::ToSwarm>,
	) -> Option<SwarmEvent<B::ToSwarm>> {
		// handle
		match &event {
			SwarmEvent::Behaviour(behaviour_event) => {
				match C::heads_event(behaviour_event) {
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
