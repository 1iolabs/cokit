// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::{
	library::{
		network_identity::network_identity, prepared_join::join_prepared_states, shared_membership::shared_membership,
	},
	services::application::{
		HeadsError, HeadsMessageReceivedAction, HeadsMessageWorkAction, HeadsMessageWorkKind, PreparedHeadsMessage,
	},
	state, Action, ActionError, CoContext, CoReducer, CoReducerFactory, MappedCoReducerState,
};
use anyhow::anyhow;
use cid::Cid;
use co_actor::{time, ActionDispatch, Actions, Epic};
use co_core_membership::MembershipState;
use co_identity::PeerDidCommHeader;
use co_network::{connections::PeerRelateCoAction, EncodedMessage, HeadsErrorCode, HeadsMessage, PeerId};
use co_primitives::{CoId, Did};
use futures::{future::ready, stream, FutureExt, Stream, StreamExt};
use std::{
	collections::{btree_map::Entry, BTreeMap, BTreeSet},
	str::FromStr,
};

/// Receive [`HeadsMessage`] DIDComm message.
///
/// In: [`Action::DidCommReceive`]
/// Out: [`Action::HeadsMessageReceived`]
pub fn heads_message_receive(
	_actions: &Actions<Action, (), CoContext>,
	action: &Action,
	_state: &(),
	_context: &CoContext,
) -> Option<impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static> {
	let message_type = HeadsMessage::message_type();
	let result = match action {
		Action::DidCommReceive { peer, message } => {
			if message.header().message_type == message_type {
				let heads_message: Option<HeadsMessage> = message.body_deserialize().ok();
				if let Some(heads_message) = heads_message {
					let header = PeerDidCommHeader::from(message.header().clone());
					let from_peer = header.from_peer_id.and_then(|s| PeerId::from_str(&s).ok());
					Some((message.sender().cloned(), *peer, from_peer, message.header().id.clone(), heads_message))
				} else {
					None
				}
			} else {
				None
			}
		},
		_ => None,
	}
	.map(|(from, peer, from_peer, message_id, message)| {
		Action::HeadsMessageReceived(HeadsMessageReceivedAction {
			co: message.co().clone(),
			from,
			from_peer,
			peer,
			message_id,
			message,
			tags: Default::default(),
		})
	})?;
	Some(stream::once(ready(Ok(result))))
}

/// Update a CO from independently prepared [`HeadsMessage::Heads`] messages.
/// Only ready work is serialized per CO; unresolved preparation owns no integration slot.
/// TODO: verify sender/heads?
#[derive(Default)]
pub struct HeadsMessageHeadsEpic {
	cos: BTreeMap<CoId, CoHeadsWork>,
}

/// Per-CO ready work. Presence means one finite batch is currently integrating.
#[derive(Default)]
struct CoHeadsWork {
	pending: Vec<PreparedHeadsMessage>,
}

impl HeadsMessageHeadsEpic {
	/// Start ready work while its CO is idle, otherwise retain it for the next finite batch.
	fn prepared(&mut self, prepared: PreparedHeadsMessage) -> Option<Vec<PreparedHeadsMessage>> {
		match self.cos.entry(prepared.message.co.clone()) {
			Entry::Vacant(entry) => {
				entry.insert(CoHeadsWork::default());
				Some(vec![prepared])
			},
			Entry::Occupied(mut entry) => {
				entry.get_mut().pending.push(prepared);
				None
			},
		}
	}

	/// Release all ready arrivals retained during the finished integration, or remove idle state.
	fn finished(&mut self, co: &CoId) -> Option<Vec<PreparedHeadsMessage>> {
		let Entry::Occupied(mut entry) = self.cos.entry(co.clone()) else {
			return None;
		};
		let work = entry.get_mut();
		if work.pending.is_empty() {
			entry.remove();
			return None;
		}
		Some(std::mem::take(&mut work.pending))
	}
}

impl Epic<Action, (), CoContext> for HeadsMessageHeadsEpic {
	fn epic(
		&mut self,
		actions: &Actions<Action, (), CoContext>,
		action: &Action,
		_state: &(),
		context: &CoContext,
	) -> Option<impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static> {
		let stream = match action {
			Action::HeadsMessageReceived(
				message @ HeadsMessageReceivedAction { message: HeadsMessage::Heads(..), .. },
			) => prepare_heads_work(actions.clone(), context.clone(), message.clone()).boxed(),
			Action::HeadsMessageWork(work) => {
				let batch = match work.kind() {
					HeadsMessageWorkKind::Prepared(prepared) => self.prepared(prepared.as_ref().clone()),
					HeadsMessageWorkKind::Finished(co) => self.finished(co),
				}?;
				integrate_heads_batch(actions.clone(), context.clone(), batch).boxed()
			},
			_ => return None,
		};
		Some(stream)
	}
}

/// Respond when receive [`HeadsMessage::HeadsRequest`] message.
pub fn heads_message_heads_request(
	_actions: &Actions<Action, (), CoContext>,
	action: &Action,
	_state: &(),
	context: &CoContext,
) -> Option<impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static> {
	match action {
		Action::HeadsMessageReceived(HeadsMessageReceivedAction {
			from,
			peer,
			message_id,
			message: HeadsMessage::HeadsRequest(co),
			..
		}) => Some({
			let context = context.clone();
			let message_id = message_id.clone();
			let from = from.clone();
			let peer = *peer;
			let co = co.clone();
			async move { handle_request_heads(context, message_id, from, peer, co).await }
				.into_stream()
				.map(Action::map_error)
				.map(Ok)
		}),
		_ => None,
	}
}

