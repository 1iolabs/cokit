# Core Quick Start
In this quick-start guide, we are implementing the data model for a basic to-do list.
First, we will set up our Rust crate, and then we will implement our Core.

## Setup
Run the following to set up our new Rust crate, and add [`co-api`](/crate/co_api/index.html) dependency
```sh
cargo init --lib ./my-todo-core
cd ./my-todo-core
cargo add co-api --git https://gitlab.1io.com/1io/co-sdk.git
```

## Implementation
Now we implement the core in `src/lib.rs`.

```admonish info
Please note that there will be some example Rust code in the 'lib.rs' file. You can safely delete the example code, as it is not required.
```

#### 1. Define your data model in a core:
```rust
#[co]
pub struct TodoTask {
  pub id: String,
  pub title: String,
  pub done: bool,
}

#[co(state)]
pub struct Todo {
  pub tasks: CoMap<String, TodoTask>,
}
```
Here we define a simple todo task data model:
- `TodoTask` → single task (id, title, done flag)
- `Todo` → state container with a map of tasks

#### 2. Define how the state can be modified:
```rust
#[co]
pub enum TodoAction {
  TaskCreate(TodoTask),
  TaskDone { id: String },
  TaskUndone { id: String },
  TaskSetTitle { id: String, title: String },
  TaskDelete { id: String },
  DeleteAllDoneTasks,
}
```
Here we enumerate all state-changing events:
- Create, complete, un-complete, rename, delete tasks
- Bulk-delete completed tasks

#### 3. Define how the modifications are applied:
```rust
impl<S> Reducer<TodoAction, S> for Todo
where
	S: BlockStorage + Clone + 'static,
{
	async fn reduce(
		state_link: OptionLink<Self>,
		event_link: Link<ReducerAction<TodoAction>>,
		storage: &S,
	) -> Result<Link<Self>, anyhow::Error> {
		let event = storage.get_value(&event_link).await?;
		let mut state = storage.get_value_or_default(&state_link).await?;
		let mut tasks = state.tasks.open(storage).await?;
		match event.payload {
			TodoAction::TaskCreate(todo_task) => {
				tasks.insert(todo_task.id.clone(), todo_task).await?;
			},
			TodoAction::TaskDone { id } => {
				tasks.update(id, move |task| task.done = true).await?;
			},
			TodoAction::TaskUndone { id } => {
				tasks.update(id, move |task| task.done = false).await?;
			},
			TodoAction::TaskSetTitle { id, title } => {
				tasks.update(id, move |task| task.title = title).await?;
			},
			TodoAction::TaskDelete { id } => {
				tasks.remove(id).await?;
			},
			TodoAction::DeleteAllDoneTasks => {
				tasks.remove_stream(tasks.stream_filter(|task| task.done)).await?;
			},
		}
		state.tasks = tasks.store().await?;
		Ok(storage.set_value(&state).await?)
	}
}
```
Here we implement how the events are applied to the existing state:
- Loads state + event → modifies task map → stores updated state
- Each `TodoAction` maps directly to a state change

#### 4. For completeness, here are the imports:
```rust
use co_api::{async_api::Reducer, co, BlockStorage, BlockStorageExt, CoMap, Link, OptionLink, ReducerAction};
```

## Build as WebAssembly
To compile to WebAssembly use the following command:
```sh
co core build
```

## Full example

You can find the full example as a git project here:
- [my-todo-core - 1io / example-todo-list - GitLab](https://gitlab.1io.com/1io/example-todo-list/-/blob/main/my-todo-core)
