use super::to_external_cid::{to_external_cids, to_external_cids_opt_force};
use crate::{
	services::{
		connections::ConnectionMessage,
		network::{CoNetworkTaskSpawner, DidCommSendNetworkTask},
	},
	types::message::heads::HeadsMessage,
	CoReducerState, CoStorage, TaskSpawner,
};
use anyhow::anyhow;
use async_trait::async_trait;
use cid::Cid;
use co_actor::{Actor, ActorError, ActorHandle, Epic, EpicExt, EpicRuntime, OnceEpic, Reducer, TracingEpic};
use co_identity::{Identity, PrivateIdentity, PrivateIdentityBox};
use co_network::didcomm::EncodedMessage;
use co_primitives::{tags, CoId, Tags};
use futures::{Stream, StreamExt};
use libp2p::PeerId;
use std::{collections::BTreeSet, future::ready, time::Duration};

///	Use PeerProvider to discover peers and send heads to them whenever a peer comes online or new heads are produced.
#[derive(Debug, Clone)]
pub struct PushHeads {
	handle: ActorHandle<PushHeadsAction>,
	/// Force the mapping to be applied by returning an error when no mapping is found.
	force_mapping: bool,
}
impl PushHeads {
	pub fn new(
		spawner: CoNetworkTaskSpawner,
		connections: ActorHandle<ConnectionMessage>,
		tasks: TaskSpawner,
		co: CoId,
		force_mapping: bool,
	) -> Result<Self, anyhow::Error> {
		let instance = Actor::spawn_with(
			tasks.clone(),
			tags!("type": "co-push-heads", "co": co.as_str()),
			PushHeadsActor { tasks, context: PushHeadsContext(spawner, connections) },
			PushHeadsState { co: co.clone(), heads: Default::default() },
		)?;
		Ok(Self { handle: instance.handle(), force_mapping })
	}

	pub async fn changed(
		&self,
		storage: &CoStorage,
		state: CoReducerState,
		identity: PrivateIdentityBox,
	) -> Result<(), anyhow::Error> {
		// verify
		identity.try_didcomm_private()?;

		// map plain heads to encrypted heads
		let heads = if self.force_mapping {
			to_external_cids_opt_force(storage, state.heads())
				.await
				.ok_or_else(|| anyhow!("Failed to map heads: {:?}", state.heads()))?
		} else {
			to_external_cids(storage, state.heads()).await
		};

		// send
		self.handle.dispatch(PushHeadsAction::Changed(identity, heads))?;
		Ok(())
	}
}

struct PushHeadsActor {
	tasks: TaskSpawner,
	context: PushHeadsContext,
}
#[async_trait]
impl Actor for PushHeadsActor {
	type Message = PushHeadsAction;
	type State = (PushHeadsState, EpicRuntime<PushHeadsAction, PushHeadsAction, PushHeadsState, PushHeadsContext>);
	type Initialize = PushHeadsState;

	async fn initialize(
		&self,
		_handle: &ActorHandle<Self::Message>,
		tags: &Tags,
		initialize: Self::Initialize,
	) -> Result<Self::State, ActorError> {
		let co = initialize.co.clone();
		Ok((
			initialize,
			EpicRuntime::new(
				PushHeadsSendEpic::new()
					.join(PushHeadsConnectEpic::new())
					.join(TracingEpic::new(tags.clone())),
				move |err| {
					tracing::error!(?err, ?co, "push-heads-error");
					None
				},
			),
		))
	}

	async fn handle(
		&self,
		handle: &ActorHandle<Self::Message>,
		action: Self::Message,
		(state, epic): &mut Self::State,
	) -> Result<(), ActorError> {
		let next_actions = state.reduce(action.clone());

		// epic
		epic.handle(&self.tasks, handle, &action, &state, &self.context);

		// dispatch
		for next_action in next_actions {
			handle.dispatch(next_action)?;
		}

		// done
		Ok(())
	}
}

#[derive(Debug, Clone)]
struct PushHeadsContext(CoNetworkTaskSpawner, ActorHandle<ConnectionMessage>);

