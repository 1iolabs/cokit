// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use anyhow::anyhow;
use cid::Cid;
use co_api::{
	co, BlockStorage, BlockStorageExt, CoList, CoListIndex, CoMap, CoTryStreamExt, CoreBlockStorage, IsDefault,
	LazyTransaction, Link, OptionLink, Reducer, ReducerAction, Tags, TagsAction,
};
use futures::{pin_mut, FutureExt, TryStreamExt};
use std::future::ready;

pub type ListName = String;
pub type TaskId = String;

/// Board actions.
#[co]
pub enum BoardAction {
	BoardRename(String),
	#[deprecated(note = "use `BoardTags` with a `TagsAction`")]
	BoardTagsInsert(Tags),
	#[deprecated(note = "use `BoardTags` with a `TagsAction`")]
	BoardTagsRemove(Tags),
	ListCreate {
		list: List,
		after: Option<ListName>,
	},
	ListArrange {
		name: ListName,
		after: Option<ListName>,
	},
	ListDelete {
		name: ListName,
		move_tasks_to_list: Option<ListName>,
	},
	#[deprecated(note = "use `ListTags` with a `TagsAction`")]
	ListTagsInsert(ListName, Tags),
	#[deprecated(note = "use `ListTags` with a `TagsAction`")]
	ListTagsRemove(ListName, Tags),
	// ListTasksDelete(ListName),
	// ListTasksMove { from: ListName, to: ListName },
	TaskCreate {
		list: ListName,
		task: Task,
		after: Option<TaskId>,
	},
	TaskMove {
		from_list: Option<ListName>,
		list: ListName,
		task: TaskId,
		after: Option<TaskId>,
		lock: TaskLock,
	},
	TaskArrange {
		task: TaskId,
		after: Option<TaskId>,
	},
	TaskDelete(TaskId),
	TaskRename(TaskId, String),
	TaskPayloadChange(TaskId, Option<Cid>),
	#[deprecated(note = "use `TaskTags` with a `TagsAction`")]
	TaskTagsInsert(TaskId, Tags),
	#[deprecated(note = "use `TaskTags` with a `TagsAction`")]
	TaskTagsRemove(TaskId, Tags),
	/// apply a [`TagsAction`] to the board tags.
	BoardTags(TagsAction),
	/// apply a [`TagsAction`] to a list's tags.
	ListTags(ListName, TagsAction),
	/// apply a [`TagsAction`] to a task's tags.
	TaskTags(TaskId, TagsAction),
}

#[co(state)]
pub struct Board {
	/// Board name.
	#[serde(rename = "n", default, skip_serializing_if = "String::is_empty")]
	pub name: String,

	/// Board lists.
	#[serde(rename = "l", default, skip_serializing_if = "CoList::is_empty")]
	pub lists: CoList<List>,

	/// Board tags.
	#[serde(rename = "t", default, skip_serializing_if = "Tags::is_empty")]
	pub tags: Tags,

	/// Board tasks.
	#[serde(rename = "i", default, skip_serializing_if = "CoMap::is_empty")]
	pub tasks: CoMap<TaskId, Task>,
}
impl Reducer<BoardAction> for Board {
	async fn reduce(
		state_link: OptionLink<Self>,
		event_link: Link<ReducerAction<BoardAction>>,
		storage: &CoreBlockStorage,
	) -> Result<Link<Self>, anyhow::Error> {
		let event = storage.get_value(&event_link).await?;
		let mut state = storage.get_value_or_default(&state_link).await?;
		reduce(storage, &mut state, event.payload).await?;
		Ok(storage.set_value(&state).await?)
	}
}

#[co]
pub struct List {
	/// List name.
	#[serde(rename = "n")]
	pub name: ListName,

	/// List tasks.
	#[serde(rename = "i", default, skip_serializing_if = "CoList::is_empty")]
	pub tasks: CoList<TaskId>,

	/// List tags.
	#[serde(rename = "t", default, skip_serializing_if = "Tags::is_empty")]
	pub tags: Tags,
}
impl List {
	pub fn new(name: impl Into<ListName>) -> Self {
		Self { name: name.into(), tags: Default::default(), tasks: Default::default() }
	}
}

#[co]
pub struct Task {
	/// Task unique id.
	#[serde(rename = "u")]
	pub id: TaskId,

