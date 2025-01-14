use super::to_plain::to_plain;
use crate::{
	reducer::core_resolver::dynamic::DynamicCoreResolver,
	services::{
		connections::ConnectionMessage,
		network::{CoNetworkTaskSpawner, DidCommSendNetworkTask},
	},
	types::message::heads::HeadsMessage,
	CoStorage, Reducer as CoreReducer, ReducerChangeContext, ReducerChangedHandler, TaskSpawner,
};
use anyhow::anyhow;
use async_trait::async_trait;
use cid::Cid;
use co_actor::{Actor, ActorError, ActorHandle, Epic, EpicExt, EpicRuntime, OnceEpic, Reducer, TracingEpic};
use co_identity::{Identity, PrivateIdentityBox};
use co_network::didcomm::EncodedMessage;
use co_primitives::{tags, CoId, Tags};
use co_storage::BlockStorageContentMapping;
use futures::{Stream, StreamExt};
use libp2p::PeerId;
use std::{collections::BTreeSet, future::ready, time::Duration};

///	Use PeerProvider to discover peers and send heads to them whenever a peer comes online or new heads are produced.
pub struct PushHeads<M> {
	handle: ActorHandle<PushHeadsAction>,
	mapping: Option<M>,
	/// Force the mapping to be applied by returning an error when no mapping is found.
	force_mapping: bool,
	initialized: bool,
}
impl<M> PushHeads<M> {
	pub fn new(
		spawner: CoNetworkTaskSpawner,
		connections: ActorHandle<ConnectionMessage>,
		tasks: TaskSpawner,
		co: CoId,
		identity: PrivateIdentityBox,
		mapping: Option<M>,
		force_mapping: bool,
	) -> Result<Self, anyhow::Error> {
		let instance = Actor::spawn_with(
			tasks,
			tags!("type": "co-push-heads", "co": co.as_str()),
			PushHeadsActor { context: PushHeadsContext(spawner, connections, identity) },
			PushHeadsState { co: co.clone(), heads: Default::default() },
		)?;
		Ok(Self { handle: instance.handle(), mapping, force_mapping, initialized: false })
	}
}
#[async_trait]
impl<M> ReducerChangedHandler<CoStorage, DynamicCoreResolver<CoStorage>> for PushHeads<M>
where
	M: BlockStorageContentMapping + Send + Sync + 'static,
{
	async fn on_state_changed(
		&mut self,
		reducer: &CoreReducer<CoStorage, DynamicCoreResolver<CoStorage>>,
		context: ReducerChangeContext,
	) -> Result<(), anyhow::Error> {
		// send local changes
		if context.is_local_change() || !self.initialized {
			self.initialized = true;

			// map plain heads to encrypted heads
			let mut heads = reducer.heads().clone();
			if self.mapping.is_some() {
				heads = to_plain(&self.mapping, self.force_mapping, heads)
					.await
					.map_err(|err| anyhow!("Failed to map head: {}", err))?;
			}

			// send
			self.handle.dispatch(PushHeadsAction::Changed(heads))?;
		}

		// done
		Ok(())
	}
}

struct PushHeadsActor {
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
		epic.handle(handle, &action, &state, &self.context);

		// dispatch
		for next_action in next_actions {
			handle.dispatch(next_action)?;
		}

		// done
		Ok(())
	}
}

#[derive(Debug, Clone)]
struct PushHeadsContext(CoNetworkTaskSpawner, ActorHandle<ConnectionMessage>, PrivateIdentityBox);

#[derive(Debug, Clone)]
#[allow(unused)] // we want to see Sent in the logs
enum PushHeadsAction {
	/// Heads changed.
	Changed(BTreeSet<Cid>),

	/// Connect and send heads to peers.
	Connect(BTreeSet<Cid>),

	/// Send heads to a connected peer.
	Send(BTreeSet<Cid>, BTreeSet<PeerId>),

	/// Sent heads to a connected peer.
	Sent(BTreeSet<Cid>, PeerId, Result<(), String>),
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
			PushHeadsAction::Changed(heads) => {
				if self.heads != heads {
					result.push(PushHeadsAction::Connect(heads.clone()));
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
			PushHeadsAction::Connect(heads) => Some({
				let id = state.co.clone();
				let connections = context.1.clone();
				let from = context.2.identity().to_owned();
				let heads = heads.clone();
				ConnectionMessage::co_use(connections, id, from, [])
					.filter_map(move |changed| {
						ready(match changed {
							Ok(change) if !change.added.is_empty() => Some(change.added),
							_ => None,
						})
					})
					.map(move |peers| PushHeadsAction::Send(heads.clone(), peers))
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
			PushHeadsAction::Send(heads, peers) => Some({
				let id = state.co.clone();
				let network = context.0.clone();
				let identity = context.2.clone();
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
						yield PushHeadsAction::Sent(heads.clone(), peer, match send { Ok(_) => Ok(()), Err(err) => Err(err.to_string()) });
					}
				}
			}),
			_ => None,
		}
	}
}
