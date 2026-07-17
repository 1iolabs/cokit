// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::{
	services::application::{CoDidCommSendAction, HeadsMessageReceivedAction},
	state,
	types::cores::CO_CORE_BOARD,
	Action, CoContext, CoReducer, CoreSource, Cores, CO_CORE_NAME_CO,
};
use anyhow::anyhow;
use co_core_board::{Board, BoardAction, List, Task, TaskLock, TaskTransition};
use co_core_co::{Co, CoAction};
use co_identity::{LocalIdentity, PrivateIdentityBox};
use co_primitives::{tag, tags, Block, CoId, CoTryStreamExt, CoreName, TagsExpr};
use co_storage::{BlockStorage, BlockStorageExt};
use futures::{pin_mut, Stream, TryStreamExt};
use std::future::ready;

pub const CO_CORE_NAME_NETWORK_QUEUE: CoreName<'static, Board> = CoreName::new("network_queue");
pub const LIST_NAME_BACKLOG: &str = "backlog";
pub const LIST_NAME_DOING: &str = "doing";
pub const LIST_NAME_FAILED: &str = "failed";
pub const LIST_NAME_DONE: &str = "done";

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum TaskState {
	Backlog,
	Doing,
	Failed,
	Done,
	Delete,
}
impl TaskState {
	pub fn list_name(&self) -> Option<&str> {
		match self {
			TaskState::Backlog => Some(LIST_NAME_BACKLOG),
			TaskState::Doing => Some(LIST_NAME_DOING),
			TaskState::Failed => Some(LIST_NAME_FAILED),
			TaskState::Done => Some(LIST_NAME_DONE),
			TaskState::Delete => None,
		}
	}
}

pub async fn network_queue_message(context: &CoContext, mut message: CoDidCommSendAction) -> Result<(), anyhow::Error> {
	let local_co = context.local_co_reducer().await?;
	let identity = context.local_identity();

	// create core
	let (storage, co) = local_co.co().await?;
	ensure_network_queue_core(context.cores(), &local_co, &identity, co).await?;

	// setup task id
	let task_id = message.message_header.id.clone();
	message.tags.insert(tag!("task_id": task_id.clone()));

	// insert message
	let payload = Some(storage.set_serialized(&message).await?);
	local_co
		.push(
			&identity,
			CO_CORE_NAME_NETWORK_QUEUE,
			&BoardAction::TaskCreate {
				list: LIST_NAME_BACKLOG.to_owned(),
				task: Task {
					id: task_id,
					name: format!("DIDComm {} to co:{}", message.message_header.id, &message.co),
					tags: tags!("co": message.co.to_string(), "type": "co-didcomm", "message_id": message.message_header.id),
					payload,
					lock: None,
				},
				after: None,
			},
		)
		.await?;

	// done
	Ok(())
}

pub async fn network_queue_heads(
	context: &CoContext,
	mut message: HeadsMessageReceivedAction,
) -> Result<(), anyhow::Error> {
	let local_co = context.local_co_reducer().await?;
	let identity = context.local_identity();

	// create core
	let (storage, co) = local_co.co().await?;
	ensure_network_queue_core(context.cores(), &local_co, &identity, co).await?;

	// setup task id
	//  TODO: SECURITY: we should not trust the task_id is random as is supplied from the network participant
	let task_id = message.message_id.clone();
	message.tags.insert(tag!("task_id": task_id.clone()));

	// insert message
	let payload = Some(storage.set_serialized(&message).await?);
	local_co
		.push(
			&identity,
			CO_CORE_NAME_NETWORK_QUEUE,
			&BoardAction::TaskCreate {
				list: LIST_NAME_BACKLOG.to_owned(),
				task: Task {
					id: task_id,
					name: format!("Heads message {} to co:{}", message.message_id, &message.co),
					tags: tags!("co": message.co.to_string(), "type": "co-heads/1.0", "message_id": message.message_id),
					payload,
					lock: None,
				},
				after: None,
			},
		)
		.await?;

	// done
	Ok(())
}