	/// Task name.
	#[serde(rename = "n")]
	pub name: String,

	/// Task tags.
	#[serde(rename = "t", default, skip_serializing_if = "Tags::is_empty")]
	pub tags: Tags,

	/// Task payload.
	#[serde(rename = "p", default, skip_serializing_if = "Option::is_none")]
	pub payload: Option<Cid>,

	/// Task exclusive lock identifier.
	#[serde(rename = "l", default, skip_serializing_if = "IsDefault::is_default")]
	pub lock: Option<String>,
}

#[co]
#[derive(Default)]
pub enum TaskLock {
	/// No lock.
	/// Fail the operation if the subject is locked.
	#[default]
	None,

	/// Force operation if the subject is locked.
	Force,

	/// Use or apply a lock.
	/// Fail the operation if the subject is locked with a different lock.
	Lock(String),

	/// Use and unlock after the operation.
	/// Fail the operation if the subject is locked with a different lock.
	Unlock(String),
}

#[allow(deprecated)]
async fn reduce<S>(storage: &S, state: &mut Board, action: BoardAction) -> Result<(), anyhow::Error>
where
	S: BlockStorage + Clone + 'static,
{
	// open
	let mut transaction = BoardTransaction {
		storage: storage.clone(),
		lists: LazyTransaction::new(storage.clone(), state.lists.clone()),
		tasks: LazyTransaction::new(storage.clone(), state.tasks.clone()),
	};

	// reduce
	match action {
		BoardAction::BoardRename(name) => reduce_board_rename(state, name).boxed().await?,
		BoardAction::BoardTagsInsert(tags) => reduce_board_tags_insert(state, tags).boxed().await?,
		BoardAction::BoardTagsRemove(tags) => reduce_board_tags_remove(state, tags).boxed().await?,
		BoardAction::ListCreate { list, after } => reduce_list_create(&mut transaction, list, after).boxed().await?,
		BoardAction::ListArrange { name, after } => reduce_list_arrange(&mut transaction, name, after).boxed().await?,
		BoardAction::ListDelete { name, move_tasks_to_list } => {
			reduce_list_delete(&mut transaction, name, move_tasks_to_list).boxed().await?
		},
		BoardAction::ListTagsInsert(name, tags) => {
			reduce_list_tags_insert(&mut transaction, name, tags).boxed().await?
		},
		BoardAction::ListTagsRemove(name, tags) => {
			reduce_list_tags_remove(&mut transaction, name, tags).boxed().await?
		},
		BoardAction::TaskCreate { list, task, after } => {
			reduce_task_create(&mut transaction, list, task, after).boxed().await?
		},
		BoardAction::TaskMove { from_list, list, task, after, lock } => {
			reduce_task_move(&mut transaction, from_list, list, task, after, lock)
				.boxed()
				.await?
		},
		BoardAction::TaskArrange { task, after } => reduce_task_arrange(&mut transaction, task, after).boxed().await?,
		BoardAction::TaskDelete(task) => reduce_task_delete(&mut transaction, task).boxed().await?,
		BoardAction::TaskRename(task, name) => reduce_task_rename(&mut transaction, task, name).boxed().await?,
		BoardAction::TaskPayloadChange(task, cid) => {
			reduce_task_payload_change(&mut transaction, task, cid).boxed().await?
		},
		BoardAction::TaskTagsInsert(task, tags) => {
			reduce_task_tags_insert(&mut transaction, task, tags).boxed().await?
		},
		BoardAction::TaskTagsRemove(task, tags) => {
			reduce_task_tags_remove(&mut transaction, task, tags).boxed().await?
		},
		BoardAction::BoardTags(action) => reduce_board_tags(state, action).boxed().await?,
		BoardAction::ListTags(name, action) => reduce_list_tags(&mut transaction, name, action).boxed().await?,
		BoardAction::TaskTags(task, action) => reduce_task_tags(&mut transaction, task, action).boxed().await?,
	}

	// store
	if transaction.lists.is_mut_access() {
		state.lists = transaction.lists.get_mut().await?.store().await?;
	}
	if transaction.tasks.is_mut_access() {
		state.tasks = transaction.tasks.get_mut().await?.store().await?;
	}

	// result
	Ok(())
}

