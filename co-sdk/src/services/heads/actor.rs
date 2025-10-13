use crate::{
	services::heads::{action::UnsubscribeAction, epics::epic, HeadsAction, ReceiveAction, SubscribeAction},
	CoNetworkTaskSpawner,
};
use async_trait::async_trait;
use co_actor::{Actor, ActorError, ActorHandle, EpicRuntime, ResponseStream, ResponseStreams, TaskSpawner};
use co_primitives::{NetworkCoHeads, Tags};
use libp2p::gossipsub;
use std::collections::BTreeMap;

#[derive(Debug)]
pub struct State {
	state: HeadsState,
	epic: EpicRuntime<Message, HeadsAction, HeadsState, HeadsContext>,
	receive: ResponseStreams<ReceiveAction>,
	context: HeadsContext,
}

#[derive(Debug, Clone)]
pub struct HeadsContext {
	pub spawner: TaskSpawner,
	pub network: CoNetworkTaskSpawner,
}

#[derive(Debug, Default)]
pub struct HeadsState {
	/// Subscribed topics.
	pub heads: BTreeMap<gossipsub::TopicHash, Vec<NetworkCoHeads>>,
}

#[derive(Debug)]
pub enum Message {
	/// Action.
	Action(HeadsAction),

	/// Subscribe to head changes.
	Receive(ResponseStream<ReceiveAction>),
}
impl From<HeadsAction> for Message {
	fn from(value: HeadsAction) -> Self {
		Self::Action(value)
	}
}

/// Handle CoHeads networks and connect them to network GossipSub.
#[derive(Debug, Default)]
pub struct HeadsActor {}
#[async_trait]
impl Actor for HeadsActor {
	type Message = Message;
	type State = State;
	type Initialize = HeadsContext;

	async fn initialize(
		&self,
		_handle: &ActorHandle<Self::Message>,
		tags: &Tags,
		context: Self::Initialize,
	) -> Result<Self::State, ActorError> {
		Ok(State {
			epic: EpicRuntime::new(epic(tags.clone()), |err| {
				tracing::error!(?err, "heads-epic-error");
				None
			}),
			state: Default::default(),
			receive: Default::default(),
			context,
		})
	}

	async fn handle(
		&self,
		handle: &ActorHandle<Self::Message>,
		message: Self::Message,
		state: &mut Self::State,
	) -> Result<(), ActorError> {
		// reduce
		match &message {
			Message::Action(HeadsAction::Subscribe(action)) => handle_subscribe(&mut state.state, action),
			Message::Action(HeadsAction::Unsubscribe(action)) => handle_unsubscribe(&mut state.state, action),
			_ => {},
		}

		// epic
		if let Message::Action(action) = &message {
			state
				.epic
				.handle(&state.context.spawner, handle, action, &state.state, &state.context);
		}

		// handle
		match message {
			Message::Action(HeadsAction::Receive(action)) => state.receive.send(action.clone()),
			Message::Receive(response) => state.receive.push(response),
			_ => {},
		}

		// result
		Ok(())
	}
}

fn handle_subscribe(state: &mut HeadsState, action: &SubscribeAction) {
	let hash = to_topic_hash(&action.network);
	state.heads.entry(hash).or_default().push(action.network.clone());
}

fn handle_unsubscribe(state: &mut HeadsState, action: &UnsubscribeAction) {
	let hash = to_topic_hash(&action.network);

	// remove network
	let remove = if let Some(networks) = state.heads.get_mut(&hash) {
		if let Some(index) = networks.iter().position(|i| i == &action.network) {
			networks.remove(index);
		}
		networks.is_empty()
	} else {
		false
	};

	// remove empty subscriptions
	if remove {
		state.heads.remove(&hash);
	}
}

pub fn to_topic_hash(network: &NetworkCoHeads) -> gossipsub::TopicHash {
	to_topic(network).hash()
}

pub fn to_topic(network: &NetworkCoHeads) -> gossipsub::IdentTopic {
	gossipsub::IdentTopic::new(network.topic.clone().unwrap_or_else(|| format!("co-{}", network.id)))
}