pub async fn network_queue_task(
	context: &CoContext,
	co_id: CoId,
	task_id: String,
	task_type: String,
	task_name: String,
	task: Block,
) -> Result<(), anyhow::Error> {
	let local_co = context.local_co_reducer().await?;
	let identity = context.local_identity();

	// create core
	let (storage, co) = local_co.co().await?;
	ensure_network_queue_core(context.cores(), &local_co, &identity, co).await?;

	// insert message
	let payload = Some(storage.set(task).await?);
	local_co
		.push(
			&identity,
			CO_CORE_NAME_NETWORK_QUEUE,
			&BoardAction::TaskCreate {
				list: LIST_NAME_BACKLOG.to_owned(),
				task: Task {
					id: task_id,
					name: task_name,
					tags: tags!("co": co_id.to_string(), "task-type": task_type),
					payload,
					lock: None,
				},
				after: None,
			},
		)
		.await?;

	// done
	Ok(())
}

pub trait ActionComplete: Send + Sync + 'static {
	fn is_complete(&self, action: &Action) -> Option<TaskState>;
	fn clone_box(&self) -> Box<dyn ActionComplete>;
}
impl<T> ActionComplete for T
where
	T: Fn(&Action) -> Option<TaskState> + Clone + Send + Sync + 'static,
{
	fn is_complete(&self, action: &Action) -> Option<TaskState> {
		self(action)
	}

	fn clone_box(&self) -> Box<dyn ActionComplete> {
		Box::new(self.clone())
	}
}
impl Clone for Box<dyn ActionComplete> {
	fn clone(&self) -> Self {
		self.clone_box()
	}
}

/// Get task action and complete trigger.
pub async fn network_queue_action(
	local_co: &CoReducer,
	task: &Task,
	lock_id: &str,
) -> Result<(Action, Box<dyn ActionComplete>), anyhow::Error> {
	let storage = local_co.storage();

	// execute
	if let (Some(co), Some(task_type)) = (task.tags.string("co"), task.tags.string("task-type")) {
		// complete
		let complete = {
			let complete_task_id = task.id.clone();
			move |action: &Action| -> Option<TaskState> {
				match action {
					Action::NetworkTaskExecuteComplete { co: _, task_id, task_state }
						if task_id == &complete_task_id =>
					{
						Some(*task_state)
					},
					_ => None,
				}
			}
		};

		// send
		let payload_reference = task.payload.ok_or(anyhow!("No payload"))?;
		let payload = storage.get(&payload_reference).await?;
		return Ok((
			Action::NetworkTaskExecute {
				co: CoId::new(co),
				task: payload,
				task_id: task.id.clone(),
				task_type: task_type.to_owned(),
			},
			Box::new(complete),
		));
	}

	// legacy
	match task.tags.string("type") {
		Some("co-didcomm") => {
			// complete
			let complete = {
				let complete_tags = tags!("task_id": task.id.clone(), "lock_id": lock_id);
				move |action: &Action| -> Option<TaskState> {
					match action {
						Action::CoDidCommSent { message, result } if message.tags.matches(&complete_tags) => {
							Some(match result {
								Ok(peers) if peers.is_empty() => TaskState::Backlog,
								Ok(_) => TaskState::Done,
								Err(_err) => TaskState::Failed,
							})
						},
						_ => None,
					}
				}
			};

			// send
			let payload_reference = task.payload.ok_or(anyhow!("No payload"))?;
			let mut payload: CoDidCommSendAction = storage.get_deserialized(&payload_reference).await?;
			payload.tags.insert(tag!("task_lock": lock_id));
			Ok((Action::CoDidCommSend(payload), Box::new(complete)))
		},
		Some("co-heads/1.0") => {
			// complete
			let complete = {
				let task_message_id = task.id.clone();
				move |action: &Action| -> Option<TaskState> {
					match action {
						Action::HeadsMessageComplete(HeadsMessageReceivedAction { message_id, .. }, result)
							if message_id == &task_message_id =>
						{
							Some(match result {
								Ok(_) => TaskState::Done,
								Err(_err) => TaskState::Failed,
							})
						},
						_ => None,
					}
				}
			};

			// send
			let payload_reference = task.payload.ok_or(anyhow!("No payload"))?;
			let mut payload: HeadsMessageReceivedAction = storage.get_deserialized(&payload_reference).await?;
			payload.tags.insert(tag!("task_lock": lock_id));
			Ok((Action::HeadsMessageReceived(payload), Box::new(complete)))
		},
		unknown => Err(anyhow!("Unknown task type: {:?}", unknown)),
	}
}