struct BoardTransaction<S>
where
	S: BlockStorage + Clone + 'static,
{
	storage: S,
	lists: LazyTransaction<S, CoList<List>>,
	tasks: LazyTransaction<S, CoMap<TaskId, Task>>,
}
impl<S> BoardTransaction<S>
where
	S: BlockStorage + Clone + 'static,
{
	/// Find list by name by scanning lists.
	async fn find_list_by_name(&mut self, name: &str) -> Result<Option<(CoListIndex, List)>, anyhow::Error> {
		Ok(self
			.lists
			.get()
			.await?
			.stream()
			.try_filter(|item| ready(item.1.name == name))
			.try_first()
			.await?)
	}

	/// Fint task's list by scanning all lists.
	async fn find_task_list(&mut self, id: &TaskId) -> Result<Option<(CoListIndex, List, CoListIndex)>, anyhow::Error> {
		Ok(self
			.lists
			.get()
			.await?
			.stream()
			.try_filter_map(|(index, list)| {
				let storage = self.storage.clone();
				async move {
					Ok(list
						.tasks
						.stream(&storage)
						.try_filter(|(_, task)| ready(task == id))
						.try_first()
						.await?
						.map(|(task_index, _task_id)| (index, list, task_index)))
				}
			})
			.try_first()
			.await?)
	}

	/// Get task by id.
	async fn task(&mut self, task_id: &TaskId) -> Result<Task, anyhow::Error> {
		self.tasks
			.get()
			.await?
			.get(task_id)
			.await?
			.ok_or_else(|| anyhow!("Task not found: {}", task_id))
	}
}

async fn reduce_task_tags_remove<S: BlockStorage + Clone + 'static>(
	transaction: &mut BoardTransaction<S>,
	task_id: TaskId,
	tags: Tags,
) -> Result<(), anyhow::Error> {
	let mut task = transaction
		.tasks
		.get()
		.await?
		.get(&task_id)
		.await?
		.ok_or(anyhow!("Task not found: {}", task_id))?;

	// apply
	task.tags.clear(Some(&tags));

	// store
	transaction.tasks.get_mut().await?.insert(task_id, task).await?;

	Ok(())
}

async fn reduce_task_tags_insert<S: BlockStorage + Clone + 'static>(
	transaction: &mut BoardTransaction<S>,
	task_id: TaskId,
	mut tags: Tags,
) -> Result<(), anyhow::Error> {
	let mut task = transaction
		.tasks
		.get()
		.await?
		.get(&task_id)
		.await?
		.ok_or(anyhow!("Task not found: {}", task_id))?;

	// apply
	task.tags.append(&mut tags);

	// store
	transaction.tasks.get_mut().await?.insert(task_id, task).await?;

	Ok(())
}

async fn reduce_task_payload_change<S: BlockStorage + Clone + 'static>(
	transaction: &mut BoardTransaction<S>,
	task_id: TaskId,
	payload: Option<Cid>,
) -> Result<(), anyhow::Error> {
	let mut task = transaction
		.tasks
		.get()
		.await?
		.get(&task_id)
		.await?
		.ok_or(anyhow!("Task not found: {}", task_id))?;

	// apply
	if task.payload != payload {
		// set
		task.payload = payload;

		// store
		transaction.tasks.get_mut().await?.insert(task_id, task).await?;
	}
	Ok(())
}

async fn reduce_task_rename<S: BlockStorage + Clone + 'static>(
	transaction: &mut BoardTransaction<S>,
	task_id: TaskId,
	name: String,
) -> Result<(), anyhow::Error> {
	let mut task = transaction
		.tasks
		.get()
		.await?
		.get(&task_id)
		.await?
		.ok_or(anyhow!("Task not found: {}", task_id))?;

	// apply
	if task.name != name {
		// set
		task.name = name;

		// store
		transaction.tasks.get_mut().await?.insert(task_id, task).await?;
	}
	Ok(())
}

async fn reduce_task_delete<S: BlockStorage + Clone + 'static>(
	transaction: &mut BoardTransaction<S>,
	task_id: TaskId,
) -> Result<(), anyhow::Error> {
	// find task list
	let (list_index, mut list, task_index) = transaction
		.find_task_list(&task_id)
		.await?
		.ok_or(anyhow!("Task list not found: {}", task_id))?;

	// remove
	transaction
		.tasks
		.get_mut()
		.await?
		.remove(task_id.clone())
		.await?
		.ok_or(anyhow!("Task not found: {}", task_id))?;

	// remove from list
	list.tasks.remove(&transaction.storage, task_index).await?;

	// store list
	transaction.lists.get_mut().await?.set(list_index, list).await?;

	Ok(())
}

