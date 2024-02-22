use super::CoNetworkTaskSpawner;
use crate::CoReducer;
use co_network::{GossipsubBehaviourProvider, Heads, HeadsHandler, NetworkTask};
use libipld::Cid;
use libp2p::{
	gossipsub::IdentTopic,
	swarm::{NetworkBehaviour, SwarmEvent},
	PeerId, Swarm,
};
use std::collections::BTreeSet;
use tokio::sync::oneshot::{self, error::TryRecvError};

/// Subscription for a single CO (`CoReducer`).
pub struct Subscription {
	spawner: CoNetworkTaskSpawner,
	co: CoReducer,
	shutdown: oneshot::Sender<()>,
}
impl Subscription {
	pub(crate) async fn subscribe(spawner: CoNetworkTaskSpawner, co: CoReducer) -> Result<Self, anyhow::Error> {
		let (tx, rx) = oneshot::channel();
		// let state: co_core_co::Co = co.state(CO_CORE_NAME_CO).await?;
		let (_, heads) = co.reducer_state().await;
		let heads = Heads::new(IdentTopic::new(co.id()), heads, SubscriptionHandler { co: co.clone() });
		spawner.spawn(NetworkSubscription { heads, shutdown: rx })?;
		Ok(Self { shutdown: tx, spawner, co })
	}

	pub fn unsubscribe(self) {
		self.shutdown.send(()).ok();
	}
}

struct SubscriptionHandler {
	co: CoReducer,
}
impl HeadsHandler for SubscriptionHandler {
	fn on_heads(&mut self, heads: BTreeSet<Cid>) {
		let co = self.co.clone();
		tokio::spawn(async move {
			match co.join(heads).await {
				Ok(update) => {
					tracing::debug!(update, "co-subscription");
				},
				Err(err) => {
					tracing::warn!(?err, "co-subscription-failure");
				},
			}
		});
	}

	fn on_subscribe(&mut self, _peer: PeerId) {}

	fn on_unsubscribe(&mut self, _peer: PeerId) {}
}

fn topic(co: &co_core_co::Co) -> IdentTopic {
	IdentTopic::new(&co.id)
}

struct NetworkSubscription {
	heads: Heads,
	shutdown: oneshot::Receiver<()>,
}
impl<B> NetworkTask<B> for NetworkSubscription
where
	B: NetworkBehaviour + GossipsubBehaviourProvider<Event = B::ToSwarm>,
{
	fn execute(&mut self, swarm: &mut Swarm<B>) {
		let gossipsub = swarm.behaviour_mut().gossipsub_mut();
		match self.heads.subscribe(gossipsub) {
			Ok(_) => {},
			Err(err) => tracing::warn!(?err, "subscription-failed"),
		};
	}

	fn on_swarm_event(
		&mut self,
		_swarm: &mut Swarm<B>,
		event: SwarmEvent<B::ToSwarm>,
	) -> Option<SwarmEvent<B::ToSwarm>> {
		match B::handle_event(event, |e| self.heads.is_our_event(e)) {
			Ok(gossip) => {
				self.heads.handle_swarm_event(gossip);
				None
			},
			Err(event) => Some(event),
		}
	}

	fn is_complete(&mut self) -> bool {
		match self.shutdown.try_recv() {
			Ok(_) | Err(TryRecvError::Closed) => true,
			Err(TryRecvError::Empty) => false,
		}
	}
}