/// Move task to doing and lock it.
pub async fn network_queue_task_doing(
	identity: &PrivateIdentityBox,
	local_co: &CoReducer,
	task: &Task,
	lock_id: &str,
) -> Result<Option<Task>, anyhow::Error> {
	// move to doing
	local_co
		.push(
			identity,
			CO_CORE_NAME_NETWORK_QUEUE,
			&BoardAction::TaskMove {
				from_list: Some(LIST_NAME_BACKLOG.to_owned()),
				list: TaskState::Doing
					.list_name()
					.expect("doing tasks always have a queue list")
					.to_owned(),
				task: task.id.clone(),
				after: None,
				lock: TaskLock::Lock(lock_id.to_string()),
			},
		)
		.await?;

	// verify the exact post-claim task in the doing list. A task-map record alone
	// does not prove that this worker won the move or owns the claim.
	let Some(task) = network_queue_task_in_list(local_co, LIST_NAME_DOING, &task.id).await? else {
		return Ok(None);
	};
	Ok((task.lock.as_deref() == Some(lock_id)).then_some(task))
}

/// Settle a claimed task only if it is still in `doing` with this lock.
pub async fn network_queue_task_complete(
	identity: &PrivateIdentityBox,
	local_co: &CoReducer,
	task: &Task,
	lock_id: &str,
	to: TaskState,
	unless_matching: Option<TagsExpr>,
) -> Result<(), anyhow::Error> {
	let transition = match to {
		TaskState::Delete => TaskTransition::Delete,
		_ => TaskTransition::Move {
			list: to
				.list_name()
				.expect("non-delete task states always have a queue list")
				.to_owned(),
			after: None,
			unless_matching,
		},
	};
	let action = BoardAction::TaskCompleteIf {
		task: task.id.clone(),
		from_list: LIST_NAME_DOING.to_owned(),
		expected_lock: lock_id.to_owned(),
		transition,
	};
	local_co.push(identity, CO_CORE_NAME_NETWORK_QUEUE, &action).await?;
	Ok(())
}

/// Read current tasks from a queue list.
pub fn network_queue_tasks(
	context: CoContext,
	list_name: impl Into<String>,
	mut filter: impl FnMut(&Task) -> bool,
) -> impl Stream<Item = Result<Task, anyhow::Error>> {
	let list_name = list_name.into();
	async_stream::try_stream! {
		let local_co = context.local_co_reducer().await?;
		let storage = local_co.storage();
		let reducer_state = local_co.reducer_state().await;
		let tasks = state::board::tasks(
			storage.clone(),
			reducer_state,
			CO_CORE_NAME_NETWORK_QUEUE.to_string(),
			list_name,
		);
		pin_mut!(tasks);
		while let Some(task) = tasks.try_next().await? {
			// filter
			if !filter(&task) {
				continue;
			}

			// result
			yield task;
		}
	}
}

/// Read current backlog tasks.
pub fn network_queue_backlog(
	context: CoContext,
	filter: impl FnMut(&Task) -> bool,
) -> impl Stream<Item = Result<Task, anyhow::Error>> {
	network_queue_tasks(context, LIST_NAME_BACKLOG, filter)
}

/// Find a task in a queue list using the reducer's current state.
pub async fn network_queue_task_in_list(
	local_co: &CoReducer,
	list_name: &str,
	task_id: &str,
) -> Result<Option<Task>, anyhow::Error> {
	let storage = local_co.storage();
	let reducer_state = local_co.reducer_state().await;
	state::board::tasks(storage, reducer_state, CO_CORE_NAME_NETWORK_QUEUE.to_string(), list_name.to_owned())
		.try_filter(|task| ready(task.id == task_id))
		.try_first()
		.await
}

