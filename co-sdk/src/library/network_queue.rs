use crate::{
	services::application::{CoDidCommSendAction, HeadsMessageReceivedAction},
	state::{self, query_core, QueryExt},
	types::cores::CO_CORE_BOARD,
	Action, CoContext, CoReducer, Cores, CO_CORE_NAME_CO,
};
use anyhow::anyhow;
use co_core_board::{Board, BoardAction, List, Task, TaskLock};
use co_core_co::{Co, CoAction};
use co_identity::{LocalIdentity, PrivateIdentityBox};
use co_primitives::{tag, tags, Block, CoId, CoreName, DefaultParams};
use co_storage::{BlockStorage, BlockStorageExt};
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
	ensure_network_queue_core(&local_co, &identity, co).await?;

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
	task: Block<DefaultParams>,
) -> Result<(), anyhow::Error> {
	let local_co = context.local_co_reducer().await?;
	let identity = context.local_identity();

	// create core
	let (storage, co) = local_co.co().await?;
	ensure_network_queue_core(&local_co, &identity, co).await?;

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
	match (task.tags.string("co"), task.tags.string("task-type")) {
		(Some(co), Some(task_type)) => {
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
		},
		_ => {},
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
						Action::HeadsMessageComplete {
							message: HeadsMessageReceivedAction { message_id, .. },
							result,
						} if message_id == &task_message_id => Some(match result {
							Ok(_) => TaskState::Done,
							Err(_err) => TaskState::Failed,
						}),
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