async fn reduce_task_arrange<S: BlockStorage + Clone + 'static>(
	transaction: &mut BoardTransaction<S>,
	task_id: TaskId,
	after: Option<TaskId>,
) -> Result<(), anyhow::Error> {
	// find task list
	let (list_index, mut list, task_index) = transaction
		.find_task_list(&task_id)
		.await?
		.ok_or(anyhow!("Task list not found: {}", task_id))?;

	// after index
	let mut list_tasks = list.tasks.open(&transaction.storage).await?;
	let task_after_index = if let Some(after) = &after {
		list_tasks
			.stream()
			.try_filter(|item| ready(&item.1 == after))
			.try_first()
			.await?
			.map(|(index, _)| index)
	} else {
		None
	};

	// remove
	list_tasks.remove(task_index).await?;

	// insert
	if let Some(task_after_index) = task_after_index {
		list_tasks.insert(task_after_index, task_id).await?;
	} else {
		list_tasks.push(task_id).await?;
	}

	// store list
	list.tasks = list_tasks.store().await?;
	transaction.lists.get_mut().await?.set(list_index, list).await?;

	Ok(())
}

async fn reduce_task_move<S: BlockStorage + Clone + 'static>(
	transaction: &mut BoardTransaction<S>,
	from_list: Option<ListName>,
	list_name: ListName,
	task_id: TaskId,
	after: Option<TaskId>,
	lock: TaskLock,
) -> Result<(), anyhow::Error> {
	// lock
	task_lock(transaction, &task_id, &lock).await?;

	// find source list and source list task index
	let (source_list_index, mut source_list, mut source_list_tasks, source_task_index) =
		if let Some(from_list) = &from_list {
			let (source_list_index, source_list) = transaction
				.find_list_by_name(from_list)
				.await?
				.ok_or(anyhow!("List not found: {}", from_list))?;
			let list_tasks = source_list.tasks.open(&transaction.storage).await?;
			let source_task_index = list_tasks
				.stream()
				.try_filter(|item| ready(item.1 == task_id))
				.try_first()
				.await?
				.map(|(index, _)| index)
				.ok_or(anyhow!("Task not found: {} in list: {}", task_id, source_list.name))?;
			(source_list_index, source_list, list_tasks, source_task_index)
		} else {
			let (source_list_index, source_list, source_task_index) = transaction
				.find_task_list(&task_id)
				.await?
				.ok_or(anyhow!("Task list not found: {}", task_id))?;
			let list_tasks = source_list.tasks.open(&transaction.storage).await?;
			(source_list_index, source_list, list_tasks, source_task_index)
		};

	// find target list
	let (list_index, mut list) = transaction
		.find_list_by_name(&list_name)
		.await?
		.ok_or(anyhow!("List not found: {}", list_name))?;
	let mut list_tasks = list.tasks.open(&transaction.storage).await?;

	// find target list index
	let task_after_index = if let Some(after) = &after {
		list_tasks
			.stream()
			.try_filter(|item| ready(&item.1 == after))
			.try_first()
			.await?
			.map(|(index, _)| index)
	} else {
		None
	};

	// remove task from source list
	source_list_tasks.remove(source_task_index).await?;

	// add task to target list
	if let Some(task_after_index) = task_after_index {
		list_tasks.insert(task_after_index, task_id.clone()).await?;
	} else {
		list_tasks.push(task_id.clone()).await?;
	}

	// store
	source_list.tasks = source_list_tasks.store().await?;
	transaction.lists.get_mut().await?.set(source_list_index, source_list).await?;
	list.tasks = list_tasks.store().await?;
	transaction.lists.get_mut().await?.set(list_index, list).await?;

	// result
	Ok(())
}

