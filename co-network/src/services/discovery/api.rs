// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use super::{
	action::{DidSubscribeAction, DidUnsubscribeAction, DiscoveryAction, ReleaseAction},
	actor::DiscoveryActor,
	message::DiscoveryMessage,
	state::{did_discovery_subscription_topic_str, DidDiscoverySubscription},
};
use crate::services::discovery;
use co_actor::{ActorError, ActorHandle, ActorInstance};
use co_identity::network_did_discovery;
use co_primitives::Did;
use futures::Stream;
use std::collections::BTreeSet;

#[derive(Debug, Clone)]
pub struct DiscoveryApi {
	handle: ActorHandle<DiscoveryMessage>,
}
impl From<&ActorInstance<DiscoveryActor>> for DiscoveryApi {
	fn from(value: &ActorInstance<DiscoveryActor>) -> Self {
		Self { handle: value.handle() }
	}
}
impl DiscoveryApi {
	/// Create a closed (disconnected) API handle useful for tests.
	#[cfg(test)]
	pub fn new_closed() -> Self {
		Self { handle: ActorHandle::new_closed() }
	}

	/// Connect peers using discovery. Returns a stream of discovery events.
	pub fn connect(
		&self,
		discovery: BTreeSet<discovery::Discovery>,
	) -> impl Stream<Item = Result<discovery::Event, ActorError>> {
		self.handle
			.clone()
			.stream(|response| DiscoveryMessage::Connect(discovery, response))
	}

	/// Release a discovery request.
	pub fn release(&self, id: u64) {
		self.handle.dispatch(DiscoveryAction::Release(ReleaseAction { id })).ok();
	}

	/// Subscribe identity for DID discovery.
	pub fn did_subscribe(
		&self,
		identity: Option<co_identity::PrivateIdentityBox>,
		network: Option<co_primitives::NetworkDidDiscovery>,
	) -> Result<(), anyhow::Error> {
		let subscription = match identity {
			Some(identity) => {
				let network = network_did_discovery(&identity, network)?;
				DidDiscoverySubscription::Identity(network, identity)
			},
			None => DidDiscoverySubscription::Default,
		};
		let topic_str = did_discovery_subscription_topic_str(&subscription).to_owned();
		self.handle
			.dispatch(DiscoveryAction::DidSubscribe(DidSubscribeAction { subscription, topic_str }))?;
		Ok(())
	}

	/// Unsubscribe identity from DID discovery.
	pub fn did_unsubscribe(&self, did: Option<Did>) -> Result<(), anyhow::Error> {
		let action = match did {
			Some(did) => DidUnsubscribeAction::Identity(did),
			None => DidUnsubscribeAction::Default,
		};
		self.handle.dispatch(DiscoveryAction::DidUnsubscribe(action))?;
		Ok(())
	}
}
