use crate::CoNetworkTaskSpawner;
use co_network::{GossipsubBehaviourProvider, NetworkTask, NetworkTaskSpawner};
use futures::Stream;
use libp2p::{
	gossipsub,
	swarm::{NetworkBehaviour, SwarmEvent},
	PeerId, Swarm,
};
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
impl<B, C> NetworkTask<B, C> for ListenGossipTask
where
	B: NetworkBehaviour + GossipsubBehaviourProvider,
{
	fn execute(&mut self, _swarm: &mut Swarm<B>, _context: &mut C) {}

	/// Handle swarm events.
	/// Events can be consumed by this handler or forwarded to next handler.
	fn on_swarm_event(
		&mut self,
		_swarm: &mut Swarm<B>,
		_context: &mut C,
		event: SwarmEvent<B::ToSwarm>,
	) -> Option<SwarmEvent<B::ToSwarm>> {
		match &event {
			SwarmEvent::Behaviour(behaviour_event) => match B::gossipsub_event(&behaviour_event) {
				Some(gossipsub::Event::Message { propagation_source, message_id, message }) => {
					if message.topic == self.topic {
						self.messages
							.send(GossipMessage {
								_propagation_source: propagation_source.clone(),
								_message_id: message_id.clone(),
								message: message.clone(),
							})
							.ok();
					}
				},
				_ => {},
			},
			_ => {},
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
		Ok(rx.await??)
	}
}
impl<B, C> NetworkTask<B, C> for PublishGossipTask
where
	B: NetworkBehaviour + GossipsubBehaviourProvider,
{
	fn execute(&mut self, swarm: &mut Swarm<B>, _context: &mut C) {
		if let Some((topic, message, result)) = take(&mut self.payload) {
			let publish_result = swarm.behaviour_mut().gossipsub_mut().publish(topic, message);
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
		Ok(rx.await??)
	}
}
impl<B, C> NetworkTask<B, C> for SubscribeGossipTask
where
	B: NetworkBehaviour + GossipsubBehaviourProvider,
{
	fn execute(&mut self, swarm: &mut Swarm<B>, _context: &mut C) {
		if let Some(result) = take(&mut self.result) {
			let subscribe_result = swarm.behaviour_mut().gossipsub_mut().subscribe(&self.topic);
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
		Ok(rx.await??)
	}
}
impl<B, C> NetworkTask<B, C> for UnsubscribeGossipTask
where
	B: NetworkBehaviour + GossipsubBehaviourProvider,
{
	fn execute(&mut self, swarm: &mut Swarm<B>, _context: &mut C) {
		if let Some(result) = take(&mut self.result) {
			let subscribe_result = swarm.behaviour_mut().gossipsub_mut().unsubscribe(&self.topic);
			result.send(subscribe_result.map_err(anyhow::Error::from)).ok();
		}
	}
}
