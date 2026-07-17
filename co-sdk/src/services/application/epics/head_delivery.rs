// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::{
	library::{
		head_delivery::{
			merge_heads_recipients, HeadsDeliveryCompleteAction, HeadsDeliveryIntent, HeadsDeliveryOutcome,
			HeadsDeliveryPhase, HeadsRecipient, PushHeadsToDidsAction,
		},
		head_delivery_queue::{enqueue_latest, HEADS_DELIVERY_TASK_TYPE},
		network_queue::{network_queue_task_in_list, TaskState, LIST_NAME_DOING},
		settings_timeout::settings_timeout,
		to_external_cid::to_external_cids_opt_force,
	},
	state, Action, ActionError, CoContext, CoReducerFactory, CoReducerFactoryError,
};
use cid::Cid;
use co_actor::{Actions, Epic};
use co_core_board::Task;
use co_identity::{PeerDidCommHeader, PrivateIdentity, PrivateIdentityResolver};
use co_network::{connections::ConnectionMessage, identities_networks, EncodedMessage, HeadsMessage, PeerId};
use co_primitives::{Block, BlockSerializer, CoId, Did, Network, WeakCid};
use futures::{Stream, StreamExt};
use std::collections::BTreeSet;

const MAX_CONCURRENT_HEAD_ADMISSIONS: usize = 16;
const MAX_CONCURRENT_HEAD_PEER_SENDS: usize = 32;

#[derive(Debug, Default)]
pub struct HeadDeliveryEpic;
impl Epic<Action, (), CoContext> for HeadDeliveryEpic {
	fn epic(
		&mut self,
		_actions: &Actions<Action, (), CoContext>,
		action: &Action,
		_state: &(),
		context: &CoContext,
	) -> Option<impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static> {
		match action {
			Action::PushHeadsToDids(request) => {
				let mut request = request.clone();
				request.recipients = merge_heads_recipients(std::mem::take(&mut request.recipients));
				Some(admit_batch(context.clone(), request).boxed())
			},
			Action::NetworkTaskExecute { co, task_id, task_type, task } if task_type == HEADS_DELIVERY_TASK_TYPE => {
				Some(execute_queued(context.clone(), co.clone(), task_id.clone(), task.clone()).boxed())
			},
			Action::HeadsDeliveryComplete(done) => queue_process_after_admission(done)
				.map(|action| futures::stream::once(std::future::ready(Ok(action))).boxed()),
			_ => None,
		}
	}
}

#[derive(Debug, Clone)]
struct PreparedHeads {
	heads: BTreeSet<Cid>,
	message: EncodedMessage,
}

#[derive(Debug, Clone)]
struct LoadedHeads {
	heads: BTreeSet<Cid>,
	external_heads: BTreeSet<Cid>,
}

#[derive(Debug, Clone)]
struct PreparedDelivery {
	prepared: PreparedHeads,
	networks: BTreeSet<Network>,
}

#[derive(Debug, Clone)]
enum AttemptFailure {
	Transient { heads: Option<BTreeSet<Cid>>, error: ActionError },
	Permanent { heads: Option<BTreeSet<Cid>>, error: ActionError },
}

#[derive(Debug, Clone)]
enum Authorization {
	Allowed,
	Denied,
	Transient(ActionError),
	Permanent(ActionError),
}

async fn load_current_heads(context: &CoContext, co: &CoId) -> Result<LoadedHeads, AttemptFailure> {
	let reducer = match context.try_co_reducer(co).await {
		Ok(reducer) => reducer,
		Err(error @ CoReducerFactoryError::CoNotFound(..)) => {
			return Err(AttemptFailure::Permanent { heads: None, error: anyhow::Error::from(error).into() });
		},
		Err(error) => {
			return Err(AttemptFailure::Transient { heads: None, error: anyhow::Error::from(error).into() });
		},
	};
	let state = reducer.reducer_state().await;
	let heads = state.heads();
	let external_heads = to_external_cids_opt_force(&reducer.storage(), heads.clone())
		.await
		.ok_or_else(|| AttemptFailure::Transient {
			heads: Some(heads.clone()),
			error: anyhow::anyhow!("external head mapping is not ready for {co}").into(),
		})?;
	Ok(LoadedHeads { heads, external_heads })
}

