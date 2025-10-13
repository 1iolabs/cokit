use crate::services::heads::{
	action::UnsubscribeAction,
	actor::{to_topic, Message},
	Heads, HeadsActor, PublishAction, ReceiveAction, SubscribeAction,
};
use co_actor::{ActorHandle, ActorInstance};
use co_primitives::NetworkCoHeads;
use futures::Stream;
use libp2p::gossipsub;

#[derive(Debug, Clone)]
pub struct HeadsApi {
	handle: ActorHandle<Message>,
}
impl From<&ActorInstance<HeadsActor>> for HeadsApi {
	fn from(value: &ActorInstance<HeadsActor>) -> Self {
		Self { handle: value.handle() }
	}
}
impl HeadsApi {
	pub fn subscribe(&self, network: NetworkCoHeads) -> Result<(), anyhow::Error> {
		let action = SubscribeAction { network };
		self.handle.dispatch(Message::Action(action.into()))?;
		Ok(())
	}

	pub fn unsubscribe(&self, network: NetworkCoHeads) -> Result<(), anyhow::Error> {
		let action = UnsubscribeAction { network };
		self.handle.dispatch(Message::Action(action.into()))?;
		Ok(())
	}

	pub fn publish(&self, network: NetworkCoHeads, heads: Heads) -> Result<(), anyhow::Error> {
		let action = PublishAction { network, heads };
		self.handle.dispatch(Message::Action(action.into()))?;
		Ok(())
	}

	pub fn heads(&self) -> impl Stream<Item = ReceiveAction> {
		self.handle.stream_graceful(Message::Receive)
	}

	pub fn to_topic_hash(network: &NetworkCoHeads) -> gossipsub::TopicHash {
		to_topic(network).hash()
	}

	pub fn to_topic(network: &NetworkCoHeads) -> gossipsub::IdentTopic {
		to_topic(network)
	}
}
