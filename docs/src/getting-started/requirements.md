# Requirements
#todo
You'll need the following to start right off:

- Rust
- Cargo
- Rust Toolchain

## Building your first app
Lets build a collaborative todo list.

1. Define your data model in a core
```rust
/// A todo task.
#[co]
pub struct TodoTask {
  /// Task UUID.
  pub id: String,

  /// Task title.
  pub title: String,

  /// Whether the task is done.
  pub done: bool,

  /// The assigned participant.
  pub assigne: Option<Did>,
}

/// The todo core state.
#[co(state)]
pub struct Todo {
  /// Tasks.
  pub tasks: CoMap<String, TodoTask>,
}
```
2. Define how it can be modified:
```rust
/// Todo task actions.
#[co]
pub enum TodoAction {
  TaskCreate(TodoTask),
  TaskDone { id: String },
  TaskUndone { id: String },
  TaskSetTitle { id: String, title: String },
  TaskAssign { id: String, assigne: Did },
  TaskUnassign { id: String },
  DeleteAllDoneTasks,
}
```
3. Define how the modifications are applied:
```rust
impl<S> Reducer<TodoAction, S> for Todo
# where
# 	S: BlockStorage + Clone + 'static,
# {
async fn reduce(
 	state_link: OptionLink<Self>,
 	event_link: Link<ReducerAction<TodoAction>>,
	storage: &S,
) -> Result<Link<Self>, anyhow::Error> {
# 	let event = storage.get_value(&event_link).await?;
# 	let mut state = storage.get_value_or_default(&state_link).await?;
  match event.payload {
      TodoAction::TaskCreate(todo_task) => {
        state.tasks.insert(storage, todo_task.id.clone(), todo_task).await?;
      },
      TodoAction::TaskDone { id } => {
        state
          .tasks
          .update_sync(storage, id.clone(), move |task| {
            task.done = true;
          })
          .await?;
      },
#        TodoAction::TaskUndone { id } => {
#        	state
#        		.tasks
#       	 	.update_sync(storage, id.clone(), move |task| {
#       			task.done = false;
#       		})
#       		.await?;
#       },
#       TodoAction::TaskSetTitle { id, title } => {
#       	state
#       		.tasks
#       		.update_sync(storage, id.clone(), move |task| {
#       			task.title = title;
#       		})
#       		.await?;
#       },
#       TodoAction::TaskAssign { id, assigne } => {
#       	state
#       		.tasks
#       		.update_sync(storage, id.clone(), move |task| {
#       			task.assigne = Some(assigne);
#       		})
#       		.await?;
#       },
#       TodoAction::TaskUnassign { id } => {
#       	state
#       		.tasks
#       		.update_sync(storage, id.clone(), move |task| {
#       			task.assigne = None;
#       		})
#       		.await?;
#       },
      TodoAction::DeleteAllDoneTasks => {
          let mut tasks = state.tasks.open(storage).await?;
          let remove_task_ids = tasks
              .stream()
              .try_filter_map(|(id, task)| ready(Ok(if task.done { Some(id) } else { None })))
              .try_collect::<Vec<String>>()
              .await?;
          for task in remove_task_ids {
              tasks.remove(task).await?;
          }
          state.tasks = tasks.store().await?;
      },
    }
#    Ok(storage.set_value(&state).await?)
#  }
}
```