async fn sign_heads(
	context: &CoContext,
	co: &CoId,
	from: &Did,
	loaded: LoadedHeads,
) -> Result<PreparedHeads, AttemptFailure> {
	let LoadedHeads { heads, external_heads } = loaded;
	let resolver = context
		.private_identity_resolver()
		.await
		.map_err(|error| AttemptFailure::Permanent { heads: Some(heads.clone()), error: error.into() })?;
	let identity = resolver
		.resolve_private(from)
		.await
		.map_err(|error| AttemptFailure::Permanent {
			heads: Some(heads.clone()),
			error: anyhow::Error::from(error).into(),
		})?;
	identity
		.try_didcomm_private()
		.map_err(|error| AttemptFailure::Permanent { heads: Some(heads.clone()), error: error.into() })?;
	let network = context.network().await.ok_or_else(|| AttemptFailure::Transient {
		heads: Some(heads.clone()),
		error: anyhow::anyhow!("network is not started").into(),
	})?;
	let header = PeerDidCommHeader {
		header: HeadsMessage::create_header(context.date()),
		from_peer_id: Some(network.local_peer_id().to_string()),
	};
	let body = HeadsMessage::Heads(co.clone(), external_heads.iter().map(WeakCid::from).collect());
	let (_, message) = EncodedMessage::create_signed_json(&identity, header.into(), &body)
		.map_err(|error| AttemptFailure::Permanent { heads: Some(heads.clone()), error: error.into() })?;
	Ok(PreparedHeads { heads, message })
}

#[cfg(test)]
fn contains_active_participant(participants: &[co_core_co::Participant], recipient: &Did) -> bool {
	participants
		.iter()
		.any(|participant| participant.did.as_str() == recipient.as_str() && participant.state.is_active())
}

async fn authorize_recipient(context: &CoContext, co: &CoId, recipient: &Did) -> Authorization {
	let reducer = match context.try_co_reducer(co).await {
		Ok(reducer) => reducer,
		Err(error @ CoReducerFactoryError::CoNotFound(..)) => {
			return Authorization::Permanent(anyhow::Error::from(error).into());
		},
		Err(error) => return Authorization::Transient(anyhow::Error::from(error).into()),
	};
	let reducer_state = reducer.reducer_state().await;
	match state::participant(&reducer.storage(), reducer_state.co(), recipient).await {
		Ok(Some(participant)) if participant.state.is_active() => return Authorization::Allowed,
		Ok(_) => {},
		Err(error) => return Authorization::Transient(anyhow::Error::from(error).into()),
	}
	#[cfg(feature = "guard")]
	{
		use co_guard::AccessGuard;
		if let Some(guard) = context.access_guard() {
			return match guard.check_access(co, recipient).await {
				Ok(true) => Authorization::Allowed,
				Ok(false) => Authorization::Denied,
				Err(error) => Authorization::Transient(error.into()),
			};
		}
	}
	Authorization::Denied
}

async fn recipient_networks(context: &CoContext, recipient: &HeadsRecipient) -> Result<BTreeSet<Network>, ActionError> {
	let mut networks = recipient.connectivity.network.clone();
	let mut last_resolution_error = None;
	let identities = std::iter::once(recipient.did.clone())
		.chain(recipient.connectivity.participants.iter().cloned())
		.collect::<Vec<_>>();
	match context.identity_resolver().await {
		Ok(resolver) => {
			let mut resolved = Box::pin(identities_networks(Some(&resolver), identities));
			while let Some(result) = resolved.next().await {
				match result {
					Ok(network) => {
						networks.insert(network);
					},
					Err(error) => {
						last_resolution_error = Some(error);
					},
				}
			}
		},
		Err(error) => {
			last_resolution_error = Some(error);
		},
	}
	if networks.is_empty() {
		return Err(last_resolution_error
			.unwrap_or_else(|| anyhow::anyhow!("no network route for {}", recipient.did))
			.into());
	}
	Ok(networks)
}