/// Prepare one received message independently. Only an effective result becomes internal ready work.
fn prepare_heads_work(
	actions: Actions<Action, (), CoContext>,
	context: CoContext,
	message: HeadsMessageReceivedAction,
) -> impl Stream<Item = Result<Action, anyhow::Error>> {
	ActionDispatch::execute(actions, context.tasks(), move |dispatch| async move {
		match prepare_heads_message(&context, message.clone()).await {
			Ok(Some(prepared)) => {
				dispatch.dispatch(Action::HeadsMessageWork(HeadsMessageWorkAction::prepared(prepared)));
			},
			Ok(None) => {
				dispatch.dispatch(Action::HeadsMessageComplete(message, Ok(())));
			},
			Err(error) => {
				dispatch.dispatch(Action::HeadsMessageComplete(message, Err(error)));
			},
		}
		Ok(())
	})
}

/// Integrate one finite ready batch and always release its per-CO slot with a final work action.
fn integrate_heads_batch(
	actions: Actions<Action, (), CoContext>,
	context: CoContext,
	batch: Vec<PreparedHeadsMessage>,
) -> impl Stream<Item = Result<Action, anyhow::Error>> {
	ActionDispatch::execute(actions, context.tasks(), move |dispatch| async move {
		let Some(first) = batch.first() else {
			return Ok(());
		};
		let co = first.message.co.clone();
		let target = if first.shared {
			Ok(first.source.clone())
		} else {
			context
				.try_co_reducer(&co)
				.await
				.map_err(|error| HeadsError::from(anyhow::Error::from(error)))
		};
		let target = match target {
			Ok(target) => target,
			Err(error) => {
				for prepared in &batch {
					dispatch.dispatch(Action::HeadsMessageComplete(prepared.message.clone(), Err(error.clone())));
				}
				dispatch.dispatch(Action::HeadsMessageWork(HeadsMessageWorkAction::finished(co)));
				return Ok(());
			},
		};

		let integration = if first.shared {
			let states = batch.iter().map(|prepared| prepared.state.clone()).collect::<BTreeSet<_>>();
			target.join_states(states).await
		} else {
			let prepared = batch
				.iter()
				.map(|prepared| (prepared.source.clone(), prepared.state.clone()))
				.collect();
			join_prepared_states(&target, prepared).await
		}
		.map_err(HeadsError::from);
		let current_heads = match &integration {
			Ok(_) => Some(create_heads_body(&target).await),
			Err(_) => None,
		};
		let last_messages = batch
			.iter()
			.enumerate()
			.map(|(index, prepared)| (prepared.message.peer, index))
			.collect::<BTreeMap<_, _>>();

		for (index, prepared) in batch.iter().enumerate() {
			let result = match (&integration, current_heads.as_ref()) {
				(Err(error), _) => Err(error.clone()),
				(Ok(_), Some(current_heads))
					if last_messages.get(&prepared.message.peer) == Some(&index)
						&& matches!(
							(&prepared.message.message, current_heads),
							(HeadsMessage::Heads(_, peer_heads), HeadsMessage::Heads(_, current_heads))
								if peer_heads != current_heads
						) =>
				{
					respond_heads(&context, &target, &dispatch, &prepared.message, current_heads).await
				},
				(Ok(_), _) => Ok(()),
			};
			dispatch.dispatch(Action::HeadsMessageComplete(prepared.message.clone(), result));
		}
		dispatch.dispatch(Action::HeadsMessageWork(HeadsMessageWorkAction::finished(co)));
		Ok(())
	})
}

/// Return whether the message has active membership or its individual rejected outcome.
async fn admit_heads_message(context: &CoContext, message: &HeadsMessageReceivedAction) -> Result<bool, HeadsError> {
	let local_co = context.local_co_reducer().await?;
	match shared_membership(&local_co, &message.co, None).await? {
		Some(membership) if membership.membership_state() == Some(MembershipState::Active) => Ok(true),
		Some(membership)
			if matches!(
				membership.membership_state(),
				Some(MembershipState::Invite | MembershipState::Join | MembershipState::Pending)
			) =>
		{
			Err(HeadsError::Transient(ActionError::from(anyhow!("Pending membership"))))
		},
		_ => Ok(false),
	}
}

/// Admit and prepare one message through its own factory-returned reducer source.
async fn prepare_heads_message(
	context: &CoContext,
	message: HeadsMessageReceivedAction,
) -> Result<Option<PreparedHeadsMessage>, HeadsError> {
	if !admit_heads_message(context, &message).await? {
		return Ok(None);
	}
	let co_reducer = context
		.try_co_reducer(&message.co)
		.await
		.map_err(|error| HeadsError::from(anyhow::Error::from(error)))?;

	// verify
	//  TODO: needs to be handled by the guard?
	verify_from_participant(context, &co_reducer, &message.from)
		.await
		.map_err(|err| HeadsError::Permanent(err.into()))?;

	// network: let others know that the Co is connected and allow to use the implicit direct peer
	// connection
	if let Some(relation) = peer_relation(&message) {
		if let Some(connections) = context.network_connections().await {
			connections.dispatch(relation).ok();
		}
	}

	// join computation
	Ok(co_reducer
		.prepare_join_state(message_heads(&message))
		.await?
		.map(|state| PreparedHeadsMessage {
			message,
			source: co_reducer,
			state,
			shared: context.settings().feature_co_open_keep(),
		}))
}