async fn reduce_task_create<S: BlockStorage + Clone + 'static>(
	transaction: &mut BoardTransaction<S>,
	list: ListName,
	task: Task,
	after: Option<TaskId>,
) -> Result<(), anyhow::Error> {
	let task_id = task.id.clone();

	// find list
	let (list_index, mut list) = transaction
		.find_list_by_name(&list)
		.await?
		.ok_or(anyhow!("List not found: {}", list))?;

	// validate id is unique
	if transaction.tasks.get().await?.contains_key(&task_id).await? {
		return Err(anyhow!("Task exists: {}", task_id));
	}

	// create task
	transaction.tasks.get_mut().await?.insert(task_id.clone(), task).await?;

	// add to list
	let mut list_tasks = list.tasks.open(&transaction.storage).await?;
	let task_after_index = if let Some(after) = &after {
		list_tasks
			.stream()
			.try_filter(|item| ready(&item.1 == after))
			.try_first()
			.await?
			.map(|(index, _)| index)
	} else {
		None
	};
	if let Some(task_after_index) = task_after_index {
		list_tasks.insert(task_after_index, task_id).await?;
	} else {
		list_tasks.push(task_id).await?;
	}

	// store list
	list.tasks = list_tasks.store().await?;
	transaction.lists.get_mut().await?.set(list_index, list).await?;

	Ok(())
}

async fn reduce_list_tags_remove<S: BlockStorage + Clone + 'static>(
	transaction: &mut BoardTransaction<S>,
	name: String,
	tags: Tags,
) -> Result<(), anyhow::Error> {
	// find
	let (list_index, mut list) = transaction
		.find_list_by_name(&name)
		.await?
		.ok_or(anyhow!("List not found: {}", name))?;

	// insert
	list.tags.clear(Some(&tags));

	// store
	transaction.lists.get_mut().await?.set(list_index, list).await?;
	Ok(())
}

async fn reduce_list_tags_insert<S: BlockStorage + Clone + 'static>(
	transaction: &mut BoardTransaction<S>,
	name: String,
	mut tags: Tags,
) -> Result<(), anyhow::Error> {
	// find
	let (list_index, mut list) = transaction
		.find_list_by_name(&name)
		.await?
		.ok_or(anyhow!("List not found: {}", name))?;

	// insert
	list.tags.append(&mut tags);

	// store
	transaction.lists.get_mut().await?.set(list_index, list).await?;
	Ok(())
}

async fn reduce_list_delete<S: BlockStorage + Clone + 'static>(
	transaction: &mut BoardTransaction<S>,
	name: String,
	move_tasks_to_list: Option<String>,
) -> Result<(), anyhow::Error> {
	// find
	let (list_index, list) = transaction
		.find_list_by_name(&name)
		.await?
		.ok_or(anyhow!("List not found: {}", name))?;

	// move tasks
	if let Some(move_tasks_to_list) = &move_tasks_to_list {
		let (_to_list_index, to_list) = transaction
			.find_list_by_name(move_tasks_to_list)
			.await?
			.ok_or(anyhow!("List not found: {}", name))?;
		if !list.tasks.is_empty() {
			let storage = transaction.storage.clone();
			let tasks = list.tasks.clone();
			let tasks = tasks.stream(&storage);
			pin_mut!(tasks);
			while let Some((_, task)) = tasks.try_next().await? {
				reduce_task_move(transaction, None, to_list.name.clone(), task, None, TaskLock::Force).await?;
			}
		}
	}

	// delete list
	transaction.lists.get_mut().await?.remove(list_index).await?;

	Ok(())
}

async fn reduce_list_arrange<S: BlockStorage + Clone + 'static>(
	transaction: &mut BoardTransaction<S>,
	name: String,
	after: Option<String>,
) -> Result<(), anyhow::Error> {
	// find
	let (list_index, list) = transaction
		.find_list_by_name(&name)
		.await?
		.ok_or(anyhow!("List not found: {}", name))?;

	// find after
	let after_index = if let Some(after) = &after {
		transaction.find_list_by_name(after).await?.map(|(index, _)| index)
	} else {
		None
	};

	// remove
	transaction.lists.get_mut().await?.remove(list_index).await?;

	// create
	if let Some(after_index) = after_index {
		transaction.lists.get_mut().await?.insert(after_index, list).await?;
	} else {
		transaction.lists.get_mut().await?.push(list).await?;
	}
	Ok(())
}