async fn prepare_delivery(
	context: &CoContext,
	co: &CoId,
	from: &Did,
	recipient: &HeadsRecipient,
) -> Result<PreparedDelivery, AttemptFailure> {
	let loaded = load_current_heads(context, co).await?;
	let networks = recipient_networks(context, recipient)
		.await
		.map_err(|error| AttemptFailure::Transient { heads: Some(loaded.heads.clone()), error })?;
	let prepared = sign_heads(context, co, from, loaded).await?;
	Ok(PreparedDelivery { prepared, networks })
}

async fn send_prepared(
	context: &CoContext,
	co: &CoId,
	from: &Did,
	recipient: &HeadsRecipient,
	networks: BTreeSet<Network>,
	prepared: &PreparedHeads,
) -> Result<BTreeSet<PeerId>, ActionError> {
	let network = context
		.network()
		.await
		.ok_or_else(|| ActionError::from(anyhow::anyhow!("network is not started")))?;
	let connections = context
		.network_connections()
		.await
		.ok_or_else(|| ActionError::from(anyhow::anyhow!("network connections are unavailable")))?;
	let timeout_duration = settings_timeout(context, co, Some("heads-delivery")).await;
	let local_peer = network.local_peer_id();
	let mut changes = Box::pin(ConnectionMessage::did_use(connections, from.clone(), recipient.did.clone(), networks));
	let result = async {
		let peers = co_actor::time::timeout(timeout_duration, async {
			while let Some(change) = changes.next().await {
				let mut peers = change.map_err(|error| ActionError::from(anyhow::Error::from(error)))?.added;
				peers.remove(&local_peer);
				if !peers.is_empty() {
					return Ok::<BTreeSet<PeerId>, ActionError>(peers);
				}
			}
			Ok(BTreeSet::new())
		})
		.await
		.map_err(|error| ActionError::from(anyhow::anyhow!("recipient connection timed out: {error:?}")))??;
		if peers.is_empty() {
			return Err(anyhow::anyhow!("no remote peer for {}", recipient.did).into());
		}

		let sent: BTreeSet<PeerId> = futures::stream::iter(peers)
			.map(|peer| {
				let network = network.clone();
				let message = prepared.message.clone();
				async move {
					let result = network.didcomm_send([peer], message, timeout_duration).await;
					(peer, result)
				}
			})
			.buffer_unordered(MAX_CONCURRENT_HEAD_PEER_SENDS)
			.filter_map(|(peer, result)| async move {
				match result {
					Ok(sent_peer) => Some(sent_peer),
					Err(error) => {
						tracing::warn!(?error, ?peer, "targeted-head-send-failed");
						None
					},
				}
			})
			.collect()
			.await;
		if sent.is_empty() {
			Err(anyhow::anyhow!("all targeted head sends failed for {}", recipient.did).into())
		} else {
			Ok(sent)
		}
	}
	.await;
	drop(changes);
	result
}

fn admit_batch(
	context: CoContext,
	request: PushHeadsToDidsAction,
) -> impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static {
	let PushHeadsToDidsAction { co, from, recipients } = request;
	futures::stream::iter(recipients)
		.map(move |recipient| {
			let context = context.clone();
			let co = co.clone();
			let from = from.clone();
			async move {
				let recipient_did = recipient.did.clone();
				let intent = HeadsDeliveryIntent {
					co: co.clone(),
					from,
					recipient: recipient.did,
					connectivity: recipient.connectivity,
				};
				let result = enqueue_latest(&context, intent).await.map(|_| ()).map_err(ActionError::from);
				admission_completion(&co, recipient_did, result)
			}
		})
		.buffer_unordered(MAX_CONCURRENT_HEAD_ADMISSIONS)
		.map(Ok)
}

fn task_complete(co: CoId, task_id: String, task_state: TaskState) -> Action {
	Action::NetworkTaskExecuteComplete { co, task_id, task_state }
}

fn execution_actions(
	co: CoId,
	task_id: String,
	completion: HeadsDeliveryCompleteAction,
	task_state: TaskState,
) -> Vec<Action> {
	vec![completion_action(completion), task_complete(co, task_id, task_state)]
}

