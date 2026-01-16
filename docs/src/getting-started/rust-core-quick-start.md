# Core Quick Start
In this quick-start guide, we are implementing the data model for a basic to-do list.  
First, we will set up our Rust crate, and then we will implement our Core.

## Setup
Run the following to set up our new Rust crate, and add [`co-api`](/crate/co_api/index.html) dependency.  
Then we need to add the `serde` and `anyhow` crates.

```sh
cargo init --lib ./my-todo-core
cd ./my-todo-core
cargo add co-api --git https://gitlab.1io.com/1io/co-sdk.git
cargo add serde anyhow
```

Add the following two parts to the end of the `./my-todo-core/Cargo.toml` file:

```toml
[lib]
crate-type = ["lib", "cdylib"]
```

We configure the crate-type as [`cdylib`](https://doc.rust-lang.org/reference/linkage.html#r-link.cdylib) to link a `wasm` file.

```toml
[features]
"core" = []
```

This feature will be activated when compiling the crate to a CO-kit-compatible WebAssembly binary.

## Implementation
Now we implement the core in `src/lib.rs`.

```admonish info
Please note that you can safely delete any example Rust code in the 'lib.rs' file. 
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
Here we define a simple to-do task data model:
- `TodoTask` → single task (id, title, done flag)
- `Todo` → state container with a map of tasks (i.e. a to-do list)

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
- Create, complete, un-complete, rename, and delete tasks
- Bulk-delete all completed tasks

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
We implement how the events are applied to the existing state:
- Loads state + event → modifies task map → stores updated state
- Each `TodoAction` maps directly to a state change

#### 4. Imports

For completeness, here are the imports to add to the top of your file:

```rust
use co_api::{async_api::Reducer, co, BlockStorage, BlockStorageExt, CoMap, Link, OptionLink, ReducerAction};
```

## Build as WebAssembly
The following command compiles the Core to WebAssembly:
```sh
co core build
```

```admonish info
Please ensure that you run this command in the `my-todo-core` folder.
```

You should find the resulting `.wasm` file at:  
```sh
/my-todo-core/target-wasm/wasm32-unknown-unknown/release/my_todo_core.wasm
```

## Full example

You can find the full example as a git project here:
- [my-todo-core - 1io / example-todo-list - GitLab](https://gitlab.1io.com/1io/example-todo-list/-/blob/main/my-todo-core)