/// Send the fixed final Heads response for one peer's selected batch message.
async fn respond_heads(
	context: &CoContext,
	co_reducer: &CoReducer,
	dispatch: &ActionDispatch<Action, (), CoContext>,
	message: &HeadsMessageReceivedAction,
	body: &HeadsMessage,
) -> Result<(), HeadsError> {
	let response =
		create_heads_message(context, co_reducer, body.clone(), Some(message.message_id.clone()), message.peer).await?;
	dispatch.dispatch(response);
	Ok(())
}

/// Peer relation for the implicit direct connection of a message's sender, when it names one.
fn peer_relation(message: &HeadsMessageReceivedAction) -> Option<PeerRelateCoAction> {
	message.from_peer.map(|peer_id| PeerRelateCoAction {
		co: message.co.clone(),
		peer_id,
		did: message.from.clone(),
		time: time::Instant::now(),
	})
}

/// The heads carried by a received [`HeadsMessage::Heads`] message.
fn message_heads(message: &HeadsMessageReceivedAction) -> BTreeSet<Cid> {
	match &message.message {
		HeadsMessage::Heads(_, heads) => heads.iter().cloned().map(Cid::from).collect(),
		_ => BTreeSet::new(),
	}
}

/// See: [`HeadsMessage::HeadsRequest`]
async fn handle_request_heads(
	context: CoContext,
	parent_message_id: String,
	from: Option<Did>,
	peer: PeerId,
	co: CoId,
) -> anyhow::Result<Action> {
	// identity
	let co_reducer = context.try_co_reducer(&co).await?;

	// body
	let body = match verify_from_participant(&context, &co_reducer, &from).await {
		Ok(_) => create_heads_body(&co_reducer).await,
		Err(err) => {
			tracing::warn!(?co, ?peer, ?from, ?err, "co-request-heads-failed");
			HeadsMessage::Error { co, code: HeadsErrorCode::Forbidden, message: "Forbidden".to_owned() }
		},
	};

	// result
	create_heads_message(&context, &co_reducer, body, Some(parent_message_id), peer).await
}

async fn create_heads_message(
	context: &CoContext,
	co_reducer: &CoReducer,
	body: HeadsMessage,
	parent_message_id: Option<String>,
	to: PeerId,
) -> anyhow::Result<Action> {
	// identity
	let identity = network_identity(context, co_reducer, None).await?;

	// message
	let mut header = HeadsMessage::create_header(context.date());
	header.thid = parent_message_id;
	let (message_header, message) = EncodedMessage::create_signed_json(&identity, header, &body)?;

	// result
	Ok(Action::DidCommSend { message_header, peer: to, message })
}

async fn create_heads_body(co: &CoReducer) -> HeadsMessage {
	HeadsMessage::Heads(co.id().clone(), MappedCoReducerState::new_co(co).await.external().weak_heads())
}

/// Respond when receive [`HeadsMessage::StateRequest`] message.
pub fn heads_message_state_request(
	_actions: &Actions<Action, (), CoContext>,
	action: &Action,
	_state: &(),
	context: &CoContext,
) -> Option<impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static> {
	match action {
		Action::HeadsMessageReceived(HeadsMessageReceivedAction {
			from,
			peer,
			message_id,
			message: HeadsMessage::StateRequest(co),
			..
		}) => Some({
			let context = context.clone();
			let message_id = message_id.clone();
			let from = from.clone();
			let peer = *peer;
			let co = co.clone();
			async move { handle_request_state(context, message_id, from, peer, co).await }
				.into_stream()
				.map(Action::map_error)
				.map(Ok)
		}),
		_ => None,
	}
}

/// See: [`HeadsMessage::StateRequest`]
async fn handle_request_state(
	context: CoContext,
	parent_message_id: String,
	from: Option<Did>,
	peer: PeerId,
	co: CoId,
) -> anyhow::Result<Action> {
	// identity
	let co_reducer = context.try_co_reducer(&co).await?;

	// body
	let body = match verify_from_participant(&context, &co_reducer, &from).await {
		Ok(_) => create_state_body(&co_reducer).await?,
		Err(err) => {
			tracing::warn!(?co, ?peer, ?from, ?err, "co-request-state-failed");
			HeadsMessage::Error { co, code: HeadsErrorCode::Forbidden, message: "Forbidden".to_owned() }
		},
	};

	// result
	create_heads_message(&context, &co_reducer, body, Some(parent_message_id), peer).await
}

async fn create_state_body(co: &CoReducer) -> anyhow::Result<HeadsMessage> {
	let (state, heads) = MappedCoReducerState::new_co(co).await.external().weak();
	Ok(HeadsMessage::State(co.id().clone(), state.ok_or_else(|| anyhow!("no state"))?, heads))
}