fn execution_completion(
	intent: &HeadsDeliveryIntent,
	attempted_heads: Option<BTreeSet<Cid>>,
	outcome: HeadsDeliveryOutcome,
) -> HeadsDeliveryCompleteAction {
	HeadsDeliveryCompleteAction {
		co: intent.co.clone(),
		recipient: intent.recipient.clone(),
		attempted_heads,
		phase: HeadsDeliveryPhase::Execution,
		outcome,
	}
}

async fn claimed_head_task(context: &CoContext, task_id: &str) -> Result<Option<Task>, anyhow::Error> {
	let local_co = context.local_co_reducer().await?;
	network_queue_task_in_list(&local_co, LIST_NAME_DOING, task_id).await
}

fn claimed_task_recipient(task_co: &CoId, task: Option<&Task>) -> Option<Did> {
	let task = task?;
	if task.tags.string("task-type") != Some(HEADS_DELIVERY_TASK_TYPE)
		|| task.tags.string("co") != Some(task_co.as_str())
	{
		return None;
	}
	task.tags.string("recipient").map(Did::from)
}

fn validate_claimed_head_task(
	task_co: &CoId,
	task_id: &str,
	payload: &Block,
	claimed_task: &Task,
	intent: &HeadsDeliveryIntent,
) -> Result<(), anyhow::Error> {
	anyhow::ensure!(claimed_task.id == task_id, "head delivery task ID does not match its claimed queue task");
	anyhow::ensure!(claimed_task.lock.is_some(), "head delivery queue task has no active claim");
	anyhow::ensure!(
		claimed_task.tags.string("task-type") == Some(HEADS_DELIVERY_TASK_TYPE),
		"head delivery task type does not match its claimed queue task"
	);
	anyhow::ensure!(
		claimed_task.tags.string("co") == Some(task_co.as_str()) && intent.co == *task_co,
		"head delivery task CO does not match its claimed queue task"
	);
	anyhow::ensure!(
		claimed_task.tags.string("recipient") == Some(intent.recipient.as_str()),
		"head delivery task recipient does not match its claimed queue task"
	);
	anyhow::ensure!(
		claimed_task.payload.as_ref() == Some(payload.cid()),
		"head delivery payload does not match its claimed queue task"
	);
	payload.clone().with_verify()?;
	Ok(())
}

fn malformed_execution_actions(
	task_co: CoId,
	task_id: String,
	claimed_task: Option<&Task>,
	error: ActionError,
) -> Vec<Action> {
	let mut actions = Vec::with_capacity(2);
	if let Some(recipient) = claimed_task_recipient(&task_co, claimed_task) {
		actions.push(completion_action(HeadsDeliveryCompleteAction {
			co: task_co.clone(),
			recipient,
			attempted_heads: None,
			phase: HeadsDeliveryPhase::Execution,
			outcome: HeadsDeliveryOutcome::Failed(error),
		}));
	}
	actions.push(task_complete(task_co, task_id, TaskState::Failed));
	actions
}