#[derive(Debug, Clone)]
#[allow(unused)] // we want to see Sent in the logs
enum PushHeadsAction {
	/// Heads changed.
	Changed(PrivateIdentityBox, BTreeSet<Cid>),

	/// Connect and send heads to peers.
	Connect(PrivateIdentityBox, BTreeSet<Cid>),

	/// Send heads to a connected peer.
	Send(PrivateIdentityBox, BTreeSet<Cid>, BTreeSet<PeerId>),

	/// Sent heads to a connected peer.
	Sent(PrivateIdentityBox, BTreeSet<Cid>, PeerId, Result<(), String>),
}

#[derive(Debug, Clone)]
struct PushHeadsState {
	pub co: CoId,
	pub heads: BTreeSet<Cid>,
}
impl Reducer<PushHeadsAction> for PushHeadsState {
	fn reduce(&mut self, action: PushHeadsAction) -> Vec<PushHeadsAction> {
		let mut result = vec![];
		match action {
			PushHeadsAction::Changed(identity, heads) => {
				if self.heads != heads {
					result.push(PushHeadsAction::Connect(identity, heads.clone()));
					self.heads = heads;
				}
			},
			_ => {},
		}
		result
	}
}

/// Use Send actions, connect the CO and return SendPeers actions.
struct PushHeadsConnectEpic();
impl PushHeadsConnectEpic {
	pub fn new() -> OnceEpic<Self> {
		Self().once()
	}
}
impl Epic<PushHeadsAction, PushHeadsState, PushHeadsContext> for PushHeadsConnectEpic {
	fn epic(
		&mut self,
		action: &PushHeadsAction,
		state: &PushHeadsState,
		context: &PushHeadsContext,
	) -> Option<impl Stream<Item = Result<PushHeadsAction, anyhow::Error>> + Send + 'static> {
		match action {
			PushHeadsAction::Connect(identity, heads) => Some({
				let id = state.co.clone();
				let connections = context.1.clone();
				let identity = identity.clone();
				let from = identity.identity().to_owned();
				let heads = heads.clone();
				ConnectionMessage::co_use(connections, id, from, [])
					.filter_map(move |changed| {
						ready(match changed {
							Ok(change) if !change.added.is_empty() => Some(change.added),
							_ => None,
						})
					})
					.map(move |peers| PushHeadsAction::Send(identity.clone(), heads.clone(), peers))
					// .flat_map(move |peers| {
					// 	stream::iter(peers.into_iter().map({
					// 		let heads = heads.clone();
					// 		move |peer| PushHeadsAction::Send(heads.clone(), peer)
					// 	}))
					// })
					.map(Ok)
			}),
			_ => None,
		}
	}
}

/// Send heads to peers.
struct PushHeadsSendEpic();
impl PushHeadsSendEpic {
	pub fn new() -> Self {
		Self()
	}
}
impl Epic<PushHeadsAction, PushHeadsState, PushHeadsContext> for PushHeadsSendEpic {
	fn epic(
		&mut self,
		action: &PushHeadsAction,
		state: &PushHeadsState,
		context: &PushHeadsContext,
	) -> Option<impl Stream<Item = Result<PushHeadsAction, anyhow::Error>> + Send + 'static> {
		match action {
			PushHeadsAction::Send(identity, heads, peers) => Some({
				let id = state.co.clone();
				let network = context.0.clone();
				let identity = identity.clone();
				let heads = heads.clone();
				let peers = peers.clone();
				async_stream::try_stream! {
					// message
					let header = HeadsMessage::create_header();
					let body = HeadsMessage::Heads(id.clone(), heads.clone());
					let (_, message) = EncodedMessage::create_signed_json(&identity, header, &body)?;

					// send
					for peer in peers {
						let send = DidCommSendNetworkTask::send(network.clone(), [peer], message.clone(), Duration::from_secs(30)).await;
						yield PushHeadsAction::Sent(identity.clone(), heads.clone(), peer, match send { Ok(_) => Ok(()), Err(err) => Err(err.to_string()) });
					}
				}
			}),
			_ => None,
		}
	}
}
