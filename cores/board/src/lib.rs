// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use anyhow::anyhow;
use cid::Cid;
use co_api::{
	co, BlockStorage, BlockStorageExt, CoList, CoListIndex, CoListTransaction, CoMap, CoTryStreamExt, CoreBlockStorage,
	IsDefault, LazyTransaction, Link, OptionLink, Reducer, ReducerAction, Tags, TagsAction, TagsExpr,
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
	TaskEnqueueOrReplace {
		list: ListName,
		matching: TagsExpr,
		task: Task,
		after: Option<TaskId>,
	},
	TaskCompleteIf {
		task: TaskId,
		from_list: ListName,
		expected_lock: String,
		transition: TaskTransition,
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

#[co]
pub enum TaskTransition {
	Delete,
	Move { list: ListName, after: Option<TaskId>, unless_matching: Option<TagsExpr> },
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
		BoardAction::TaskEnqueueOrReplace { list, matching, task, after } => {
			reduce_task_enqueue_or_replace(&mut transaction, list, matching, task, after)
				.boxed()
				.await?
		},
		BoardAction::TaskCompleteIf { task, from_list, expected_lock, transition } => {
			reduce_task_complete_if(&mut transaction, task, from_list, expected_lock, transition)
				.boxed()
				.await?
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

	/// Find matching unlocked task memberships in one list, preserving membership order.
	async fn matching_unlocked_task_memberships(
		&mut self,
		list: &List,
		matching: &TagsExpr,
		exclude: Option<&TaskId>,
	) -> Result<Vec<(CoListIndex, TaskId)>, anyhow::Error> {
		let storage = self.storage.clone();
		let list_tasks = list.tasks.clone();
		let memberships = list_tasks.stream(&storage);
		pin_mut!(memberships);
		let mut result = Vec::new();
		while let Some((index, task_id)) = memberships.try_next().await? {
			let task = self
				.tasks
				.get()
				.await?
				.get(&task_id)
				.await?
				.ok_or_else(|| anyhow!("Broken task membership: {} in list: {}", task_id, list.name))?;
			if exclude == Some(&task_id) {
				continue;
			}
			if task.tags.matches(matching) {
				if task.lock.is_some() {
					return Err(anyhow!("Task locked: {}", task_id));
				}
				result.push((index, task_id));
			}
		}
		Ok(result)
	}

	/// Check whether a different task in one list matches a selector.
	async fn has_matching_task_membership(
		&mut self,
		list: &List,
		matching: &TagsExpr,
		exclude: &TaskId,
	) -> Result<bool, anyhow::Error> {
		let storage = self.storage.clone();
		let memberships = list.tasks.stream(&storage);
		pin_mut!(memberships);
		let mut found_match = false;
		while let Some((_, task_id)) = memberships.try_next().await? {
			if &task_id == exclude {
				continue;
			}
			let task = self
				.tasks
				.get()
				.await?
				.get(&task_id)
				.await?
				.ok_or_else(|| anyhow!("Broken task membership: {} in list: {}", task_id, list.name))?;
			if task.tags.matches(matching) {
				found_match = true;
			}
		}
		Ok(found_match)
	}
}

async fn find_task_membership<S>(
	storage: &S,
	list_tasks: &CoList<TaskId>,
	task_id: &TaskId,
) -> Result<Option<CoListIndex>, anyhow::Error>
where
	S: BlockStorage + Clone + 'static,
{
	Ok(list_tasks
		.stream(storage)
		.try_filter(|item| ready(&item.1 == task_id))
		.try_first()
		.await?
		.map(|(index, _)| index))
}

async fn find_task_index<S>(
	list_tasks: &CoListTransaction<S, TaskId>,
	task_id: &TaskId,
) -> Result<Option<CoListIndex>, anyhow::Error>
where
	S: BlockStorage + Clone + 'static,
{
	Ok(list_tasks
		.stream()
		.try_filter(|item| ready(&item.1 == task_id))
		.try_first()
		.await?
		.map(|(index, _)| index))
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
	let task_after_index = if let Some(after) = &after { find_task_index(&list_tasks, after).await? } else { None };

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
	if let (Some(from_list), TaskLock::Lock(lock)) = (&from_list, &lock) {
		return reduce_task_move_claim(transaction, from_list, &list_name, &task_id, after.as_ref(), lock).await;
	}

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
			let source_task_index = find_task_index(&list_tasks, &task_id).await?.ok_or(anyhow!(
				"Task not found: {} in list: {}",
				task_id,
				source_list.name
			))?;
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
	let task_after_index = if let Some(after) = &after { find_task_index(&list_tasks, after).await? } else { None };

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

async fn reduce_task_move_claim<S: BlockStorage + Clone + 'static>(
	transaction: &mut BoardTransaction<S>,
	from_list: &ListName,
	target_name: &ListName,
	task_id: &TaskId,
	after: Option<&TaskId>,
	lock: &String,
) -> Result<(), anyhow::Error> {
	// a claim may have lost a race before it is reduced. Treat stale source,
	// membership, task, and competing-lock preconditions as successful no-ops.
	let (source_list_index, mut source_list) = match transaction.find_list_by_name(from_list).await? {
		Some(source) => source,
		None => return Ok(()),
	};
	let source_task_index = match find_task_membership(&transaction.storage, &source_list.tasks, task_id).await? {
		Some(index) => index,
		None => return Ok(()),
	};
	let mut task = match transaction.tasks.get().await?.get(task_id).await? {
		Some(task) => task,
		None => return Ok(()),
	};
	if task.lock.as_ref().is_some_and(|task_lock| task_lock != lock) {
		return Ok(());
	}

	// missing target configuration remains an error for an otherwise live claim.
	let same_list = source_list.name == *target_name;
	let (target_list_index, mut target_list) = if same_list {
		(source_list_index, source_list.clone())
	} else {
		transaction
			.find_list_by_name(target_name)
			.await?
			.ok_or_else(|| anyhow!("List not found: {}", target_name))?
	};
	let task_after_index = match after {
		Some(after_task) if after_task != task_id => {
			find_task_membership(&transaction.storage, &target_list.tasks, after_task).await?
		},
		_ => None,
	};

	// all claim and target validation is complete before mutable access.
	task.lock = Some(lock.clone());
	if same_list {
		let mut list_tasks = source_list.tasks.open(&transaction.storage).await?;
		list_tasks.remove(source_task_index).await?;
		if let Some(task_after_index) = task_after_index {
			list_tasks.insert(task_after_index, task_id.clone()).await?;
		} else {
			list_tasks.push(task_id.clone()).await?;
		}
		source_list.tasks = list_tasks.store().await?;
		transaction.lists.get_mut().await?.set(source_list_index, source_list).await?;
	} else {
		let mut source_tasks = source_list.tasks.open(&transaction.storage).await?;
		let mut target_tasks = target_list.tasks.open(&transaction.storage).await?;
		source_tasks.remove(source_task_index).await?;
		if let Some(task_after_index) = task_after_index {
			target_tasks.insert(task_after_index, task_id.clone()).await?;
		} else {
			target_tasks.push(task_id.clone()).await?;
		}
		source_list.tasks = source_tasks.store().await?;
		target_list.tasks = target_tasks.store().await?;
		let lists = transaction.lists.get_mut().await?;
		lists.set(source_list_index, source_list).await?;
		lists.set(target_list_index, target_list).await?;
	}
	transaction.tasks.get_mut().await?.insert(task_id.clone(), task).await?;

	Ok(())
}

async fn reduce_task_complete_if<S: BlockStorage + Clone + 'static>(
	transaction: &mut BoardTransaction<S>,
	task_id: TaskId,
	from_list: ListName,
	expected_lock: String,
	transition: TaskTransition,
) -> Result<(), anyhow::Error> {
	// stale completion attempts are successful no-ops. check every claim
	// precondition before opening any collection for mutation.
	let (source_list_index, mut source_list) = match transaction.find_list_by_name(&from_list).await? {
		Some(source) => source,
		None => return Ok(()),
	};
	let source_task_index = match find_task_membership(&transaction.storage, &source_list.tasks, &task_id).await? {
		Some(index) => index,
		None => return Ok(()),
	};
	let mut task = match transaction.tasks.get().await?.get(&task_id).await? {
		Some(task) => task,
		None => return Ok(()),
	};
	if task.lock.as_deref() != Some(expected_lock.as_str()) {
		return Ok(());
	}

	match transition {
		TaskTransition::Delete => {
			let mut source_tasks = source_list.tasks.open(&transaction.storage).await?;
			source_tasks.remove(source_task_index).await?;
			source_list.tasks = source_tasks.store().await?;
			transaction.lists.get_mut().await?.set(source_list_index, source_list).await?;
			transaction
				.tasks
				.get_mut()
				.await?
				.remove(task_id.clone())
				.await?
				.ok_or_else(|| anyhow!("Task not found: {}", task_id))?;
		},
		TaskTransition::Move { list: target_name, after, unless_matching } => {
			// a missing target is an error only for a completion whose claim is still live.
			let same_list = source_list.name == target_name;
			let (target_list_index, mut target_list) = if same_list {
				(source_list_index, source_list.clone())
			} else {
				transaction
					.find_list_by_name(&target_name)
					.await?
					.ok_or_else(|| anyhow!("List not found: {}", target_name))?
			};

			let has_successor = if let Some(matching) = &unless_matching {
				transaction
					.has_matching_task_membership(&target_list, matching, &task_id)
					.await?
			} else {
				false
			};
			if has_successor {
				let mut source_tasks = source_list.tasks.open(&transaction.storage).await?;
				source_tasks.remove(source_task_index).await?;
				source_list.tasks = source_tasks.store().await?;
				transaction.lists.get_mut().await?.set(source_list_index, source_list).await?;
				transaction
					.tasks
					.get_mut()
					.await?
					.remove(task_id.clone())
					.await?
					.ok_or_else(|| anyhow!("Task not found: {}", task_id))?;
				return Ok(());
			}

			let task_after_index = match &after {
				Some(after_task) if after_task != &task_id => {
					find_task_membership(&transaction.storage, &target_list.tasks, after_task).await?
				},
				_ => None,
			};

			// all validation is complete. Apply the move and clear the exact claim.
			task.lock = None;
			if same_list {
				let mut list_tasks = source_list.tasks.open(&transaction.storage).await?;
				list_tasks.remove(source_task_index).await?;
				if let Some(task_after_index) = task_after_index {
					list_tasks.insert(task_after_index, task_id.clone()).await?;
				} else {
					list_tasks.push(task_id.clone()).await?;
				}
				source_list.tasks = list_tasks.store().await?;
				transaction.lists.get_mut().await?.set(source_list_index, source_list).await?;
			} else {
				let mut source_tasks = source_list.tasks.open(&transaction.storage).await?;
				let mut target_tasks = target_list.tasks.open(&transaction.storage).await?;
				source_tasks.remove(source_task_index).await?;
				if let Some(task_after_index) = task_after_index {
					target_tasks.insert(task_after_index, task_id.clone()).await?;
				} else {
					target_tasks.push(task_id.clone()).await?;
				}
				source_list.tasks = source_tasks.store().await?;
				target_list.tasks = target_tasks.store().await?;
				let lists = transaction.lists.get_mut().await?;
				lists.set(source_list_index, source_list).await?;
				lists.set(target_list_index, target_list).await?;
			}
			transaction.tasks.get_mut().await?.insert(task_id, task).await?;
		},
	}

	Ok(())
}

async fn reduce_task_enqueue_or_replace<S: BlockStorage + Clone + 'static>(
	transaction: &mut BoardTransaction<S>,
	list_name: ListName,
	matching: TagsExpr,
	task: Task,
	after: Option<TaskId>,
) -> Result<(), anyhow::Error> {
	let task_id = task.id.clone();

	// validate the complete operation before accessing a collection mutably.
	let (list_index, mut list) = transaction
		.find_list_by_name(&list_name)
		.await?
		.ok_or(anyhow!("List not found: {}", list_name))?;
	if task.lock.is_some() {
		return Err(anyhow!("New task is locked: {}", task_id));
	}
	if !task.tags.matches(&matching) {
		return Err(anyhow!("Task tags do not match selector: {}", task_id));
	}
	if transaction.tasks.get().await?.contains_key(&task_id).await? {
		return Err(anyhow!("Task exists: {}", task_id));
	}
	let matching_tasks = transaction
		.matching_unlocked_task_memberships(&list, &matching, Some(&task_id))
		.await?;

	let mut list_tasks = list.tasks.open(&transaction.storage).await?;
	if let Some((first_index, _)) = matching_tasks.first() {
		// preserve the first match's exact position and collapse any later matches.
		list_tasks.set(*first_index, task_id.clone()).await?;
		for (index, _) in matching_tasks.iter().skip(1) {
			list_tasks.remove(*index).await?;
		}
	} else {
		let task_after_index = if let Some(after) = &after { find_task_index(&list_tasks, after).await? } else { None };
		if let Some(task_after_index) = task_after_index {
			list_tasks.insert(task_after_index, task_id.clone()).await?;
		} else {
			list_tasks.push(task_id.clone()).await?;
		}
	}

	let tasks = transaction.tasks.get_mut().await?;
	for (_, old_task_id) in matching_tasks {
		tasks.remove(old_task_id).await?;
	}
	tasks.insert(task_id, task).await?;

	list.tasks = list_tasks.store().await?;
	transaction.lists.get_mut().await?.set(list_index, list).await?;

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
	let task_after_index = if let Some(after) = &after { find_task_index(&list_tasks, after).await? } else { None };
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
		return Ok(());
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
	use crate::{Board, BoardAction, List, Task, TaskLock, TaskTransition};
	use co_api::{
		BlockStorageExt, CoTryStreamExt, CoreBlockStorage, Link, OptionLink, Reducer, ReducerAction, Tags, TagsAction,
		TagsExpr,
	};
	use co_storage::MemoryBlockStorage;
	use futures::TryStreamExt;

	fn tags(pairs: &[(&str, &str)]) -> Tags {
		let mut result = Tags::new();
		for (key, value) in pairs {
			result.insert(((*key).to_owned(), (*value).to_owned().into()));
		}
		result
	}

	fn task(id: &str, task_tags: Tags, lock: Option<&str>) -> Task {
		Task { id: id.to_owned(), name: id.to_owned(), tags: task_tags, payload: None, lock: lock.map(str::to_owned) }
	}

	async fn try_reduce_action(
		storage: &MemoryBlockStorage,
		core_storage: &CoreBlockStorage,
		state: OptionLink<Board>,
		payload: BoardAction,
	) -> Result<(Board, Link<Board>), anyhow::Error> {
		let action = ReducerAction { from: "did:local:test".to_owned(), core: "board".to_owned(), time: 0, payload };
		let action_link = storage.set_value(&action).await?;
		let link = Board::reduce(state, action_link, core_storage).await?;
		let next = storage.get_value(&link).await?;
		Ok((next, link))
	}

	async fn reduce_action(
		storage: &MemoryBlockStorage,
		core_storage: &CoreBlockStorage,
		state: OptionLink<Board>,
		payload: BoardAction,
	) -> (Board, Link<Board>) {
		try_reduce_action(storage, core_storage, state, payload).await.unwrap()
	}

	async fn seed_list(
		storage: &MemoryBlockStorage,
		core_storage: &CoreBlockStorage,
		state: OptionLink<Board>,
		name: &str,
	) -> (Board, Link<Board>) {
		reduce_action(storage, core_storage, state, BoardAction::ListCreate { list: List::new(name), after: None })
			.await
	}

	async fn seed_task(
		storage: &MemoryBlockStorage,
		core_storage: &CoreBlockStorage,
		state: OptionLink<Board>,
		list: &str,
		task: Task,
	) -> (Board, Link<Board>) {
		reduce_action(
			storage,
			core_storage,
			state,
			BoardAction::TaskCreate { list: list.to_owned(), task, after: None },
		)
		.await
	}

	/// Build the narrow broken-state fixtures that public board actions cannot produce.
	async fn without_task_record(mut state: Board, core_storage: &CoreBlockStorage, task_id: &str) -> Board {
		assert!(state.tasks.remove(core_storage, task_id.to_owned()).await.unwrap().is_some());
		state
	}

	async fn with_appended_broken_membership(
		mut state: Board,
		core_storage: &CoreBlockStorage,
		list_name: &str,
		task_id: &str,
	) -> Board {
		let (list_index, mut list) = state
			.lists
			.stream(core_storage)
			.try_filter(|(_, list)| std::future::ready(list.name == list_name))
			.try_first()
			.await
			.unwrap()
			.unwrap();
		list.tasks.push(core_storage, task_id.to_owned()).await.unwrap();
		state.lists.set(core_storage, list_index, list).await.unwrap();
		state
	}

	async fn list_task_ids(state: &Board, core_storage: &CoreBlockStorage, name: &str) -> Vec<String> {
		let lists: Vec<(_, List)> = state.lists.stream(core_storage).try_collect().await.unwrap();
		let (_, list) = lists.into_iter().find(|(_, list)| list.name == name).unwrap();
		list.tasks
			.stream(core_storage)
			.map_ok(|(_, task_id)| task_id)
			.try_collect()
			.await
			.unwrap()
	}

	async fn list_by_name(state: &Board, core_storage: &CoreBlockStorage, name: &str) -> List {
		state
			.lists
			.stream(core_storage)
			.map_ok(|(_, list)| list)
			.try_filter(|list| std::future::ready(list.name == name))
			.try_first()
			.await
			.unwrap()
			.unwrap()
	}

	async fn list_names(state: &Board, core_storage: &CoreBlockStorage) -> Vec<String> {
		state
			.lists
			.stream(core_storage)
			.map_ok(|(_, list)| list.name)
			.try_collect()
			.await
			.unwrap()
	}

	fn head_key(co: &str, recipient: &str) -> TagsExpr {
		TagsExpr::new("task-type", "co-heads-did")
			.and(TagsExpr::new("co", co))
			.and(TagsExpr::new("recipient", recipient))
	}

	#[tokio::test]
	async fn task_enqueue_or_replace_creates_after_unmatched_task() {
		let storage = MemoryBlockStorage::default();
		let core_storage = CoreBlockStorage::new(storage.clone(), false);
		let matching = head_key("co:one", "did:key:recipient");
		let matching_tags =
			tags(&[("task-type", "co-heads-did"), ("co", "co:one"), ("recipient", "did:key:recipient")]);

		let (_state, link) = reduce_action(
			&storage,
			&core_storage,
			OptionLink::none(),
			BoardAction::ListCreate { list: List::new("backlog"), after: None },
		)
		.await;
		let (_state, link) = reduce_action(
			&storage,
			&core_storage,
			link.into(),
			BoardAction::TaskCreate { list: "backlog".to_owned(), task: task("a", Tags::new(), None), after: None },
		)
		.await;
		let (state, _link) = reduce_action(
			&storage,
			&core_storage,
			link.into(),
			BoardAction::TaskEnqueueOrReplace {
				list: "backlog".to_owned(),
				matching,
				task: task("b", matching_tags, None),
				after: Some("a".to_owned()),
			},
		)
		.await;

		assert_eq!(list_task_ids(&state, &core_storage, "backlog").await, ["a", "b"]);
	}

	#[tokio::test]
	async fn task_enqueue_or_replace_replaces_first_match_and_collapses_duplicates() {
		let storage = MemoryBlockStorage::default();
		let core_storage = CoreBlockStorage::new(storage.clone(), false);
		let matching = head_key("co:one", "did:key:recipient");
		let matching_tags =
			tags(&[("task-type", "co-heads-did"), ("co", "co:one"), ("recipient", "did:key:recipient")]);

		let (_state, mut link) = reduce_action(
			&storage,
			&core_storage,
			OptionLink::none(),
			BoardAction::ListCreate { list: List::new("backlog"), after: None },
		)
		.await;
		for seeded_task in [
			task("keep-a", Tags::new(), None),
			task("old-1", matching_tags.clone(), None),
			task("keep-b", tags(&[("kind", "keep")]), None),
			task("old-2", matching_tags.clone(), None),
		] {
			let (_state, next_link) = reduce_action(
				&storage,
				&core_storage,
				link.into(),
				BoardAction::TaskCreate { list: "backlog".to_owned(), task: seeded_task, after: None },
			)
			.await;
			link = next_link;
		}

		let (state, _link) = reduce_action(
			&storage,
			&core_storage,
			link.into(),
			BoardAction::TaskEnqueueOrReplace {
				list: "backlog".to_owned(),
				matching,
				task: task("new", matching_tags, None),
				after: Some("keep-b".to_owned()),
			},
		)
		.await;

		assert_eq!(list_task_ids(&state, &core_storage, "backlog").await, ["keep-a", "new", "keep-b"]);
		assert!(state.tasks.get(&core_storage, &"old-1".to_owned()).await.unwrap().is_none());
		assert!(state.tasks.get(&core_storage, &"old-2".to_owned()).await.unwrap().is_none());
	}

	#[tokio::test]
	async fn task_enqueue_or_replace_never_scans_another_list() {
		let storage = MemoryBlockStorage::default();
		let core_storage = CoreBlockStorage::new(storage.clone(), false);
		let matching = head_key("co:one", "did:key:recipient");
		let matching_tags =
			tags(&[("task-type", "co-heads-did"), ("co", "co:one"), ("recipient", "did:key:recipient")]);

		let (_state, link) = seed_list(&storage, &core_storage, OptionLink::none(), "backlog").await;
		let (_state, link) = seed_list(&storage, &core_storage, link.into(), "doing").await;
		let (_state, link) = reduce_action(
			&storage,
			&core_storage,
			link.into(),
			BoardAction::TaskCreate {
				list: "doing".to_owned(),
				task: task("doing-match", matching_tags.clone(), None),
				after: None,
			},
		)
		.await;
		let (state, _link) = reduce_action(
			&storage,
			&core_storage,
			link.into(),
			BoardAction::TaskEnqueueOrReplace {
				list: "backlog".to_owned(),
				matching,
				task: task("backlog-new", matching_tags, None),
				after: None,
			},
		)
		.await;

		assert_eq!(list_task_ids(&state, &core_storage, "doing").await, ["doing-match"]);
		assert_eq!(list_task_ids(&state, &core_storage, "backlog").await, ["backlog-new"]);
		assert!(state
			.tasks
			.get(&core_storage, &"doing-match".to_owned())
			.await
			.unwrap()
			.is_some());
		assert!(state
			.tasks
			.get(&core_storage, &"backlog-new".to_owned())
			.await
			.unwrap()
			.is_some());
	}

	#[tokio::test]
	async fn task_enqueue_or_replace_rejects_existing_id() {
		let storage = MemoryBlockStorage::default();
		let core_storage = CoreBlockStorage::new(storage.clone(), false);
		let matching = head_key("co:one", "did:key:recipient");
		let matching_tags =
			tags(&[("task-type", "co-heads-did"), ("co", "co:one"), ("recipient", "did:key:recipient")]);

		let (_state, link) = reduce_action(
			&storage,
			&core_storage,
			OptionLink::none(),
			BoardAction::ListCreate { list: List::new("backlog"), after: None },
		)
		.await;
		let (_state, link) = reduce_action(
			&storage,
			&core_storage,
			link.into(),
			BoardAction::ListCreate { list: List::new("doing"), after: None },
		)
		.await;
		let (_state, link) = reduce_action(
			&storage,
			&core_storage,
			link.into(),
			BoardAction::TaskCreate {
				list: "doing".to_owned(),
				task: task("existing", Tags::new(), None),
				after: None,
			},
		)
		.await;
		let prior_link = link;
		let prior_state: Board = storage.get_value(&prior_link).await.unwrap();

		let result = try_reduce_action(
			&storage,
			&core_storage,
			prior_link.into(),
			BoardAction::TaskEnqueueOrReplace {
				list: "backlog".to_owned(),
				matching,
				task: task("existing", matching_tags, None),
				after: None,
			},
		)
		.await;

		assert!(result.is_err());
		let unchanged_state: Board = storage.get_value(&prior_link).await.unwrap();
		assert_eq!(unchanged_state, prior_state);
		assert_eq!(list_task_ids(&unchanged_state, &core_storage, "backlog").await, Vec::<String>::new());
		assert_eq!(list_task_ids(&unchanged_state, &core_storage, "doing").await, ["existing"]);
	}

	#[tokio::test]
	async fn task_enqueue_or_replace_rejects_invalid_lock_or_tags() {
		let storage = MemoryBlockStorage::default();
		let core_storage = CoreBlockStorage::new(storage.clone(), false);
		let matching = head_key("co:one", "did:key:recipient");
		let matching_tags =
			tags(&[("task-type", "co-heads-did"), ("co", "co:one"), ("recipient", "did:key:recipient")]);

		let (_state, link) = reduce_action(
			&storage,
			&core_storage,
			OptionLink::none(),
			BoardAction::ListCreate { list: List::new("backlog"), after: None },
		)
		.await;

		// Supplied task is locked.
		let result = try_reduce_action(
			&storage,
			&core_storage,
			link.into(),
			BoardAction::TaskEnqueueOrReplace {
				list: "backlog".to_owned(),
				matching: matching.clone(),
				task: task("locked-new", matching_tags.clone(), Some("lock")),
				after: None,
			},
		)
		.await;
		assert!(result.is_err());

		// Supplied tags do not match the selector.
		let result = try_reduce_action(
			&storage,
			&core_storage,
			link.into(),
			BoardAction::TaskEnqueueOrReplace {
				list: "backlog".to_owned(),
				matching: matching.clone(),
				task: task("unmatched-new", Tags::new(), None),
				after: None,
			},
		)
		.await;
		assert!(result.is_err());

		// An existing matching task is locked.
		let (_state, link) = reduce_action(
			&storage,
			&core_storage,
			link.into(),
			BoardAction::TaskCreate {
				list: "backlog".to_owned(),
				task: task("locked-old", matching_tags.clone(), Some("lock")),
				after: None,
			},
		)
		.await;
		let result = try_reduce_action(
			&storage,
			&core_storage,
			link.into(),
			BoardAction::TaskEnqueueOrReplace {
				list: "backlog".to_owned(),
				matching,
				task: task("new", matching_tags, None),
				after: None,
			},
		)
		.await;
		assert!(result.is_err());
	}

	#[tokio::test]
	async fn task_complete_if_missing_source_or_wrong_lock_is_noop() {
		let storage = MemoryBlockStorage::default();
		let core_storage = CoreBlockStorage::new(storage.clone(), false);

		let (_state, link) = seed_list(&storage, &core_storage, OptionLink::none(), "backlog").await;
		let (_state, link) = seed_list(&storage, &core_storage, link.into(), "doing").await;
		let (_state, link) =
			seed_task(&storage, &core_storage, link.into(), "doing", task("locked", Tags::new(), Some("token-a")))
				.await;
		let (_state, mut link) =
			seed_task(&storage, &core_storage, link.into(), "doing", task("unlocked", Tags::new(), None)).await;

		let stale_actions = [
			BoardAction::TaskCompleteIf {
				task: "missing".to_owned(),
				from_list: "doing".to_owned(),
				expected_lock: "token-a".to_owned(),
				transition: TaskTransition::Delete,
			},
			BoardAction::TaskCompleteIf {
				task: "locked".to_owned(),
				from_list: "missing-list".to_owned(),
				expected_lock: "token-a".to_owned(),
				transition: TaskTransition::Delete,
			},
			BoardAction::TaskCompleteIf {
				task: "locked".to_owned(),
				from_list: "backlog".to_owned(),
				expected_lock: "token-a".to_owned(),
				transition: TaskTransition::Delete,
			},
			BoardAction::TaskCompleteIf {
				task: "unlocked".to_owned(),
				from_list: "doing".to_owned(),
				expected_lock: "token-a".to_owned(),
				transition: TaskTransition::Delete,
			},
			BoardAction::TaskCompleteIf {
				task: "locked".to_owned(),
				from_list: "doing".to_owned(),
				expected_lock: "token-b".to_owned(),
				transition: TaskTransition::Delete,
			},
		];

		for action in stale_actions {
			let previous: Board = storage.get_value(&link).await.unwrap();
			let (state, next_link) = try_reduce_action(&storage, &core_storage, link.into(), action)
				.await
				.expect("stale completion must succeed");
			assert_eq!(state, previous);
			assert_eq!(list_task_ids(&state, &core_storage, "backlog").await, Vec::<String>::new());
			assert_eq!(list_task_ids(&state, &core_storage, "doing").await, ["locked", "unlocked"]);
			assert_eq!(
				state
					.tasks
					.get(&core_storage, &"locked".to_owned())
					.await
					.unwrap()
					.unwrap()
					.lock,
				Some("token-a".to_owned())
			);
			assert!(state.tasks.get(&core_storage, &"unlocked".to_owned()).await.unwrap().is_some());
			link = next_link;
		}

		// Membership remains valid while the corresponding task-map record is absent.
		let state: Board = storage.get_value(&link).await.unwrap();
		let missing_record = without_task_record(state, &core_storage, "locked").await;
		let missing_record_link = storage.set_value(&missing_record).await.unwrap();
		let (state, _link) = try_reduce_action(
			&storage,
			&core_storage,
			missing_record_link.into(),
			BoardAction::TaskCompleteIf {
				task: "locked".to_owned(),
				from_list: "doing".to_owned(),
				expected_lock: "token-a".to_owned(),
				transition: TaskTransition::Delete,
			},
		)
		.await
		.expect("completion with a missing task-map record must succeed");
		assert_eq!(state, missing_record);
		assert_eq!(list_task_ids(&state, &core_storage, "doing").await, ["locked", "unlocked"]);
	}

	#[tokio::test]
	async fn task_complete_if_deletes_only_the_exact_claim() {
		let storage = MemoryBlockStorage::default();
		let core_storage = CoreBlockStorage::new(storage.clone(), false);

		let (_state, link) = seed_list(&storage, &core_storage, OptionLink::none(), "doing").await;
		let sibling = task("sibling", tags(&[("preserve", "yes")]), None);
		let (_state, link) = seed_task(&storage, &core_storage, link.into(), "doing", sibling.clone()).await;
		let (_state, link) =
			seed_task(&storage, &core_storage, link.into(), "doing", task("claimed", Tags::new(), Some("token-a")))
				.await;
		let (state, _link) = reduce_action(
			&storage,
			&core_storage,
			link.into(),
			BoardAction::TaskCompleteIf {
				task: "claimed".to_owned(),
				from_list: "doing".to_owned(),
				expected_lock: "token-a".to_owned(),
				transition: TaskTransition::Delete,
			},
		)
		.await;

		assert_eq!(list_task_ids(&state, &core_storage, "doing").await, ["sibling"]);
		assert!(state.tasks.get(&core_storage, &"claimed".to_owned()).await.unwrap().is_none());
		assert_eq!(state.tasks.get(&core_storage, &"sibling".to_owned()).await.unwrap(), Some(sibling));
	}

	#[tokio::test]
	async fn task_complete_if_moves_and_unlocks_the_exact_claim() {
		let storage = MemoryBlockStorage::default();
		let core_storage = CoreBlockStorage::new(storage.clone(), false);

		let (_state, link) = seed_list(&storage, &core_storage, OptionLink::none(), "doing").await;
		let (_state, link) = seed_list(&storage, &core_storage, link.into(), "failed").await;
		let source_sibling = task("source-sibling", tags(&[("preserve", "source")]), None);
		let target_anchor = task("failed-first", tags(&[("preserve", "target")]), None);
		let (_state, link) = seed_task(&storage, &core_storage, link.into(), "doing", source_sibling.clone()).await;
		let (_state, link) = seed_task(&storage, &core_storage, link.into(), "failed", target_anchor.clone()).await;
		let (_state, link) =
			seed_task(&storage, &core_storage, link.into(), "doing", task("claimed", Tags::new(), Some("token-a")))
				.await;
		let (state, _link) = reduce_action(
			&storage,
			&core_storage,
			link.into(),
			BoardAction::TaskCompleteIf {
				task: "claimed".to_owned(),
				from_list: "doing".to_owned(),
				expected_lock: "token-a".to_owned(),
				transition: TaskTransition::Move {
					list: "failed".to_owned(),
					after: Some("failed-first".to_owned()),
					unless_matching: None,
				},
			},
		)
		.await;

		assert_eq!(list_task_ids(&state, &core_storage, "doing").await, ["source-sibling"]);
		assert_eq!(list_task_ids(&state, &core_storage, "failed").await, ["failed-first", "claimed"]);
		assert_eq!(state.tasks.get(&core_storage, &"source-sibling".to_owned()).await.unwrap(), Some(source_sibling));
		assert_eq!(state.tasks.get(&core_storage, &"failed-first".to_owned()).await.unwrap(), Some(target_anchor));
		assert_eq!(
			state
				.tasks
				.get(&core_storage, &"claimed".to_owned())
				.await
				.unwrap()
				.unwrap()
				.lock,
			None
		);

		// Same-list completion must exclude the completing task from both matching
		// and the `after` anchor while repositioning through one list transaction.
		let same_matching = TagsExpr::new("kind", "same-list");
		let (_state, same_link) = seed_list(&storage, &core_storage, OptionLink::none(), "same").await;
		let (_state, same_link) = seed_task(
			&storage,
			&core_storage,
			same_link.into(),
			"same",
			task("same-claimed", tags(&[("kind", "same-list")]), Some("same-token")),
		)
		.await;
		let (_state, same_link) =
			seed_task(&storage, &core_storage, same_link.into(), "same", task("same-anchor", Tags::new(), None)).await;
		let (same_state, _link) = reduce_action(
			&storage,
			&core_storage,
			same_link.into(),
			BoardAction::TaskCompleteIf {
				task: "same-claimed".to_owned(),
				from_list: "same".to_owned(),
				expected_lock: "same-token".to_owned(),
				transition: TaskTransition::Move {
					list: "same".to_owned(),
					after: Some("same-claimed".to_owned()),
					unless_matching: Some(same_matching),
				},
			},
		)
		.await;
		assert_eq!(list_task_ids(&same_state, &core_storage, "same").await, ["same-anchor", "same-claimed"]);
		assert_eq!(
			same_state
				.tasks
				.get(&core_storage, &"same-claimed".to_owned())
				.await
				.unwrap()
				.unwrap()
				.lock,
			None
		);
	}

	#[tokio::test]
	async fn task_complete_if_retry_deletes_old_generation_when_successor_matches() {
		let storage = MemoryBlockStorage::default();
		let core_storage = CoreBlockStorage::new(storage.clone(), false);
		let matching = head_key("co:one", "did:key:recipient");
		let matching_tags =
			tags(&[("task-type", "co-heads-did"), ("co", "co:one"), ("recipient", "did:key:recipient")]);

		let (_state, link) = seed_list(&storage, &core_storage, OptionLink::none(), "backlog").await;
		let (_state, link) = seed_list(&storage, &core_storage, link.into(), "doing").await;
		let (_state, link) = seed_task(
			&storage,
			&core_storage,
			link.into(),
			"doing",
			task("old", matching_tags.clone(), Some("token-a")),
		)
		.await;
		let (seeded, link) = seed_task(
			&storage,
			&core_storage,
			link.into(),
			"backlog",
			task("new", matching_tags, Some("successor-lock")),
		)
		.await;
		let completion = BoardAction::TaskCompleteIf {
			task: "old".to_owned(),
			from_list: "doing".to_owned(),
			expected_lock: "token-a".to_owned(),
			transition: TaskTransition::Move {
				list: "backlog".to_owned(),
				after: None,
				unless_matching: Some(matching),
			},
		};

		// Validate the complete target even when a valid matching successor appears
		// before a later membership whose task-map record is missing.
		let broken = with_appended_broken_membership(seeded, &core_storage, "backlog", "broken").await;
		let broken_link = storage.set_value(&broken).await.unwrap();
		let result = try_reduce_action(&storage, &core_storage, broken_link.into(), completion.clone()).await;
		assert!(result.is_err());
		let unchanged: Board = storage.get_value(&broken_link).await.unwrap();
		assert_eq!(unchanged, broken);

		let (state, _link) = reduce_action(&storage, &core_storage, link.into(), completion).await;

		assert_eq!(list_task_ids(&state, &core_storage, "doing").await, Vec::<String>::new());
		assert_eq!(list_task_ids(&state, &core_storage, "backlog").await, ["new"]);
		assert!(state.tasks.get(&core_storage, &"old".to_owned()).await.unwrap().is_none());
		assert_eq!(
			state.tasks.get(&core_storage, &"new".to_owned()).await.unwrap().unwrap().lock,
			Some("successor-lock".to_owned())
		);
	}

	#[tokio::test]
	async fn task_complete_if_retry_moves_old_generation_when_no_successor_matches() {
		let storage = MemoryBlockStorage::default();
		let core_storage = CoreBlockStorage::new(storage.clone(), false);
		let matching = head_key("co:one", "did:key:recipient");
		let matching_tags =
			tags(&[("task-type", "co-heads-did"), ("co", "co:one"), ("recipient", "did:key:recipient")]);

		let (_state, link) = seed_list(&storage, &core_storage, OptionLink::none(), "backlog").await;
		let (_state, link) = seed_list(&storage, &core_storage, link.into(), "doing").await;
		let (_state, link) = seed_task(
			&storage,
			&core_storage,
			link.into(),
			"backlog",
			task("unrelated", tags(&[("kind", "other")]), None),
		)
		.await;
		let (_state, link) =
			seed_task(&storage, &core_storage, link.into(), "doing", task("old", matching_tags, Some("token-a"))).await;
		let (state, _link) = reduce_action(
			&storage,
			&core_storage,
			link.into(),
			BoardAction::TaskCompleteIf {
				task: "old".to_owned(),
				from_list: "doing".to_owned(),
				expected_lock: "token-a".to_owned(),
				transition: TaskTransition::Move {
					list: "backlog".to_owned(),
					after: None,
					unless_matching: Some(matching),
				},
			},
		)
		.await;

		assert_eq!(list_task_ids(&state, &core_storage, "doing").await, Vec::<String>::new());
		assert_eq!(list_task_ids(&state, &core_storage, "backlog").await, ["unrelated", "old"]);
		assert_eq!(state.tasks.get(&core_storage, &"old".to_owned()).await.unwrap().unwrap().lock, None);
	}

	#[tokio::test]
	async fn task_completion_and_enqueue_are_correct_in_both_orders() {
		let storage = MemoryBlockStorage::default();
		let core_storage = CoreBlockStorage::new(storage.clone(), false);
		let matching = head_key("co:one", "did:key:recipient");
		let matching_tags =
			tags(&[("task-type", "co-heads-did"), ("co", "co:one"), ("recipient", "did:key:recipient")]);

		let (_state, link) = seed_list(&storage, &core_storage, OptionLink::none(), "backlog").await;
		let (_state, link) = seed_list(&storage, &core_storage, link.into(), "doing").await;
		let (_seeded, seed_link) = seed_task(
			&storage,
			&core_storage,
			link.into(),
			"doing",
			task("old", matching_tags.clone(), Some("token-a")),
		)
		.await;

		let (_state, completion_first_link) = reduce_action(
			&storage,
			&core_storage,
			seed_link.into(),
			BoardAction::TaskCompleteIf {
				task: "old".to_owned(),
				from_list: "doing".to_owned(),
				expected_lock: "token-a".to_owned(),
				transition: TaskTransition::Move {
					list: "backlog".to_owned(),
					after: None,
					unless_matching: Some(matching.clone()),
				},
			},
		)
		.await;
		let (completion_then_enqueue, _link) = reduce_action(
			&storage,
			&core_storage,
			completion_first_link.into(),
			BoardAction::TaskEnqueueOrReplace {
				list: "backlog".to_owned(),
				matching: matching.clone(),
				task: task("fresh", matching_tags.clone(), None),
				after: None,
			},
		)
		.await;

		let (_state, enqueue_first_link) = reduce_action(
			&storage,
			&core_storage,
			seed_link.into(),
			BoardAction::TaskEnqueueOrReplace {
				list: "backlog".to_owned(),
				matching: matching.clone(),
				task: task("fresh", matching_tags, None),
				after: None,
			},
		)
		.await;
		let (enqueue_then_completion, _link) = reduce_action(
			&storage,
			&core_storage,
			enqueue_first_link.into(),
			BoardAction::TaskCompleteIf {
				task: "old".to_owned(),
				from_list: "doing".to_owned(),
				expected_lock: "token-a".to_owned(),
				transition: TaskTransition::Move {
					list: "backlog".to_owned(),
					after: None,
					unless_matching: Some(matching),
				},
			},
		)
		.await;

		for state in [&completion_then_enqueue, &enqueue_then_completion] {
			assert_eq!(list_task_ids(state, &core_storage, "doing").await, Vec::<String>::new());
			assert_eq!(list_task_ids(state, &core_storage, "backlog").await, ["fresh"]);
			assert!(state.tasks.get(&core_storage, &"old".to_owned()).await.unwrap().is_none());
			assert!(state.tasks.get(&core_storage, &"fresh".to_owned()).await.unwrap().is_some());
		}
	}

	#[tokio::test]
	async fn task_move_claim_missing_source_or_competing_lock_is_noop() {
		let storage = MemoryBlockStorage::default();
		let core_storage = CoreBlockStorage::new(storage.clone(), false);

		let (_state, link) = seed_list(&storage, &core_storage, OptionLink::none(), "backlog").await;
		let (_state, link) = seed_list(&storage, &core_storage, link.into(), "doing").await;
		let (_state, link) = seed_list(&storage, &core_storage, link.into(), "other").await;
		let (_state, link) =
			seed_task(&storage, &core_storage, link.into(), "backlog", task("claimable", Tags::new(), None)).await;
		let (seeded, seed_link) = seed_task(
			&storage,
			&core_storage,
			link.into(),
			"backlog",
			task("claimed-by-a", Tags::new(), Some("token-a")),
		)
		.await;

		let result = try_reduce_action(
			&storage,
			&core_storage,
			seed_link.into(),
			BoardAction::TaskMove {
				from_list: Some("backlog".to_owned()),
				list: "missing-target".to_owned(),
				task: "claimable".to_owned(),
				after: None,
				lock: TaskLock::Lock("token-a".to_owned()),
			},
		)
		.await;
		assert!(result.is_err());
		let unchanged: Board = storage.get_value(&seed_link).await.unwrap();
		assert_eq!(unchanged, seeded);
		assert_eq!(
			unchanged
				.tasks
				.get(&core_storage, &"claimable".to_owned())
				.await
				.unwrap()
				.unwrap()
				.lock,
			None
		);

		for action in [
			BoardAction::TaskMove {
				from_list: Some("backlog".to_owned()),
				list: "doing".to_owned(),
				task: "missing".to_owned(),
				after: None,
				lock: TaskLock::Lock("token-a".to_owned()),
			},
			BoardAction::TaskMove {
				from_list: Some("missing-list".to_owned()),
				list: "doing".to_owned(),
				task: "claimable".to_owned(),
				after: None,
				lock: TaskLock::Lock("token-a".to_owned()),
			},
			BoardAction::TaskMove {
				from_list: Some("other".to_owned()),
				list: "doing".to_owned(),
				task: "claimable".to_owned(),
				after: None,
				lock: TaskLock::Lock("token-a".to_owned()),
			},
			BoardAction::TaskMove {
				from_list: Some("backlog".to_owned()),
				list: "doing".to_owned(),
				task: "claimed-by-a".to_owned(),
				after: None,
				lock: TaskLock::Lock("token-b".to_owned()),
			},
		] {
			let (state, _link) = try_reduce_action(&storage, &core_storage, seed_link.into(), action)
				.await
				.expect("stale claim must succeed");
			assert_eq!(state, seeded);
		}

		let missing_record = without_task_record(seeded.clone(), &core_storage, "claimable").await;
		let missing_record_link = storage.set_value(&missing_record).await.unwrap();
		let (state, _link) = try_reduce_action(
			&storage,
			&core_storage,
			missing_record_link.into(),
			BoardAction::TaskMove {
				from_list: Some("backlog".to_owned()),
				list: "doing".to_owned(),
				task: "claimable".to_owned(),
				after: None,
				lock: TaskLock::Lock("token-a".to_owned()),
			},
		)
		.await
		.expect("claim with a missing task-map record must succeed");
		assert_eq!(state, missing_record);
	}

	#[tokio::test]
	async fn task_move_claim_moves_once_and_sets_the_exact_lock() {
		let storage = MemoryBlockStorage::default();
		let core_storage = CoreBlockStorage::new(storage.clone(), false);

		let (_state, link) = seed_list(&storage, &core_storage, OptionLink::none(), "backlog").await;
		let (_state, link) = seed_list(&storage, &core_storage, link.into(), "doing").await;
		let (_state, link) =
			seed_task(&storage, &core_storage, link.into(), "backlog", task("claimable", Tags::new(), None)).await;
		let (claimed, link) = reduce_action(
			&storage,
			&core_storage,
			link.into(),
			BoardAction::TaskMove {
				from_list: Some("backlog".to_owned()),
				list: "doing".to_owned(),
				task: "claimable".to_owned(),
				after: None,
				lock: TaskLock::Lock("token-a".to_owned()),
			},
		)
		.await;

		assert_eq!(list_task_ids(&claimed, &core_storage, "backlog").await, Vec::<String>::new());
		assert_eq!(list_task_ids(&claimed, &core_storage, "doing").await, ["claimable"]);
		assert_eq!(
			claimed
				.tasks
				.get(&core_storage, &"claimable".to_owned())
				.await
				.unwrap()
				.unwrap()
				.lock,
			Some("token-a".to_owned())
		);

		let (replayed, link) = try_reduce_action(
			&storage,
			&core_storage,
			link.into(),
			BoardAction::TaskMove {
				from_list: Some("backlog".to_owned()),
				list: "doing".to_owned(),
				task: "claimable".to_owned(),
				after: None,
				lock: TaskLock::Lock("token-a".to_owned()),
			},
		)
		.await
		.expect("replayed claim must succeed");
		assert_eq!(replayed, claimed);

		let (competing, _link) = try_reduce_action(
			&storage,
			&core_storage,
			link.into(),
			BoardAction::TaskMove {
				from_list: Some("backlog".to_owned()),
				list: "doing".to_owned(),
				task: "claimable".to_owned(),
				after: None,
				lock: TaskLock::Lock("token-b".to_owned()),
			},
		)
		.await
		.expect("competing stale claim must succeed");
		assert_eq!(competing, claimed);
		assert_eq!(list_task_ids(&competing, &core_storage, "doing").await, ["claimable"]);
		assert_eq!(
			competing
				.tasks
				.get(&core_storage, &"claimable".to_owned())
				.await
				.unwrap()
				.unwrap()
				.lock,
			Some("token-a".to_owned())
		);
	}

	#[tokio::test]
	async fn list_create_existing_name_is_noop() {
		let storage = MemoryBlockStorage::default();
		let core_storage = CoreBlockStorage::new(storage.clone(), false);

		let (_state, link) = seed_list(&storage, &core_storage, OptionLink::none(), "backlog").await;
		let (_state, link) = seed_list(&storage, &core_storage, link.into(), "doing").await;
		let (_state, link) = reduce_action(
			&storage,
			&core_storage,
			link.into(),
			BoardAction::ListTags("backlog".to_owned(), TagsAction::insert(tags(&[("queue", "ready")]))),
		)
		.await;
		let (seeded, link) =
			seed_task(&storage, &core_storage, link.into(), "backlog", task("existing", Tags::new(), None)).await;

		let (state, _link) = try_reduce_action(
			&storage,
			&core_storage,
			link.into(),
			BoardAction::ListCreate { list: List::new("backlog"), after: Some("doing".to_owned()) },
		)
		.await
		.expect("duplicate list creation must succeed");

		assert_eq!(state, seeded);
		assert_eq!(list_names(&state, &core_storage).await, ["backlog", "doing"]);
		assert_eq!(list_task_ids(&state, &core_storage, "backlog").await, ["existing"]);
		assert_eq!(list_by_name(&state, &core_storage, "backlog").await.tags, tags(&[("queue", "ready")]));
		assert!(state.tasks.get(&core_storage, &"existing".to_owned()).await.unwrap().is_some());
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
