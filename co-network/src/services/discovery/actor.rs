// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use super::{
	action::{ConnectAction, DiscoveryAction},
	epics::epic,
	message::DiscoveryMessage,
	state::DiscoveryState,
};
use crate::services::{discovery, network::CoNetworkTaskSpawner};
use async_trait::async_trait;
use co_actor::{Actor, ActorError, ActorHandle, EpicRuntime, Reducer, ResponseStreams, TaskSpawner};
use co_identity::IdentityResolverBox;
use co_primitives::{DynamicCoDate, Tags};
use libp2p::PeerId;
use std::{collections::BTreeMap, time::Duration};

#[derive(Debug, Clone)]
pub struct DiscoveryContext {
	pub tasks: TaskSpawner,
	pub network: CoNetworkTaskSpawner,
	pub date: DynamicCoDate,
	pub resolver: IdentityResolverBox,
	pub local_peer_id: PeerId,
}

pub struct State {
	state: DiscoveryState,
	epic: EpicRuntime<DiscoveryMessage, DiscoveryAction, DiscoveryState, DiscoveryContext>,
	streams: BTreeMap<u64, ResponseStreams<discovery::Event>>,
}

pub struct DiscoveryActor {
	context: DiscoveryContext,
}
impl DiscoveryActor {
	pub fn new(context: DiscoveryContext) -> Self {
		Self { context }
	}
}

#[async_trait]
impl Actor for DiscoveryActor {
	type Message = DiscoveryMessage;
	type State = State;
	type Initialize = ();

	async fn initialize(
		&self,
		_handle: &ActorHandle<Self::Message>,
		tags: &Tags,
		_initialize: Self::Initialize,
	) -> Result<Self::State, ActorError> {
		Ok(State {
			state: DiscoveryState {
				local_peer_id: self.context.local_peer_id,
				next_id: 1,
				requests: Default::default(),
				did_subscriptions: Default::default(),
				pending_discovery: Default::default(),
				timeout: Duration::from_secs(30),
				max_peers: None,
				connected_peers: Default::default(),
				did_peer_cache: Default::default(),
			},
			epic: EpicRuntime::new(epic(tags.clone()), |err| {
				tracing::error!(?err, "discovery-epic-error");
				None
			}),
			streams: Default::default(),
		})
	}

	async fn handle(
		&self,
		handle: &ActorHandle<Self::Message>,
		message: Self::Message,
		state: &mut Self::State,
	) -> Result<(), ActorError> {
		let (action, response) = match message {
			DiscoveryMessage::Connect(discovery, response) => {
				let id = state.state.allocate_id();
				let action = DiscoveryAction::Connect(ConnectAction { id, discovery });
				(action, Some((id, response)))
			},
			DiscoveryMessage::Action(action) => (action, None),
		};

		// reduce
		let next_actions = state.state.reduce(action.clone());

		// register response stream
		if let Some((id, response)) = response {
			state.streams.entry(id).or_default().push(response);
		}

		// epic
		state
			.epic
			.handle(&self.context.tasks, handle, &action, &state.state, &self.context);

		// streams
		dispatch_events(&action, &mut state.streams, &mut state.state);

		// actions
		for next_action in next_actions {
			handle.dispatch(next_action)?;
		}

		Ok(())
	}
}

fn dispatch_events(
	action: &DiscoveryAction,
	streams: &mut BTreeMap<u64, ResponseStreams<discovery::Event>>,
	_state: &mut DiscoveryState,
) {
	match action {
		DiscoveryAction::Event(event) => {
			let id = event_id(event);
			if let Some(s) = streams.get_mut(&id) {
				s.send(event.clone());
			}
			// terminal events — close the stream.
			if matches!(event, discovery::Event::Failed { .. }) {
				streams.remove(&id);
			}
		},
		DiscoveryAction::Release(release) => {
			streams.remove(&release.id);
		},
		DiscoveryAction::Timeout(timeout) => {
			if let Some(s) = streams.get_mut(&timeout.id) {
				s.send(discovery::Event::Timeout { id: timeout.id });
			}
			streams.remove(&timeout.id);
		},
		_ => {},
	}
}

fn event_id(event: &discovery::Event) -> u64 {
	match event {
		discovery::Event::Connected { id, .. } => *id,
		discovery::Event::Disconnected { id, .. } => *id,
		discovery::Event::InsufficentPeers { id } => *id,
		discovery::Event::Failed { id } => *id,
		discovery::Event::Timeout { id } => *id,
	}
}