async fn verify_from_participant(
	context: &CoContext,
	co_reducer: &CoReducer,
	from: &Option<Did>,
) -> anyhow::Result<()> {
	let storage = co_reducer.storage();
	let state = co_reducer.reducer_state().await;

	// verify
	if !state::is_participant(&storage, state.co(), from).await? {
		context
			.check_access_or(co_reducer.id(), from.as_ref(), || {
				anyhow!("Permission denied for {:?} to {}", from, co_reducer.id())
			})
			.await?;
	}

	// result
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{
		application::memory::create_memory_reducer, library::create_reducer_action::create_reducer_action, Application,
		ApplicationBuilder, CreateCo, DidKeyIdentity, DidKeyProvider, MonotonicCoUuid, CO_CORE_NAME_CO,
		CO_CORE_NAME_KEYSTORE, CO_CORE_NAME_MEMBERSHIP,
	};
	use co_core_co::CoAction;
	use co_core_membership::MembershipsAction;
	use co_identity::Identity;
	use co_primitives::{tags, MonotonicCoDate, TagsAction, WeakCid};
	use co_storage::BlockStorage;
	use std::{collections::VecDeque, time::Duration};

	fn received(co: &str, message_id: &str) -> HeadsMessageReceivedAction {
		HeadsMessageReceivedAction {
			co: CoId::from(co),
			from: None,
			from_peer: None,
			peer: PeerId::random(),
			message_id: message_id.to_owned(),
			message: HeadsMessage::Heads(CoId::from(co), Default::default()),
			tags: Default::default(),
		}
	}

	fn prepared(source: &CoReducer, state: &crate::CoReducerState, co: &str, message_id: &str) -> PreparedHeadsMessage {
		PreparedHeadsMessage {
			message: received(co, message_id),
			source: source.clone(),
			state: state.clone(),
			shared: false,
		}
	}

	#[test]
	fn each_message_relates_its_own_peer() {
		let peer = PeerId::random();
		let mut first = received("co-a", "m1");
		first.from_peer = Some(peer);
		first.from = Some(Did::from("did:key:sender"));
		let relation = peer_relation(&first).expect("relation for a named sender peer");
		assert_eq!(relation.co, CoId::from("co-a"));
		assert_eq!(relation.peer_id, peer);
		assert_eq!(relation.did, Some(Did::from("did:key:sender")));

		let second = received("co-a", "m2");
		assert!(peer_relation(&second).is_none(), "no sender peer, no relation");
	}

	struct Fixture {
		application: Application,
		identity: DidKeyIdentity,
		co: CoReducer,
	}

	async fn fixture_with_open_keep(name: &str, open_keep: bool) -> Fixture {
		co_test::init_test_log();
		let mut builder = ApplicationBuilder::new_memory(name.to_owned())
			.without_keychain()
			.with_disabled_feature("co-local-encryption")
			.with_co_date(MonotonicCoDate::default())
			.with_co_uuid(MonotonicCoUuid::default());
		if open_keep {
			builder = builder.with_setting("feature", "co-open-keep");
		}
		let application = builder.build().await.expect("application");
		let identity = DidKeyIdentity::generate(None);
		let local_co = application.local_co_reducer().await.expect("local co");
		DidKeyProvider::new(local_co, CO_CORE_NAME_KEYSTORE)
			.store(&identity, None)
			.await
			.expect("store identity");
		let co = application
			.create_co(identity.clone(), CreateCo::new(name, None))
			.await
			.expect("create co");
		Fixture { application, identity, co }
	}

	async fn fixture(name: &str) -> Fixture {
		fixture_with_open_keep(name, false).await
	}

	impl Fixture {
		/// Create heads unknown to the CO's reducer by pushing an action through a detached
		/// memory reducer over the shared storage.
		async fn branch_heads(&self, tag: &str) -> BTreeSet<WeakCid> {
			let storage = self.co.context.storage(false);
			let runtime = self.application.context().inner.runtime();
			let core_resolver = self
				.application
				.context()
				.inner
				.create_shared_core_resolver(self.co.id().clone());
			let mut reducer = create_memory_reducer(
				runtime.runtime(),
				self.co.date().clone(),
				self.co.id(),
				&storage,
				Some(core_resolver),
				self.co.reducer_state().await,
			)
			.await
			.expect("memory reducer");
			let action = create_reducer_action(
				&storage,
				&self.identity,
				CO_CORE_NAME_CO,
				&CoAction::Tags { action: TagsAction::insert(tags!("heads-test": tag)) },
				Default::default(),
				self.co.date(),
			)
			.await
			.expect("reducer action");
			reducer
				.push_reference(&storage, runtime.runtime(), &self.identity, action)
				.await
				.expect("branch push");
			reducer.heads().iter().map(WeakCid::from).collect()
		}

		fn received(&self, message_id: &str, heads: BTreeSet<WeakCid>) -> HeadsMessageReceivedAction {
			HeadsMessageReceivedAction {
				co: self.co.id().clone(),
				from: Some(Did::from(self.identity.identity())),
				from_peer: None,
				peer: PeerId::random(),
				message_id: message_id.to_owned(),
				message: HeadsMessage::Heads(self.co.id().clone(), heads),
				tags: Default::default(),
			}
		}

		/// Run one action through the epic and collect the worker's emitted actions.
		async fn run_epic(&self, epic: &mut HeadsMessageHeadsEpic, action: &Action) -> Option<Vec<Action>> {
			let actions = Actions::default();
			let stream = epic.epic(&actions, action, &(), self.application.context())?;
			Some(stream.map(|item| item.expect("emitted action")).collect::<Vec<Action>>().await)
		}

		/// Feed opaque work actions back through the Epic like the application actor does.
		async fn run_workflow(&self, epic: &mut HeadsMessageHeadsEpic, action: Action) -> Vec<Action> {
			let mut pending = VecDeque::from([action]);
			let mut visible = Vec::new();
			while let Some(action) = pending.pop_front() {
				if let Some(emitted) = self.run_epic(epic, &action).await {
					for action in emitted {
						if matches!(action, Action::HeadsMessageWork(_)) {
							pending.push_back(action);
						} else {
							visible.push(action);
						}
					}
				}
			}
			visible
		}
	}

	fn prepared_ids(batch: Option<Vec<PreparedHeadsMessage>>) -> Option<Vec<String>> {
		batch.map(|batch| batch.into_iter().map(|prepared| prepared.message.message_id).collect())
	}

	#[co_test::timeout(10000)]
	#[tokio::test]
	async fn ready_work_batches_once_per_co_and_drops_idle_state() {
		let fixture = fixture("heads-ready-queue").await;
		let state = fixture.co.reducer_state().await;
		let mut epic = HeadsMessageHeadsEpic::default();

		assert_eq!(
			prepared_ids(epic.prepared(prepared(&fixture.co, &state, "co-a", "m1"))),
			Some(vec!["m1".to_owned()])
		);
		assert!(epic.prepared(prepared(&fixture.co, &state, "co-a", "m2")).is_none());
		assert!(epic.prepared(prepared(&fixture.co, &state, "co-a", "m3")).is_none());
		assert_eq!(prepared_ids(epic.finished(&CoId::from("co-a"))), Some(vec!["m2".to_owned(), "m3".to_owned()]));

		assert!(epic.prepared(prepared(&fixture.co, &state, "co-a", "m4")).is_none());
		assert_eq!(prepared_ids(epic.finished(&CoId::from("co-a"))), Some(vec!["m4".to_owned()]));
		assert!(epic.finished(&CoId::from("co-a")).is_none());
		assert!(epic.cos.is_empty(), "idle CO state is removed");

		assert!(epic.prepared(prepared(&fixture.co, &state, "co-a", "a")).is_some());
		assert!(epic.prepared(prepared(&fixture.co, &state, "co-b", "b")).is_some());
		assert!(epic.finished(&CoId::from("co-b")).is_none());
		assert!(epic.cos.contains_key(&CoId::from("co-a")), "unrelated CO work stays active");
		assert!(!epic.cos.contains_key(&CoId::from("co-b")), "finished CO state is removed");
		assert!(epic.finished(&CoId::from("unknown")).is_none(), "unknown finish is ignored");
		assert!(epic.finished(&CoId::from("co-a")).is_none());
		assert!(epic.cos.is_empty());
	}

	fn log_since(offset: usize) -> String {
		let bytes = std::fs::read(co_test::test_log_path()).expect("test log");
		String::from_utf8_lossy(bytes.get(offset..).unwrap_or_default()).into_owned()
	}

	fn target_creation_logged(log: &str, co: &CoId) -> bool {
		log.lines()
			.any(|line| line.contains("co-storage-failed") && line.contains(co.as_str()))
	}

	#[co_test::timeout(10000)]
	#[tokio::test]
	async fn absent_and_pending_memberships_do_not_resolve_target_reducer() {
		let fixture = fixture("heads-membership-admission").await;
		let mut epic = HeadsMessageHeadsEpic::default();

		let absent = received("heads-absent-membership", "absent");
		let offset = std::fs::metadata(co_test::test_log_path())
			.map(|metadata| metadata.len() as usize)
			.unwrap_or_default();
		let absent_actions = fixture
			.run_epic(&mut epic, &Action::HeadsMessageReceived(absent.clone()))
			.await
			.expect("absent membership completes");
		assert_eq!(absent_actions.len(), 1, "absent membership only completes");
		let Action::HeadsMessageComplete(done, result) = &absent_actions[0] else {
			panic!("expected absent membership completion, got {:?}", absent_actions[0]);
		};
		assert_eq!(done.message_id, "absent");
		assert!(result.is_ok(), "absent membership is ignored, got {result:?}");
		let absent_log = log_since(offset);

		let pending_co = CoId::from("heads-pending-membership");
		fixture
			.application
			.local_co_reducer()
			.await
			.expect("local co")
			.push(
				&fixture.identity,
				CO_CORE_NAME_MEMBERSHIP,
				&MembershipsAction::Invited {
					id: pending_co.clone(),
					did: fixture.identity.identity().to_owned(),
					options: Default::default(),
				},
			)
			.await
			.expect("pending membership");
		let pending = received(pending_co.as_str(), "pending");
		let offset = std::fs::metadata(co_test::test_log_path())
			.map(|metadata| metadata.len() as usize)
			.unwrap_or_default();
		let pending_actions = fixture
			.run_epic(&mut epic, &Action::HeadsMessageReceived(pending.clone()))
			.await
			.expect("pending membership completes");
		assert_eq!(pending_actions.len(), 1, "pending membership only completes");
		let Action::HeadsMessageComplete(done, result) = &pending_actions[0] else {
			panic!("expected pending membership completion, got {:?}", pending_actions[0]);
		};
		assert_eq!(done.message_id, "pending");
		assert!(matches!(result, Err(HeadsError::Transient(_))), "pending membership stays transient, got {result:?}");
		let pending_log = log_since(offset);

		assert_eq!(
			(target_creation_logged(&absent_log, &absent.co), target_creation_logged(&pending_log, &pending_co),),
			(false, false),
			"rejected messages must not start target storage creation",
		);
	}

	#[co_test::timeout(10000)]
	#[tokio::test]
	async fn default_preparation_isolated_then_integrates_and_completes_once() {
		let fixture = fixture("heads-immediate").await;
		let mut epic = HeadsMessageHeadsEpic::default();
		let heads = fixture.branch_heads("immediate").await;
		let message = fixture.received("m1", heads.clone());
		let baseline = fixture.co.reducer_state().await;

		let mut preparation = fixture
			.run_epic(&mut epic, &Action::HeadsMessageReceived(message.clone()))
			.await;
		let preparation = preparation.as_mut().expect("effective message prepares");
		assert_eq!(preparation.len(), 1);
		let Action::HeadsMessageWork(work) = &preparation[0] else {
			panic!("expected prepared work, got {:?}", preparation[0]);
		};
		let HeadsMessageWorkKind::Prepared(prepared) = work.kind() else {
			panic!("expected Prepared work");
		};
		assert!(!prepared.shared, "default preparation uses an isolated source");
		assert_ne!(prepared.state, baseline, "the source prepared the incoming heads");
		assert_eq!(fixture.co.reducer_state().await, baseline, "preparation does not publish through the CO actor");

		let emitted = fixture.run_workflow(&mut epic, preparation.remove(0)).await;

		// the message's heads are integrated
		let state = fixture.co.reducer_state().await;
		assert_ne!(state, baseline, "the join changed the reducer");
		assert_eq!(state.heads(), heads.iter().cloned().map(Cid::from).collect::<BTreeSet<Cid>>());

		// one response to the sender, then exactly one completion
		assert_eq!(emitted.len(), 2, "one response and one completion, got {emitted:?}");
		let Action::DidCommSend { message_header, peer, .. } = &emitted[0] else {
			panic!("expected the response first, got {:?}", emitted[0]);
		};
		assert_eq!(message_header.thid.as_deref(), Some("m1"), "the response references the message");
		assert_eq!(peer, &message.peer, "the response goes to the sender peer");
		let Action::HeadsMessageComplete(done, result) = &emitted[1] else {
			panic!("expected the completion, got {:?}", emitted[1]);
		};
		assert_eq!(done.message_id, "m1");
		assert!(result.is_ok(), "effective message completes successfully, got {result:?}");

		// the completion leaves the CO idle
		assert!(fixture.run_epic(&mut epic, &emitted[1]).await.is_none());
		assert!(epic.cos.is_empty(), "idle CO state is removed");
	}

	#[co_test::timeout(10000)]
	#[tokio::test]
	async fn co_open_keep_reuses_the_preparing_reducer() {
		let fixture = fixture_with_open_keep("heads-open-keep", true).await;
		let mut epic = HeadsMessageHeadsEpic::default();
		let heads = fixture.branch_heads("shared").await;
		let message = fixture.received("shared", heads.clone());
		let mut preparation = fixture
			.run_epic(&mut epic, &Action::HeadsMessageReceived(message.clone()))
			.await
			.expect("effective message prepares");
		assert_eq!(preparation.len(), 1);
		let Action::HeadsMessageWork(work) = &preparation[0] else {
			panic!("expected prepared work, got {:?}", preparation[0]);
		};
		let HeadsMessageWorkKind::Prepared(prepared) = work.kind() else {
			panic!("expected Prepared work");
		};
		assert!(prepared.shared, "co-open-keep marks the source for shared integration");

		let emitted = fixture.run_workflow(&mut epic, preparation.remove(0)).await;
		assert_eq!(emitted.len(), 2, "one response and one completion, got {emitted:?}");
		assert!(
			matches!(&emitted[0], Action::DidCommSend { message_header, .. } if message_header.thid.as_deref() == Some("shared"))
		);
		assert!(matches!(&emitted[1], Action::HeadsMessageComplete(done, Ok(())) if done.message_id == "shared"));
		assert_eq!(
			fixture.co.reducer_state().await.heads(),
			heads.into_iter().map(Cid::from).collect(),
			"shared integration publishes the incoming heads"
		);
		assert!(epic.cos.is_empty());
	}

	#[co_test::timeout(10000)]
	#[tokio::test]
	async fn unresolved_preparation_does_not_block_later_valid_heads() {
		let fixture = fixture("heads-unresolved-bypass").await;
		let valid_heads = fixture.branch_heads("valid").await;
		let invalid = fixture.received("invalid", BTreeSet::from([WeakCid::from(Cid::default())]));
		let valid = fixture.received("valid", valid_heads.clone());
		let mut observed = Box::pin(fixture.application.actions());

		fixture
			.application
			.handle()
			.dispatch(Action::HeadsMessageReceived(invalid))
			.expect("dispatch unresolved heads");
		fixture
			.application
			.handle()
			.dispatch(Action::HeadsMessageReceived(valid))
			.expect("dispatch valid heads");

		let invalid_completed = tokio::time::timeout(Duration::from_secs(10), async {
			let mut invalid_completed = false;
			loop {
				match observed.next().await.expect("application action stream") {
					Action::HeadsMessageComplete(done, result) if done.message_id == "invalid" => {
						invalid_completed = true;
						assert!(result.is_err(), "missing heads cannot succeed");
					},
					Action::HeadsMessageComplete(done, result) if done.message_id == "valid" => {
						assert!(result.is_ok(), "later valid heads failed: {result:?}");
						break invalid_completed;
					},
					_ => {},
				}
			}
		})
		.await
		.expect("later valid heads complete while the missing block remains unresolved");
		assert!(!invalid_completed, "the unresolved message must not own the integration slot");
		assert_eq!(
			fixture.co.reducer_state().await.heads(),
			valid_heads.into_iter().map(Cid::from).collect(),
			"the later valid heads are published"
		);
	}

	#[co_test::timeout(10000)]
	#[tokio::test]
	async fn prepared_work_is_visible_to_application_subscribers() {
		let fixture = fixture("heads-visible-work").await;
		let mut observed = Box::pin(fixture.application.actions());
		let co = fixture.co.id().clone();
		let message_id = "visible-prepared";
		let source = fixture
			.application
			.context()
			.try_co_reducer(&co)
			.await
			.expect("prepared source");
		let prepared = Action::HeadsMessageWork(HeadsMessageWorkAction::prepared(PreparedHeadsMessage {
			message: fixture.received(message_id, fixture.co.reducer_state().await.weak_heads()),
			source,
			state: fixture.co.reducer_state().await,
			shared: false,
		}));
		assert!(!format!("{prepared:?}").contains(message_id), "opaque Debug must not reveal prepared work");

		fixture.application.handle().dispatch(prepared).expect("dispatch Prepared work");

		tokio::time::timeout(Duration::from_secs(10), async {
			let mut saw_prepared = false;
			loop {
				let Action::HeadsMessageWork(work) = observed.next().await.expect("application action stream") else {
					continue;
				};
				match work.kind() {
					HeadsMessageWorkKind::Prepared(prepared) => {
						assert_eq!(prepared.message.message_id, message_id);
						saw_prepared = true;
					},
					HeadsMessageWorkKind::Finished(finished_co) if finished_co == &co => {
						assert!(saw_prepared, "Prepared work must be observed before Finished");
						break;
					},
					HeadsMessageWorkKind::Finished(_) => {},
				}
			}
		})
		.await
		.expect("subscriber receives Prepared and Finished work");
	}

	#[co_test::timeout(10000)]
	#[tokio::test]
	async fn prepared_overlay_merge_failure_fails_the_finite_batch() {
		let fixture = fixture("heads-prepared-merge-failure").await;
		let baseline = fixture.co.reducer_state().await;
		let baseline_state = baseline.0.expect("baseline state");
		let valid_message = fixture.received("valid", fixture.branch_heads("valid").await);
		let invalid_message = fixture.received("invalid", fixture.branch_heads("invalid").await);
		let valid = prepare_heads_message(fixture.application.context(), valid_message)
			.await
			.expect("valid preparation")
			.expect("effective valid preparation");
		let mut invalid = prepare_heads_message(fixture.application.context(), invalid_message)
			.await
			.expect("invalid source prepares before corruption")
			.expect("effective invalid preparation");
		invalid
			.source
			.storage()
			.remove(&baseline_state)
			.await
			.expect("record unexpected prepared-source remove");
		invalid.state = crate::CoReducerState::new(Some(baseline_state), invalid.state.1.clone());

		let emitted =
			integrate_heads_batch(Actions::default(), fixture.application.context().clone(), vec![valid, invalid])
				.map(|item| item.expect("integration action"))
				.collect::<Vec<_>>()
				.await;

		assert!(
			!emitted.iter().any(|action| matches!(action, Action::DidCommSend { .. })),
			"a failed prepared-overlay merge sends no heads response"
		);
		let completions = emitted
			.iter()
			.filter_map(|action| match action {
				Action::HeadsMessageComplete(message, result) => Some((&message.message_id, result)),
				_ => None,
			})
			.collect::<Vec<_>>();
		assert_eq!(completions.len(), 2, "both prepared messages settle");
		for (completion, message_id) in completions.iter().zip(["valid", "invalid"]) {
			assert_eq!(completion.0, message_id);
			assert!(completion.1.is_err(), "{message_id} must share the finite-batch failure");
		}
	}

	#[co_test::timeout(10000)]
	#[tokio::test]
	async fn divergent_heads_respond_once_to_the_peers_last_message() {
		let fixture = fixture("heads-peer-feedback").await;
		let peer = PeerId::random();
		let mut first_message = fixture.received("first", fixture.branch_heads("first").await);
		first_message.peer = peer;
		let mut last_message = fixture.received("last", fixture.branch_heads("last").await);
		last_message.peer = peer;
		let last_heads = match &last_message.message {
			HeadsMessage::Heads(_, heads) => heads.clone(),
			_ => unreachable!("the fixture creates a Heads message"),
		};
		let first = prepare_heads_message(fixture.application.context(), first_message)
			.await
			.expect("first preparation")
			.expect("effective first preparation");
		let last = prepare_heads_message(fixture.application.context(), last_message)
			.await
			.expect("last preparation")
			.expect("effective last preparation");

		let emitted =
			integrate_heads_batch(Actions::default(), fixture.application.context().clone(), vec![first, last])
				.map(|item| item.expect("integration action"))
				.collect::<Vec<_>>()
				.await;
		let responses = emitted
			.iter()
			.filter_map(|action| match action {
				Action::DidCommSend { message_header, peer, .. } => Some((message_header, peer)),
				_ => None,
			})
			.collect::<Vec<_>>();
		assert_eq!(responses.len(), 1, "one divergent-heads response per peer");
		assert_eq!(responses[0].0.thid.as_deref(), Some("last"));
		assert_eq!(responses[0].1, &peer);
		let final_heads = match create_heads_body(&fixture.co).await {
			HeadsMessage::Heads(_, heads) => heads,
			_ => unreachable!("create_heads_body always returns Heads"),
		};
		assert_ne!(final_heads, last_heads, "the final external Heads diverge");
		let completions = emitted
			.iter()
			.filter_map(|action| match action {
				Action::HeadsMessageComplete(message, result) => Some((&message.message_id, result)),
				_ => None,
			})
			.collect::<Vec<_>>();
		assert_eq!(completions.len(), 2, "both messages complete");
		assert!(completions.iter().all(|(_, result)| result.is_ok()));
	}

	#[co_test::timeout(10000)]
	#[tokio::test]
	async fn matching_last_peer_heads_need_no_feedback() {
		let fixture = fixture("heads-peer-feedback-equal").await;
		let state = fixture.co.reducer_state().await;
		let heads = match create_heads_body(&fixture.co).await {
			HeadsMessage::Heads(_, heads) => heads,
			_ => unreachable!("create_heads_body always returns Heads"),
		};
		let source = fixture
			.application
			.context()
			.try_co_reducer(fixture.co.id())
			.await
			.expect("prepared source");
		let prepared = PreparedHeadsMessage { message: fixture.received("equal", heads), source, state, shared: false };

		let emitted = integrate_heads_batch(Actions::default(), fixture.application.context().clone(), vec![prepared])
			.map(|item| item.expect("integration action"))
			.collect::<Vec<_>>()
			.await;

		assert!(!emitted.iter().any(|action| matches!(action, Action::DidCommSend { .. })));
		assert!(matches!(
			emitted.first(),
			Some(Action::HeadsMessageComplete(message, Ok(()))) if message.message_id == "equal"
		));
	}

	#[co_test::timeout(10000)]
	#[tokio::test]
	async fn buffered_batch_keeps_outcomes_individual() {
		let fixture = fixture("heads-mixed").await;
		let mut epic = HeadsMessageHeadsEpic::default();
		let baseline = fixture.co.reducer_state().await;
		let baseline_heads = baseline.weak_heads();

		// Prepare the first effective message and start its integration without consuming
		// the final Finished action yet. This owns the ready-work slot for the CO.
		let first = fixture.received("first", fixture.branch_heads("first").await);
		let first_preparation = fixture
			.run_epic(&mut epic, &Action::HeadsMessageReceived(first.clone()))
			.await
			.expect("the first message prepares");
		assert_eq!(first_preparation.len(), 1, "preparation emits one work action");
		let Action::HeadsMessageWork(_) = &first_preparation[0] else {
			panic!("expected prepared work, got {:?}", first_preparation[0]);
		};
		let actions = Actions::default();
		let first_stream = epic
			.epic(&actions, &first_preparation[0], &(), fixture.application.context())
			.expect("the first ready message starts immediately");

		// Equal ready states that arrive while integration is active form the next batch.
		// Ineffective and rejected raw messages complete independently and never enter it.
		let duplicate_heads = fixture.branch_heads("duplicate").await;
		let effective = fixture.received("effective", duplicate_heads.clone());
		let duplicate = fixture.received("duplicate", duplicate_heads);
		let ineffective = fixture.received("ineffective", baseline_heads);
		let mut forbidden = fixture.received("forbidden", fixture.branch_heads("forbidden").await);
		forbidden.from = Some(Did::from("did:key:unknown"));
		for message in [&effective, &duplicate] {
			let preparation = fixture
				.run_epic(&mut epic, &Action::HeadsMessageReceived(message.clone()))
				.await
				.expect("effective message prepares");
			assert_eq!(preparation.len(), 1, "preparation emits one work action");
			assert!(
				fixture.run_epic(&mut epic, &preparation[0]).await.is_none(),
				"ready arrivals are retained while integration is active"
			);
		}
		let ineffective_actions = fixture
			.run_epic(&mut epic, &Action::HeadsMessageReceived(ineffective.clone()))
			.await
			.expect("ineffective message settles independently");
		let forbidden_actions = fixture
			.run_epic(&mut epic, &Action::HeadsMessageReceived(forbidden.clone()))
			.await
			.expect("rejected message settles independently");
		for (actions, (message_id, ok)) in [ineffective_actions, forbidden_actions]
			.iter()
			.zip([("ineffective", true), ("forbidden", false)])
		{
			assert_eq!(actions.len(), 1, "{message_id} only completes");
			let Action::HeadsMessageComplete(done, result) = &actions[0] else {
				panic!("expected {message_id} completion, got {:?}", actions[0]);
			};
			assert_eq!(done.message_id, message_id);
			assert_eq!(result.is_ok(), ok, "{message_id}: unexpected result {result:?}");
			if !ok {
				assert!(matches!(result, Err(HeadsError::Permanent(_))));
			}
		}

		// Consuming Finished for the first integration releases exactly one finite batch.
		let first_actions = first_stream
			.map(|item| item.expect("first integration action"))
			.collect::<Vec<_>>()
			.await;
		assert_eq!(first_actions.len(), 3, "response, completion, and Finished, got {first_actions:?}");
		let Action::HeadsMessageComplete(done, result) = &first_actions[1] else {
			panic!("expected first completion, got {:?}", first_actions[1]);
		};
		assert_eq!(done.message_id, "first");
		assert!(result.is_ok());
		let Action::HeadsMessageWork(_) = &first_actions[2] else {
			panic!("expected Finished work, got {:?}", first_actions[2]);
		};
		let batch_actions = fixture
			.run_epic(&mut epic, &first_actions[2])
			.await
			.expect("the retained batch starts");
		assert_eq!(batch_actions.len(), 5, "two responses, two completions, and Finished, got {batch_actions:?}");
		for (action, message) in [(&batch_actions[0], &effective), (&batch_actions[2], &duplicate)] {
			let Action::DidCommSend { message_header, peer, .. } = action else {
				panic!("expected a response, got {action:?}");
			};
			assert_eq!(message_header.thid.as_deref(), Some(message.message_id.as_str()));
			assert_eq!(peer, &message.peer);
		}
		for (action, message_id) in [(&batch_actions[1], "effective"), (&batch_actions[3], "duplicate")] {
			let Action::HeadsMessageComplete(done, result) = action else {
				panic!("expected a completion, got {action:?}");
			};
			assert_eq!(done.message_id, message_id, "completions arrive in arrival order");
			assert!(result.is_ok(), "{message_id}: unexpected result {result:?}");
		}
		let Action::HeadsMessageWork(_) = &batch_actions[4] else {
			panic!("expected Finished work, got {:?}", batch_actions[4]);
		};

		// The actor-ineffective duplicate still receives divergent-Heads feedback for its peer.
		// Every message settles once, and idle state is dropped.
		assert!(fixture.run_epic(&mut epic, &batch_actions[4]).await.is_none());
		assert!(epic.cos.is_empty(), "idle CO state is removed");
	}
}
