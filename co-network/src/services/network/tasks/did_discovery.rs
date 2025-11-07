use crate::{
	network::{Behaviour, Context},
	types::network_task::NetworkTask,
};
use co_identity::PrivateIdentity;
use co_primitives::{Did, NetworkDidDiscovery};
use libp2p::Swarm;
use std::fmt::Debug;
use tokio::sync::oneshot;

/// Subscribe identity for DID Discovery.
#[derive(Debug)]
pub struct DidDiscoverySubscribe<I: Debug> {
	task: Option<(I, Option<NetworkDidDiscovery>, oneshot::Sender<Result<(), anyhow::Error>>)>,
}
impl<I> DidDiscoverySubscribe<I>
where
	I: PrivateIdentity + Debug + Clone + Send + Sync + 'static,
{
	pub fn new(
		identity: I,
		network: Option<NetworkDidDiscovery>,
	) -> (Self, oneshot::Receiver<Result<(), anyhow::Error>>) {
		let (tx, rx) = oneshot::channel();
		(Self { task: Some((identity, network, tx)) }, rx)
	}
}
impl<I> NetworkTask<Behaviour, Context> for DidDiscoverySubscribe<I>
where
	I: PrivateIdentity + Debug + Clone + Send + Sync + 'static,
{
	fn execute(&mut self, swarm: &mut Swarm<Behaviour>, context: &mut Context) {
		if let Some((identity, network, result)) = Option::take(&mut self.task) {
			result
				.send(context.discovery.did_discovery_subscribe(swarm, network, identity))
				.ok();
		}
	}
}

/// Subscribe identity for DID Discovery.
#[derive(Debug)]
pub struct DidDiscoveryUnsubscribe {
	task: Option<(Did, oneshot::Sender<Result<(), anyhow::Error>>)>,
}
impl DidDiscoveryUnsubscribe {
	pub fn new(identity: Did) -> (Self, oneshot::Receiver<Result<(), anyhow::Error>>) {
		let (tx, rx) = oneshot::channel();
		(Self { task: Some((identity, tx)) }, rx)
	}
}
impl NetworkTask<Behaviour, Context> for DidDiscoveryUnsubscribe {
	fn execute(&mut self, swarm: &mut Swarm<Behaviour>, context: &mut Context) {
		if let Some((identity, result)) = Option::take(&mut self.task) {
			result.send(context.discovery.did_discovery_unsubscribe(swarm, &identity)).ok();
		}
	}
}