async fn reduce_list_create<S: BlockStorage + Clone + 'static>(
	transaction: &mut BoardTransaction<S>,
	list: List,
	after: Option<String>,
) -> Result<(), anyhow::Error> {
	// verify name not exists yet
	if transaction.find_list_by_name(&list.name).await?.is_some() {
		return Err(anyhow!("List already exists: {}", list.name));
	}

	// find after
	let after_index = if let Some(after) = &after {
		transaction.find_list_by_name(after).await?.map(|(index, _)| index)
	} else {
		None
	};

	// create
	if let Some(after_index) = after_index {
		transaction.lists.get_mut().await?.insert(after_index, list).await?;
	} else {
		transaction.lists.get_mut().await?.push(list).await?;
	}
	Ok(())
}

async fn reduce_board_tags_remove(state: &mut Board, tags: Tags) -> Result<(), anyhow::Error> {
	state.tags.clear(Some(&tags));
	Ok(())
}

async fn reduce_board_tags_insert(state: &mut Board, mut tags: Tags) -> Result<(), anyhow::Error> {
	state.tags.append(&mut tags);
	Ok(())
}

async fn reduce_board_rename(state: &mut Board, name: String) -> Result<(), anyhow::Error> {
	state.name = name;
	Ok(())
}

async fn reduce_board_tags(state: &mut Board, action: TagsAction) -> Result<(), anyhow::Error> {
	state.tags.reduce(action);
	Ok(())
}

async fn reduce_list_tags<S: BlockStorage + Clone + 'static>(
	transaction: &mut BoardTransaction<S>,
	name: String,
	action: TagsAction,
) -> Result<(), anyhow::Error> {
	// find
	let (list_index, mut list) = transaction
		.find_list_by_name(&name)
		.await?
		.ok_or(anyhow!("List not found: {}", name))?;

	// apply
	list.tags.reduce(action);

	// store
	transaction.lists.get_mut().await?.set(list_index, list).await?;
	Ok(())
}

async fn reduce_task_tags<S: BlockStorage + Clone + 'static>(
	transaction: &mut BoardTransaction<S>,
	task_id: TaskId,
	action: TagsAction,
) -> Result<(), anyhow::Error> {
	let mut task = transaction
		.tasks
		.get()
		.await?
		.get(&task_id)
		.await?
		.ok_or(anyhow!("Task not found: {}", task_id))?;

	// apply
	task.tags.reduce(action);

	// store
	transaction.tasks.get_mut().await?.insert(task_id, task).await?;
	Ok(())
}

async fn task_lock<S: BlockStorage + Clone + 'static>(
	transaction: &mut BoardTransaction<S>,
	task_id: &TaskId,
	lock: &TaskLock,
) -> Result<(), anyhow::Error> {
	match lock {
		TaskLock::None => {
			let task = transaction.task(task_id).await?;
			if task.lock.is_some() {
				Err(anyhow!("Task locked"))
			} else {
				Ok(())
			}
		},
		TaskLock::Force => Ok(()),
		TaskLock::Lock(lock) => {
			let mut task = transaction.task(task_id).await?;
			match task.lock {
				Some(task_lock) if lock == &task_lock => Ok(()),
				Some(_task_lock) => Err(anyhow!("Task locked")),
				None => {
					task.lock = Some(lock.clone());
					transaction.tasks.get_mut().await?.insert(task_id.clone(), task).await?;
					Ok(())
				},
			}
		},
		TaskLock::Unlock(lock) => {
			let mut task = transaction.task(task_id).await?;
			match task.lock {
				Some(task_lock) if lock == &task_lock => {
					task.lock = None;
					transaction.tasks.get_mut().await?.insert(task_id.clone(), task).await?;
					Ok(())
				},
				Some(_task_lock) => Err(anyhow!("Task locked")),
				None => Ok(()),
			}
		},
	}
}

#[cfg(test)]
mod tests {
	use crate::{Board, BoardAction, List, Task};
	use co_api::{BlockStorageExt, CoreBlockStorage, Link, OptionLink, Reducer, ReducerAction, Tags, TagsAction};
	use co_storage::MemoryBlockStorage;
	use futures::TryStreamExt;

	fn tags(pairs: &[(&str, &str)]) -> Tags {
		let mut result = Tags::new();
		for (key, value) in pairs {
			result.insert(((*key).to_owned(), (*value).to_owned().into()));
		}
		result
	}

