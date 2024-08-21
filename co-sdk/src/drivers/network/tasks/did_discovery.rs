use co_identity::{IdentityResolverBox, PrivateIdentity};
use co_network::{discovery, DiscoveryLayerBehaviourProvider, NetworkTask};
use co_primitives::{Did, NetworkDidDiscovery};
use libp2p::{swarm::NetworkBehaviour, Swarm};
use tokio::sync::oneshot;

/// Subscribe identity for DID Discovery.
pub struct DidDiscoverySubscribe<I> {
	task: Option<(I, Option<NetworkDidDiscovery>, oneshot::Sender<Result<(), anyhow::Error>>)>,
}
impl<I> DidDiscoverySubscribe<I>
where
	I: PrivateIdentity + Clone + Send + Sync + 'static,
{
	pub fn new(
		identity: I,
		network: Option<NetworkDidDiscovery>,
	) -> (Self, oneshot::Receiver<Result<(), anyhow::Error>>) {
		let (tx, rx) = oneshot::channel();
		(Self { task: Some((identity, network, tx)) }, rx)
	}
}
impl<B, C, I> NetworkTask<B, C> for DidDiscoverySubscribe<I>
where
	B: NetworkBehaviour + discovery::DiscoveryBehaviour,
	C: DiscoveryLayerBehaviourProvider<IdentityResolverBox, Event = <B as NetworkBehaviour>::ToSwarm>,
	I: PrivateIdentity + Clone + Send + Sync + 'static,
{
	fn execute(&mut self, swarm: &mut Swarm<B>, context: &mut C) {
		if let Some((identity, network, result)) = Option::take(&mut self.task) {
			result
				.send(context.discovery_mut().did_discovery_subscribe(swarm, network, identity))
				.ok();
		}
	}
}

/// Subscribe identity for DID Discovery.
pub struct DidDiscoveryUnsubscribe {
	task: Option<(Did, oneshot::Sender<Result<(), anyhow::Error>>)>,
}
impl DidDiscoveryUnsubscribe {
	pub fn new(identity: Did) -> (Self, oneshot::Receiver<Result<(), anyhow::Error>>) {
		let (tx, rx) = oneshot::channel();
		(Self { task: Some((identity, tx)) }, rx)
	}
}
impl<B, C> NetworkTask<B, C> for DidDiscoveryUnsubscribe
where
	B: NetworkBehaviour + discovery::DiscoveryBehaviour,
	C: DiscoveryLayerBehaviourProvider<IdentityResolverBox, Event = <B as NetworkBehaviour>::ToSwarm>,
{
	fn execute(&mut self, swarm: &mut Swarm<B>, context: &mut C) {
		if let Some((identity, result)) = Option::take(&mut self.task) {
			result
				.send(context.discovery_mut().did_discovery_unsubscribe(swarm, &identity))
				.ok();
		}
	}
}