fn execute_queued(
	context: CoContext,
	task_co: CoId,
	task_id: String,
	task: Block,
) -> impl Stream<Item = Result<Action, anyhow::Error>> + Send + 'static {
	async_stream::stream! {
		let decoded = BlockSerializer::default().deserialize::<HeadsDeliveryIntent>(&task);
		let claimed_task = match claimed_head_task(&context, &task_id).await {
			Ok(Some(task)) => task,
			Ok(None) => {
				tracing::warn!(%task_id, "targeted-head-task-not-claimed");
				yield Ok(task_complete(task_co, task_id, TaskState::Failed));
				return;
			},
			Err(error) => {
				tracing::warn!(?error, %task_id, "targeted-head-claimed-task-lookup-failed");
				yield Ok(task_complete(task_co, task_id, TaskState::Backlog));
				return;
			},
		};
		let intent = match decoded {
			Ok(intent) => intent,
			Err(error) => {
				tracing::error!(?error, %task_id, "targeted-head-task-malformed");
				for action in malformed_execution_actions(
					task_co,
					task_id,
					Some(&claimed_task),
					anyhow::Error::from(error).into(),
				) {
					yield Ok(action);
				}
				return;
			},
		};
		if let Err(error) = validate_claimed_head_task(&task_co, &task_id, &task, &claimed_task, &intent) {
			tracing::error!(?error, %task_id, "targeted-head-task-claim-mismatch");
			for action in malformed_execution_actions(
				task_co,
				task_id,
				Some(&claimed_task),
				error.into(),
			) {
				yield Ok(action);
			}
			return;
		}
		let recipient = HeadsRecipient {
			did: intent.recipient.clone(),
			connectivity: intent.connectivity.clone(),
		};
		let authorization = authorize_recipient(&context, &intent.co, &intent.recipient).await;
		let (completion, task_state) = match authorization {
			Authorization::Denied => (
				execution_completion(&intent, None, HeadsDeliveryOutcome::Cancelled),
				TaskState::Delete,
			),
			Authorization::Permanent(error) => (
				execution_completion(&intent, None, HeadsDeliveryOutcome::Failed(error)),
				TaskState::Failed,
			),
			Authorization::Transient(error) => {
				tracing::warn!(?error, co = %intent.co, recipient = %intent.recipient, "targeted-head-guard-queued");
				(execution_completion(&intent, None, HeadsDeliveryOutcome::Queued), TaskState::Backlog)
			},
			Authorization::Allowed => {
				match prepare_delivery(&context, &intent.co, &intent.from, &recipient).await {
					Ok(PreparedDelivery { prepared, networks }) => {
						let attempted_heads = Some(prepared.heads.clone());
						match send_prepared(
							&context,
							&intent.co,
							&intent.from,
							&recipient,
							networks,
							&prepared,
						)
						.await
						{
							Ok(peers) => (
								execution_completion(
									&intent,
									attempted_heads,
									HeadsDeliveryOutcome::Delivered { peers },
								),
								TaskState::Delete,
							),
							Err(error) => {
								tracing::warn!(
									?error,
									co = %intent.co,
									recipient = %intent.recipient,
									"targeted-head-send-queued"
								);
								(
									execution_completion(&intent, attempted_heads, HeadsDeliveryOutcome::Queued),
									TaskState::Backlog,
								)
							},
						}
					},
					Err(AttemptFailure::Transient { heads, error }) => {
						tracing::warn!(
							?error,
							co = %intent.co,
							recipient = %intent.recipient,
							"targeted-head-prepare-queued"
						);
						(
							execution_completion(&intent, heads, HeadsDeliveryOutcome::Queued),
							TaskState::Backlog,
						)
					},
					Err(AttemptFailure::Permanent { heads, error }) => (
						execution_completion(&intent, heads, HeadsDeliveryOutcome::Failed(error)),
						TaskState::Failed,
					),
				}
			},
		};

		for action in execution_actions(task_co, task_id, completion, task_state) {
			yield Ok(action);
		}
	}
}

fn admission_completion(co: &CoId, recipient: Did, result: Result<(), ActionError>) -> Action {
	let outcome = match result {
		Ok(()) => HeadsDeliveryOutcome::Queued,
		Err(error) => HeadsDeliveryOutcome::Failed(error),
	};
	completion_action(HeadsDeliveryCompleteAction {
		co: co.clone(),
		recipient,
		attempted_heads: None,
		phase: HeadsDeliveryPhase::Admission,
		outcome,
	})
}

fn queue_process_after_admission(done: &HeadsDeliveryCompleteAction) -> Option<Action> {
	(done.phase == HeadsDeliveryPhase::Admission && matches!(&done.outcome, HeadsDeliveryOutcome::Queued))
		.then(|| Action::NetworkQueueProcess { co: Some(done.co.clone()), retry: 0 })
}