/// Create `network_queue` core if absent, or upgrade it to the built-in Board binary.
pub(crate) async fn ensure_network_queue_core(
	cores: &Cores,
	local_co: &CoReducer,
	identity: &LocalIdentity,
	co: Co,
) -> Result<(), anyhow::Error> {
	let expected_board_binary = cores
		.binary(CO_CORE_BOARD)
		.ok_or_else(|| anyhow!("Unknown built-in core: {}", CO_CORE_BOARD))?;
	match co.cores.get(CO_CORE_NAME_NETWORK_QUEUE.as_ref()) {
		Some(core) if core.binary == expected_board_binary => return Ok(()),
		Some(_) => {
			let board_binary = CoreSource::built_in(CO_CORE_BOARD).binary(&local_co.storage(), cores).await?;
			local_co
				.push(
					identity,
					CO_CORE_NAME_CO,
					&CoAction::CoreUpgrade {
						core: CO_CORE_NAME_NETWORK_QUEUE.to_string(),
						binary: board_binary,
						// Board, List, and Task serialization remains compatible.
						migrate: None,
					},
				)
				.await?;
		},
		None => {
			local_co
				.push(
					identity,
					CO_CORE_NAME_CO,
					&CoreSource::built_in(CO_CORE_BOARD)
						.to_core_create(
							&local_co.storage(),
							cores,
							CO_CORE_NAME_NETWORK_QUEUE,
							tags!( "core": CO_CORE_BOARD ),
						)
						.await?,
				)
				.await?;
			local_co
				.push(
					identity,
					CO_CORE_NAME_NETWORK_QUEUE,
					&BoardAction::ListCreate { list: List::new(LIST_NAME_BACKLOG), after: None },
				)
				.await?;
			local_co
				.push(
					identity,
					CO_CORE_NAME_NETWORK_QUEUE,
					&BoardAction::ListCreate { list: List::new(LIST_NAME_DOING), after: None },
				)
				.await?;
			local_co
				.push(
					identity,
					CO_CORE_NAME_NETWORK_QUEUE,
					&BoardAction::ListCreate { list: List::new(LIST_NAME_FAILED), after: None },
				)
				.await?;
			local_co
				.push(
					identity,
					CO_CORE_NAME_NETWORK_QUEUE,
					&BoardAction::ListCreate { list: List::new(LIST_NAME_DONE), after: None },
				)
				.await?;
		},
	}
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::{
		ensure_network_queue_core, network_queue_task, network_queue_task_complete, network_queue_task_doing,
		network_queue_task_in_list, network_queue_tasks, CoreSource, Cores, TaskState, CO_CORE_BOARD,
		CO_CORE_NAME_NETWORK_QUEUE, LIST_NAME_BACKLOG, LIST_NAME_DOING, LIST_NAME_DONE, LIST_NAME_FAILED,
	};
	use crate::{ApplicationBuilder, CoContext, CO_CORE_FILE, CO_CORE_NAME_CO};
	use co_core_board::{BoardAction, Task};
	use co_core_co::CoAction;
	use co_identity::PrivateIdentity;
	use co_primitives::{tags, BlockSerializer, TagsExpr};
	use co_storage::BlockStorageExt;
	use futures::TryStreamExt;

	#[tokio::test]
	async fn ensure_network_queue_core_upgrades_binary_without_losing_tasks() {
		let application = ApplicationBuilder::new_memory("network-queue-core-upgrade")
			.without_keychain()
			.build()
			.await
			.expect("application");
		let context = application.context();
		let local_co = application.local_co_reducer().await.expect("local co");
		let identity = application.local_identity();

		let (_, co) = local_co.co().await.expect("read local co");
		ensure_network_queue_core(context.cores(), &local_co, &identity, co)
			.await
			.expect("initialize network queue");

		let payload_value = "deliver persisted network message";
		let payload = local_co
			.storage()
			.set_serialized(&payload_value)
			.await
			.expect("store task payload");
		let expected_task = Task {
			id: "persisted-task".to_owned(),
			name: "Deliver persisted network message".to_owned(),
			tags: tags!("co": "co:example:recipient", "task-type": "message-delivery"),
			payload: Some(payload),
			lock: Some("worker-lock".to_owned()),
		};
		local_co
			.push(
				&identity,
				CO_CORE_NAME_NETWORK_QUEUE,
				&BoardAction::TaskCreate {
					list: LIST_NAME_BACKLOG.to_owned(),
					task: expected_task.clone(),
					after: None,
				},
			)
			.await
			.expect("create backlog task");

		let backlog_before = network_queue_tasks(context.clone(), LIST_NAME_BACKLOG, |_| true)
			.try_collect::<Vec<_>>()
			.await
			.expect("read backlog before upgrade");
		assert_eq!(backlog_before, vec![expected_task.clone()]);

		let board_binary = CoreSource::built_in(CO_CORE_BOARD)
			.binary(&local_co.storage(), context.cores())
			.await
			.expect("built-in board binary");
		let stale_binary = Cores::default().binary(CO_CORE_FILE).expect("built-in file binary");
		assert_ne!(stale_binary, board_binary, "stale binary must differ from Board");
		local_co
			.push(
				&identity,
				CO_CORE_NAME_CO,
				&CoAction::CoreUpgrade {
					core: CO_CORE_NAME_NETWORK_QUEUE.to_string(),
					binary: stale_binary,
					migrate: None,
				},
			)
			.await
			.expect("mark network queue binary stale");

		let (_, stale_co) = local_co.co().await.expect("read stale local co");
		assert_eq!(
			stale_co
				.cores
				.get(CO_CORE_NAME_NETWORK_QUEUE.as_ref())
				.expect("network queue core")
				.binary,
			stale_binary
		);
		ensure_network_queue_core(context.cores(), &local_co, &identity, stale_co)
			.await
			.expect("ensure network queue core");

		let (_, upgraded_co) = local_co.co().await.expect("read upgraded local co");
		assert_eq!(
			upgraded_co
				.cores
				.get(CO_CORE_NAME_NETWORK_QUEUE.as_ref())
				.expect("network queue core")
				.binary,
			board_binary
		);

		let backlog_after = network_queue_tasks(context.clone(), LIST_NAME_BACKLOG, |_| true)
			.try_collect::<Vec<_>>()
			.await
			.expect("read backlog after upgrade");
		assert_eq!(
			backlog_after.iter().map(|task| task.id.as_str()).collect::<Vec<_>>(),
			backlog_before.iter().map(|task| task.id.as_str()).collect::<Vec<_>>(),
			"backlog membership and order changed"
		);
		let actual_task = network_queue_task_in_list(&local_co, LIST_NAME_BACKLOG, &expected_task.id)
			.await
			.expect("read persisted task")
			.expect("persisted task remains in backlog");
		assert_eq!(actual_task, expected_task);
		assert_eq!(actual_task.payload, Some(payload));
		assert_eq!(actual_task.tags, tags!("co": "co:example:recipient", "task-type": "message-delivery"));
		assert_eq!(actual_task.lock.as_deref(), Some("worker-lock"));
		assert_eq!(
			local_co
				.storage()
				.get_deserialized::<String>(&payload)
				.await
				.expect("read persisted task payload"),
			payload_value
		);
	}

	async fn seed_queue_task(context: &CoContext, list: &str, task: Task) {
		let local_co = context.local_co_reducer().await.expect("local co");
		let identity = context.local_identity();
		let (_, co) = local_co.co().await.expect("read local co");
		ensure_network_queue_core(context.cores(), &local_co, &identity, co)
			.await
			.expect("initialize network queue");
		local_co
			.push(
				&identity,
				CO_CORE_NAME_NETWORK_QUEUE,
				&BoardAction::TaskCreate { list: list.to_owned(), task, after: None },
			)
			.await
			.expect("seed queue task");
	}

	async fn queue_contents(context: &CoContext) -> Vec<(String, Vec<Task>)> {
		let mut contents = Vec::new();
		for list in [LIST_NAME_BACKLOG, LIST_NAME_DOING, LIST_NAME_FAILED, LIST_NAME_DONE] {
			let tasks = network_queue_tasks(context.clone(), list, |_| true)
				.try_collect()
				.await
				.expect("read queue list");
			contents.push((list.to_owned(), tasks));
		}
		contents
	}

	#[tokio::test]
	async fn task_claim_returns_exact_post_claim_task_and_rejects_competing_snapshot() {
		let application = ApplicationBuilder::new_memory("network-queue-task-claim")
			.without_keychain()
			.build()
			.await
			.expect("application");
		let local_co = application.local_co_reducer().await.expect("local co");
		let identity = application.local_identity().boxed();
		let payload = BlockSerializer::default().serialize(&"payload").expect("payload");

		network_queue_task(
			application.context(),
			local_co.id().clone(),
			"claim-task".to_owned(),
			"test".to_owned(),
			"Claim task".to_owned(),
			payload,
		)
		.await
		.expect("enqueue task");
		let snapshot = network_queue_task_in_list(&local_co, LIST_NAME_BACKLOG, "claim-task")
			.await
			.expect("read backlog")
			.expect("task in backlog");

		let claimed = network_queue_task_doing(&identity, &local_co, &snapshot, "worker-a")
			.await
			.expect("claim task")
			.expect("claim won");
		let mut expected = snapshot.clone();
		expected.lock = Some("worker-a".to_owned());
		assert_eq!(claimed, expected);
		assert_eq!(
			network_queue_task_in_list(&local_co, LIST_NAME_DOING, "claim-task")
				.await
				.expect("read doing"),
			Some(claimed.clone())
		);
		assert!(network_queue_task_in_list(&local_co, LIST_NAME_BACKLOG, "claim-task")
			.await
			.expect("read backlog")
			.is_none());

		assert!(network_queue_task_doing(&identity, &local_co, &snapshot, "worker-b")
			.await
			.expect("competing claim")
			.is_none());
		let mut missing = snapshot;
		missing.id = "missing-task".to_owned();
		assert!(network_queue_task_doing(&identity, &local_co, &missing, "worker-c")
			.await
			.expect("missing claim")
			.is_none());
		assert_eq!(
			network_queue_task_in_list(&local_co, LIST_NAME_DOING, "claim-task")
				.await
				.expect("read doing"),
			Some(claimed)
		);
	}

	#[tokio::test]
	async fn stale_completions_are_successful_noops() {
		let application = ApplicationBuilder::new_memory("network-queue-stale-completion")
			.without_keychain()
			.build()
			.await
			.expect("application");
		let context = application.context();
		let local_co = application.local_co_reducer().await.expect("local co");
		let identity = application.local_identity().boxed();
		let payload = BlockSerializer::default().serialize(&"payload").expect("payload");

		network_queue_task(
			context,
			local_co.id().clone(),
			"claimed".to_owned(),
			"test".to_owned(),
			"Claimed task".to_owned(),
			payload,
		)
		.await
		.expect("enqueue task");
		let snapshot = network_queue_task_in_list(&local_co, LIST_NAME_BACKLOG, "claimed")
			.await
			.expect("read backlog")
			.expect("task in backlog");
		let claimed = network_queue_task_doing(&identity, &local_co, &snapshot, "worker-a")
			.await
			.expect("claim task")
			.expect("claim won");
		let wrong_source = Task {
			id: "wrong-source".to_owned(),
			name: "Wrong source".to_owned(),
			tags: tags!("kind": "test"),
			payload: None,
			lock: Some("source-lock".to_owned()),
		};
		seed_queue_task(context, LIST_NAME_FAILED, wrong_source.clone()).await;
		let unclaimed = Task {
			id: "unclaimed".to_owned(),
			name: "Missing claim".to_owned(),
			tags: tags!("kind": "test"),
			payload: None,
			lock: None,
		};
		seed_queue_task(context, LIST_NAME_DOING, unclaimed.clone()).await;
		let before = queue_contents(context).await;

		network_queue_task_complete(&identity, &local_co, &claimed, "worker-b", TaskState::Delete, None)
			.await
			.expect("wrong-lock completion is accepted");
		assert_eq!(queue_contents(context).await, before);

		network_queue_task_complete(&identity, &local_co, &wrong_source, "source-lock", TaskState::Delete, None)
			.await
			.expect("wrong-source completion is accepted");
		assert_eq!(queue_contents(context).await, before);

		network_queue_task_complete(&identity, &local_co, &unclaimed, "missing-lock", TaskState::Delete, None)
			.await
			.expect("missing-claim completion is accepted");
		assert_eq!(queue_contents(context).await, before);

		let mut missing = claimed;
		missing.id = "missing-task".to_owned();
		network_queue_task_complete(&identity, &local_co, &missing, "worker-a", TaskState::Delete, None)
			.await
			.expect("missing-task completion is accepted");
		assert_eq!(queue_contents(context).await, before);
	}

	#[tokio::test]
	async fn completion_to_backlog_deletes_claim_when_matching_successor_exists() {
		let application = ApplicationBuilder::new_memory("network-queue-conditional-completion")
			.without_keychain()
			.build()
			.await
			.expect("application");
		let context = application.context();
		let local_co = application.local_co_reducer().await.expect("local co");
		let identity = application.local_identity().boxed();
		let matching = TagsExpr::new("kind", "head-delivery");
		let old = Task {
			id: "old".to_owned(),
			name: "Old generation".to_owned(),
			tags: tags!("kind": "head-delivery"),
			payload: None,
			lock: None,
		};
		seed_queue_task(context, LIST_NAME_BACKLOG, old.clone()).await;
		let claimed = network_queue_task_doing(&identity, &local_co, &old, "worker")
			.await
			.expect("claim task")
			.expect("claim won");
		let successor = Task {
			id: "successor".to_owned(),
			name: "Fresh generation".to_owned(),
			tags: tags!("kind": "head-delivery"),
			payload: None,
			lock: None,
		};
		seed_queue_task(context, LIST_NAME_BACKLOG, successor.clone()).await;

		network_queue_task_complete(&identity, &local_co, &claimed, "worker", TaskState::Backlog, Some(matching))
			.await
			.expect("complete task");

		assert!(network_queue_task_in_list(&local_co, LIST_NAME_DOING, "old")
			.await
			.expect("read doing")
			.is_none());
		assert!(network_queue_task_in_list(&local_co, LIST_NAME_BACKLOG, "old")
			.await
			.expect("read backlog")
			.is_none());
		assert_eq!(
			network_queue_task_in_list(&local_co, LIST_NAME_BACKLOG, "successor")
				.await
				.expect("read backlog"),
			Some(successor)
		);
	}

	#[tokio::test]
	async fn task_completion_uses_doing_as_source_and_delete_removes_task() {
		let application = ApplicationBuilder::new_memory("network-queue-task-completion")
			.without_keychain()
			.build()
			.await
			.expect("application");
		let local_co = application.local_co_reducer().await.expect("local co");
		let identity = application.local_identity().boxed();
		let payload = BlockSerializer::default().serialize(&"payload").expect("payload");

		network_queue_task(
			application.context(),
			local_co.id().clone(),
			"task-1".to_owned(),
			"test".to_owned(),
			"Test task".to_owned(),
			payload,
		)
		.await
		.expect("enqueue task");
		let task = network_queue_task_in_list(&local_co, LIST_NAME_BACKLOG, "task-1")
			.await
			.expect("read backlog")
			.expect("task in backlog");

		let task = network_queue_task_doing(&identity, &local_co, &task, "lock-1")
			.await
			.expect("claim task")
			.expect("claim won");
		network_queue_task_complete(&identity, &local_co, &task, "lock-1", TaskState::Backlog, None)
			.await
			.expect("complete task to backlog");
		let task = network_queue_task_in_list(&local_co, LIST_NAME_BACKLOG, "task-1")
			.await
			.expect("read backlog")
			.expect("task returned to backlog");

		let task = network_queue_task_doing(&identity, &local_co, &task, "lock-2")
			.await
			.expect("claim task again")
			.expect("second claim won");
		network_queue_task_complete(&identity, &local_co, &task, "lock-2", TaskState::Delete, None)
			.await
			.expect("delete task");

		for list_name in [LIST_NAME_BACKLOG, LIST_NAME_DOING, LIST_NAME_FAILED, LIST_NAME_DONE] {
			assert!(
				network_queue_task_in_list(&local_co, list_name, "task-1")
					.await
					.expect("read queue list")
					.is_none(),
				"task remains in {list_name}"
			);
		}
	}
}
