use crate::{
	services::application::CoDidCommSendAction,
	state::{self, query_core, QueryExt},
	types::cores::CO_CORE_BOARD,
	Action, CoContext, CoReducer, Cores, CO_CORE_NAME_CO,
};
use anyhow::anyhow;
use co_core_board::{Board, BoardAction, List, Task, TaskLock};
use co_core_co::{Co, CoAction};
use co_identity::{LocalIdentity, PrivateIdentityBox};
use co_primitives::{tag, tags, CoreName};
use co_storage::BlockStorageExt;
use futures::{pin_mut, Stream, TryStreamExt};

pub const CO_CORE_NAME_NETWORK_QUEUE: CoreName<'static, Board> = CoreName::new("network_queue");
pub const LIST_NAME_BACKLOG: &str = "backlog";
pub const LIST_NAME_DOING: &str = "doing";
pub const LIST_NAME_FAILED: &str = "failed";
pub const LIST_NAME_DONE: &str = "done";

#[derive(Debug, Copy, Clone)]
pub enum TaskState {
	Backlog,
	Doing,
	Failed,
	Done,
}
impl TaskState {
	pub fn to_list_name(&self) -> &str {
		match self {
			TaskState::Backlog => LIST_NAME_BACKLOG,
			TaskState::Doing => LIST_NAME_DOING,
			TaskState::Failed => LIST_NAME_FAILED,
			TaskState::Done => LIST_NAME_DONE,
		}
	}
}

pub async fn network_queue_message(context: &CoContext, mut message: CoDidCommSendAction) -> Result<(), anyhow::Error> {
	let local_co = context.local_co_reducer().await?;
	let identity = context.local_identity();

	// create core
	let (storage, co) = local_co.co().await?;
	ensure_network_queue_core(&local_co, &identity, co).await?;

	// setup task id
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
					name: format!("DIDComm {} to co:{}", message.message_id, &message.co),
					tags: tags!("co": message.co.to_string(), "type": "co-didcomm", "message_id": message.message_id),
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

/// Get task action and complete trigger.
pub async fn network_queue_action(
	local_co: &CoReducer,
	task: &Task,
	lock_id: &str,
) -> Result<(Action, impl Fn(&Action) -> Option<TaskState> + Clone + Send + 'static), anyhow::Error> {
	let storage = local_co.storage();
	match task.tags.string("type") {
		Some("co-didcomm") => {
			// complete
			let complete_tags = tags!("task_id": task.id.clone(), "lock_id": lock_id);
			let complete = move |action: &Action| -> Option<TaskState> {
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
			};

			// send
			let payload_reference = task.payload.ok_or(anyhow!("No payload"))?;
			let mut payload: CoDidCommSendAction = storage.get_deserialized(&payload_reference).await?;
			payload.tags.insert(tag!("task_lock": lock_id));
			Ok((Action::CoDidCommSend(payload), complete))
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
) -> Result<bool, anyhow::Error> {
	// move to doing
	local_co
		.push(
			identity,
			CO_CORE_NAME_NETWORK_QUEUE,
			&BoardAction::TaskMove {
				from_list: Some(LIST_NAME_BACKLOG.to_owned()),
				list: TaskState::Doing.to_list_name().to_string(),
				task: task.id.clone(),
				after: None,
				lock: TaskLock::Lock(lock_id.to_string()),
			},
		)
		.await?;

	// verify that we locked the item
	let (_, Some(task)) = query_core(CO_CORE_NAME_NETWORK_QUEUE)
		.with_default()
		.map(|board| board.tasks)
		.get_value(task.id.clone())
		.execute_reducer(&local_co)
		.await?
	else {
		return Ok(false);
	};
	if let Some(task_lock) = &task.lock {
		if lock_id != task_lock {
			return Ok(false);
		}
	}

	// result
	Ok(true)
}

/// Move task to doing and lock it.
pub async fn network_queue_task_complete(
	identity: &PrivateIdentityBox,
	local_co: &CoReducer,
	task: &Task,
	lock_id: &str,
	to: TaskState,
) -> Result<(), anyhow::Error> {
	local_co
		.push(
			identity,
			CO_CORE_NAME_NETWORK_QUEUE,
			&BoardAction::TaskMove {
				from_list: Some(LIST_NAME_BACKLOG.to_owned()),
				list: to.to_list_name().to_string(),
				task: task.id.clone(),
				after: None,
				lock: TaskLock::Unlock(lock_id.to_string()),
			},
		)
		.await?;
	Ok(())
}

/// Read current backlog tasks.
pub fn network_queue_backlog(
	context: CoContext,
	mut filter: impl FnMut(&Task) -> bool,
) -> impl Stream<Item = Result<Task, anyhow::Error>> {
	async_stream::try_stream! {
		let local_co = context.local_co_reducer().await?;
		let storage = local_co.storage();
		let reducer_state = local_co.reducer_state().await;
		let tasks = state::board::tasks(
			storage.clone(),
			reducer_state,
			CO_CORE_NAME_NETWORK_QUEUE.to_string(),
			LIST_NAME_BACKLOG.to_owned(),
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

/// Create `network_queue` core if not exists yet.
async fn ensure_network_queue_core(
	local_co: &CoReducer,
	identity: &LocalIdentity,
	co: Co,
) -> Result<(), anyhow::Error> {
	Ok(if co.cores.get(CO_CORE_NAME_NETWORK_QUEUE.as_ref()).is_none() {
		local_co
			.push(
				identity,
				CO_CORE_NAME_CO,
				&CoAction::CoreCreate {
					core: CO_CORE_NAME_NETWORK_QUEUE.to_string(),
					binary: Cores::default().binary(CO_CORE_BOARD).expect(CO_CORE_BOARD),
					tags: tags!( "core": CO_CORE_BOARD ),
				},
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
	})
}
