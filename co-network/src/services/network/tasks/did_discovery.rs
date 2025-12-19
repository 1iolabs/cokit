use crate::{
	network::{Behaviour, Context},
	types::network_task::NetworkTask,
};
use co_identity::PrivateIdentityBox;
use co_primitives::{Did, NetworkDidDiscovery};
use libp2p::Swarm;
use std::fmt::Debug;
use tokio::sync::oneshot;

/// Subscribe identity for DID Discovery.
#[derive(Debug)]
pub struct DidDiscoverySubscribe {
	task: Option<Task>,
}
impl DidDiscoverySubscribe {
	pub fn new(
		identity: PrivateIdentityBox,
		network: Option<NetworkDidDiscovery>,
	) -> (Self, oneshot::Receiver<Result<(), anyhow::Error>>) {
		let (tx, rx) = oneshot::channel();
		(Self { task: Some(Task { subscribe: Some((identity, network)), result: tx }) }, rx)
	}

	pub fn default() -> (Self, oneshot::Receiver<Result<(), anyhow::Error>>) {
		let (tx, rx) = oneshot::channel();
		(Self { task: Some(Task { subscribe: None, result: tx }) }, rx)
	}
}
impl NetworkTask<Behaviour, Context> for DidDiscoverySubscribe {
	fn execute(&mut self, swarm: &mut Swarm<Behaviour>, context: &mut Context) {
		if let Some(Task { subscribe, result }) = Option::take(&mut self.task) {
			let subscribe_result = match subscribe {
				Some((identity, network)) => context.discovery.did_discovery_subscribe(swarm, network, identity),
				None => context.discovery.did_discovery_subscribe_default(swarm),
			};
			result.send(subscribe_result).ok();
		}
	}
}

/// Subscribe identity for DID Discovery.
#[derive(Debug)]
pub struct DidDiscoveryUnsubscribe {
	task: Option<(Option<Did>, oneshot::Sender<Result<(), anyhow::Error>>)>,
}
impl DidDiscoveryUnsubscribe {
	pub fn new(identity: Did) -> (Self, oneshot::Receiver<Result<(), anyhow::Error>>) {
		let (tx, rx) = oneshot::channel();
		(Self { task: Some((Some(identity), tx)) }, rx)
	}

	pub fn default() -> (Self, oneshot::Receiver<Result<(), anyhow::Error>>) {
		let (tx, rx) = oneshot::channel();
		(Self { task: Some((None, tx)) }, rx)
	}
}
impl NetworkTask<Behaviour, Context> for DidDiscoveryUnsubscribe {
	fn execute(&mut self, swarm: &mut Swarm<Behaviour>, context: &mut Context) {
		if let Some((identity, result)) = Option::take(&mut self.task) {
			let unsubscribe_result = match identity {
				Some(identity) => context.discovery.did_discovery_unsubscribe(swarm, &identity),
				None => context.discovery.did_discovery_unsubscribe_default(swarm),
			};
			result.send(unsubscribe_result).ok();
		}
	}
}

#[derive(Debug)]
struct Task {
	subscribe: Option<(PrivateIdentityBox, Option<NetworkDidDiscovery>)>,
	result: oneshot::Sender<Result<(), anyhow::Error>>,
}
