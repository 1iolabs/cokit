// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{
	network::{Behaviour, Context, NetworkEvent},
	services::network::CoNetworkTaskSpawner,
	types::network_task::{NetworkTask, NetworkTaskSpawner},
};
use futures::Stream;
use libp2p::{gossipsub, swarm::SwarmEvent, PeerId, Swarm};
use std::mem::take;

#[derive(Debug, Clone)]
pub struct GossipMessage {
	/// The peer that forwarded us this message.
	_propagation_source: PeerId,

	/// The [`MessageId`] of the message. This should be referenced by the application when
	/// validating a message (if required).
	_message_id: gossipsub::MessageId,

	/// The decompressed message itself.
	message: gossipsub::Message,
}
impl GossipMessage {
	pub fn data(&self) -> &[u8] {
		&self.message.data
	}
}

#[derive(Debug)]
pub struct ListenGossipTask {
	topic: gossipsub::TopicHash,
	messages: tokio::sync::mpsc::UnboundedSender<GossipMessage>,
}
impl ListenGossipTask {
	pub fn subscribe(spawner: CoNetworkTaskSpawner, topic: gossipsub::TopicHash) -> impl Stream<Item = GossipMessage> {
		let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
		let task = Self { topic, messages: tx };
		// spwan (and just shutdown if network is down)
		spawner.spawn(task).ok();
		tokio_stream::wrappers::UnboundedReceiverStream::new(rx)
	}
}
impl NetworkTask<Behaviour, Context> for ListenGossipTask {
	fn execute(&mut self, _swarm: &mut Swarm<Behaviour>, _context: &mut Context) {}

	/// Handle swarm events.
	/// Events can be consumed by this handler or forwarded to next handler.
	fn on_swarm_event(
		&mut self,
		_swarm: &mut Swarm<Behaviour>,
		_context: &mut Context,
		event: SwarmEvent<NetworkEvent>,
	) -> Option<SwarmEvent<NetworkEvent>> {
		if let SwarmEvent::Behaviour(NetworkEvent::Gossipsub(gossipsub::Event::Message {
			propagation_source,
			message_id,
			message,
		})) = &event
		{
			if message.topic == self.topic {
				self.messages
					.send(GossipMessage {
						_propagation_source: *propagation_source,
						_message_id: message_id.clone(),
						message: message.clone(),
					})
					.ok();
			}
		}
		Some(event)
	}

	/// Test if the task is complete and can be removed from the queue.
	/// This will be called only after execute has been called.
	fn is_complete(&mut self) -> bool {
		self.messages.is_closed()
	}
}

#[derive(Debug)]
pub struct PublishGossipTask {
	payload: Option<(
		gossipsub::TopicHash,
		Vec<u8>,
		tokio::sync::oneshot::Sender<Result<gossipsub::MessageId, anyhow::Error>>,
	)>,
}
impl PublishGossipTask {
	pub async fn publish(
		spawner: CoNetworkTaskSpawner,
		topic: gossipsub::TopicHash,
		message: Vec<u8>,
	) -> Result<gossipsub::MessageId, anyhow::Error> {
		let (tx, rx) = tokio::sync::oneshot::channel();
		let task = Self { payload: Some((topic, message, tx)) };
		spawner.spawn(task)?;
		rx.await?
	}
}
impl NetworkTask<Behaviour, Context> for PublishGossipTask {
	fn execute(&mut self, swarm: &mut Swarm<Behaviour>, _context: &mut Context) {
		if let Some((topic, message, result)) = take(&mut self.payload) {
			let publish_result = swarm.behaviour_mut().gossipsub.publish(topic, message);
			result.send(publish_result.map_err(anyhow::Error::from)).ok();
		}
	}
}

#[derive(Debug)]
pub struct SubscribeGossipTask {
	topic: gossipsub::IdentTopic,
	result: Option<tokio::sync::oneshot::Sender<Result<bool, anyhow::Error>>>,
}
impl SubscribeGossipTask {
	pub async fn subscribe(spawner: CoNetworkTaskSpawner, topic: gossipsub::IdentTopic) -> Result<bool, anyhow::Error> {
		let (tx, rx) = tokio::sync::oneshot::channel();
		let task = Self { topic, result: Some(tx) };
		spawner.spawn(task)?;
		rx.await?
	}
}
impl NetworkTask<Behaviour, Context> for SubscribeGossipTask {
	fn execute(&mut self, swarm: &mut Swarm<Behaviour>, _context: &mut Context) {
		if let Some(result) = take(&mut self.result) {
			let subscribe_result = swarm.behaviour_mut().gossipsub.subscribe(&self.topic);
			result.send(subscribe_result.map_err(anyhow::Error::from)).ok();
		}
	}
}

#[derive(Debug)]
pub struct UnsubscribeGossipTask {
	topic: gossipsub::IdentTopic,
	result: Option<tokio::sync::oneshot::Sender<Result<bool, anyhow::Error>>>,
}
impl UnsubscribeGossipTask {
	pub async fn unsubscribe(
		spawner: CoNetworkTaskSpawner,
		topic: gossipsub::IdentTopic,
	) -> Result<bool, anyhow::Error> {
		let (tx, rx) = tokio::sync::oneshot::channel();
		let task = Self { topic, result: Some(tx) };
		spawner.spawn(task)?;
		rx.await?
	}
}
impl NetworkTask<Behaviour, Context> for UnsubscribeGossipTask {
	fn execute(&mut self, swarm: &mut Swarm<Behaviour>, _context: &mut Context) {
		if let Some(result) = take(&mut self.result) {
			let unsubscribe_result = swarm.behaviour_mut().gossipsub.unsubscribe(&self.topic);
			result.send(Ok(unsubscribe_result)).ok();
		}
	}
}