fn completion_action(done: HeadsDeliveryCompleteAction) -> Action {
	let (outcome, peer_count) = match &done.outcome {
		HeadsDeliveryOutcome::Delivered { peers } => ("delivered", peers.len()),
		HeadsDeliveryOutcome::Queued => ("queued", 0),
		HeadsDeliveryOutcome::Cancelled => ("cancelled", 0),
		HeadsDeliveryOutcome::Failed(_) => ("failed", 0),
	};
	tracing::info!(
		co = %done.co,
		recipient = %done.recipient,
		attempted_heads = ?done.attempted_heads,
		phase = ?done.phase,
		outcome,
		peer_count,
		"heads-delivery-complete"
	);
	Action::HeadsDeliveryComplete(done)
}

#[cfg(test)]
mod tests {
	use super::*;
	use co_core_board::Task;
	use co_core_co::{Participant, ParticipantState};
	use co_primitives::tags;

	#[test]
	fn only_active_participants_bypass_the_access_guard() {
		let did = Did::from("did:key:target");
		for state in [ParticipantState::Inactive, ParticipantState::Invite, ParticipantState::Pending] {
			let participant = Participant { did: did.clone(), state, tags: Default::default() };
			assert!(!contains_active_participant(&[participant], &did));
		}
		let active = Participant { did: did.clone(), state: ParticipantState::Active, tags: Default::default() };
		assert!(contains_active_participant(&[active], &did));
	}

	#[test]
	fn head_delivery_epic_has_no_process_local_state() {
		assert_eq!(std::mem::size_of::<HeadDeliveryEpic>(), 0);
	}

	#[test]
	fn persistence_failure_is_admission_failure_and_cannot_start_execution() {
		let action = admission_completion(
			&CoId::from("co-test"),
			Did::from("did:key:target"),
			Err(anyhow::anyhow!("persistence failed").into()),
		);
		let Action::HeadsDeliveryComplete(done) = action else {
			panic!("expected completion action");
		};
		assert_eq!(done.attempted_heads, None);
		assert_eq!(done.phase, HeadsDeliveryPhase::Admission);
		assert!(matches!(done.outcome, HeadsDeliveryOutcome::Failed(_)));
		assert!(queue_process_after_admission(&done).is_none());
	}

	#[test]
	fn malformed_task_recovers_truthful_recipient_from_claimed_tags() {
		let co = CoId::from("co-test");
		let recipient = Did::from("did:key:target");
		let task = Task {
			id: "task-id".to_owned(),
			name: "malformed head delivery".to_owned(),
			tags: tags!(
				"co": co.to_string(),
				"task-type": HEADS_DELIVERY_TASK_TYPE,
				"recipient": recipient.clone(),
			),
			payload: None,
			lock: Some("worker".to_owned()),
		};
		let actions = malformed_execution_actions(
			co.clone(),
			task.id.clone(),
			Some(&task),
			anyhow::anyhow!("decode failed").into(),
		);

		assert_eq!(actions.len(), 2);
		let Action::HeadsDeliveryComplete(done) = &actions[0] else {
			panic!("expected truthful execution completion first");
		};
		assert_eq!(done.co, co);
		assert_eq!(done.recipient, recipient);
		assert_eq!(done.phase, HeadsDeliveryPhase::Execution);
		assert!(matches!(done.outcome, HeadsDeliveryOutcome::Failed(_)));
		assert!(matches!(
			&actions[1],
			Action::NetworkTaskExecuteComplete { task_id, task_state: TaskState::Failed, .. }
				if task_id == "task-id"
		));
	}

	#[test]
	fn malformed_task_without_semantic_tags_emits_only_exact_task_failure() {
		let co = CoId::from("co-test");
		let task = Task {
			id: "task-id".to_owned(),
			name: "malformed head delivery".to_owned(),
			tags: tags!("task-type": HEADS_DELIVERY_TASK_TYPE),
			payload: None,
			lock: Some("worker".to_owned()),
		};
		let actions =
			malformed_execution_actions(co, task.id.clone(), Some(&task), anyhow::anyhow!("decode failed").into());

		assert_eq!(actions.len(), 1);
		assert!(matches!(
			&actions[0],
			Action::NetworkTaskExecuteComplete { task_id, task_state: TaskState::Failed, .. }
				if task_id == "task-id"
		));
	}
}