	async fn reduce_action(
		storage: &MemoryBlockStorage,
		core_storage: &CoreBlockStorage,
		state: OptionLink<Board>,
		payload: BoardAction,
	) -> (Board, Link<Board>) {
		let action = ReducerAction { from: "did:local:test".to_owned(), core: "board".to_owned(), time: 0, payload };
		let action_link = storage.set_value(&action).await.unwrap();
		let link = Board::reduce(state, action_link, core_storage).await.unwrap();
		let next = storage.get_value(&link).await.unwrap();
		(next, link)
	}

	#[tokio::test]
	async fn test_board_tags_action() {
		let storage = MemoryBlockStorage::default();
		let core_storage = CoreBlockStorage::new(storage.clone(), false);

		let (state, link) = reduce_action(
			&storage,
			&core_storage,
			OptionLink::none(),
			BoardAction::BoardTags(TagsAction::insert(tags(&[("a", "1"), ("b", "2")]))),
		)
		.await;
		assert_eq!(state.tags, tags(&[("a", "1"), ("b", "2")]));

		// set replaces the value for key "a"
		let (state, link) = reduce_action(
			&storage,
			&core_storage,
			link.into(),
			BoardAction::BoardTags(TagsAction::set(tags(&[("a", "9")]))),
		)
		.await;
		assert_eq!(state.tags, tags(&[("a", "9"), ("b", "2")]));

		let (state, _link) =
			reduce_action(&storage, &core_storage, link.into(), BoardAction::BoardTags(TagsAction::remove_key("a")))
				.await;
		assert_eq!(state.tags, tags(&[("b", "2")]));
	}

	#[tokio::test]
	async fn test_list_tags_action() {
		let storage = MemoryBlockStorage::default();
		let core_storage = CoreBlockStorage::new(storage.clone(), false);

		// seed a list named "backlog"
		let (_state, link) = reduce_action(
			&storage,
			&core_storage,
			OptionLink::none(),
			BoardAction::ListCreate { list: List::new("backlog"), after: None },
		)
		.await;

		// insert then remove tags on the list
		let (_state, link) = reduce_action(
			&storage,
			&core_storage,
			link.into(),
			BoardAction::ListTags("backlog".to_owned(), TagsAction::insert(tags(&[("a", "1"), ("b", "2")]))),
		)
		.await;
		let (state, _link) = reduce_action(
			&storage,
			&core_storage,
			link.into(),
			BoardAction::ListTags("backlog".to_owned(), TagsAction::remove_key("a")),
		)
		.await;

		// read the "backlog" list back
		let lists: Vec<(_, List)> = state.lists.stream(&core_storage).try_collect().await.unwrap();
		let (_index, list) = lists.into_iter().find(|(_index, list)| list.name == "backlog").unwrap();
		assert_eq!(list.tags, tags(&[("b", "2")]));
	}

	#[tokio::test]
	async fn test_task_tags_action() {
		let storage = MemoryBlockStorage::default();
		let core_storage = CoreBlockStorage::new(storage.clone(), false);

		// seed a list and a task with id "t1" in it
		let (_state, link) = reduce_action(
			&storage,
			&core_storage,
			OptionLink::none(),
			BoardAction::ListCreate { list: List::new("backlog"), after: None },
		)
		.await;
		let task =
			Task { id: "t1".to_owned(), name: "Task 1".to_owned(), tags: Tags::new(), payload: None, lock: None };
		let (_state, link) = reduce_action(
			&storage,
			&core_storage,
			link.into(),
			BoardAction::TaskCreate { list: "backlog".to_owned(), task, after: None },
		)
		.await;

		// insert then remove tags on the task
		let (_state, link) = reduce_action(
			&storage,
			&core_storage,
			link.into(),
			BoardAction::TaskTags("t1".to_owned(), TagsAction::insert(tags(&[("a", "1"), ("b", "2")]))),
		)
		.await;
		let (state, _link) = reduce_action(
			&storage,
			&core_storage,
			link.into(),
			BoardAction::TaskTags("t1".to_owned(), TagsAction::remove_key("a")),
		)
		.await;

		// read task "t1" back
		let task = state.tasks.get(&core_storage, &"t1".to_owned()).await.unwrap().unwrap();
		assert_eq!(task.tags, tags(&[("b", "2")]));
	}
}
