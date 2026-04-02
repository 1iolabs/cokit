# COKIT

COKIT implementation using the rust programming language.

## Project Status

**Status: Early Public Release** — COKIT is under active development.
APIs may change. Use in production at your own discretion.

## Usage

### Define a Core data structure

```rust
use co_api::{Reducer, co, BlockStorageExt, CoMap, CoreBlockStorage, Link, OptionLink, ReducerAction};

/// Todo task actions.
#[co]
pub enum TodoAction {
	TaskCreate(TodoTask),
	TaskDone { id: String },
	TaskUndone { id: String },
	TaskSetTitle { id: String, title: String },
	TaskDelete { id: String },
	DeleteAllDoneTasks,
}

/// A todo task.
#[co]
pub struct TodoTask {
	/// Task UUID.
	pub id: String,
	/// Task title.
	pub title: String,
	/// Whether the task is done.
	pub done: bool,
}

/// The todo core state.
#[co(state)]
pub struct Todo {
	/// Tasks.
	pub tasks: CoMap<String, TodoTask>,
}
impl Reducer<TodoAction> for Todo {
	async fn reduce(
		state_link: OptionLink<Self>,
		event_link: Link<ReducerAction<TodoAction>>,
		storage: &CoreBlockStorage,
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

For further information, see:
- [Example Todo List](https://gitlab.1io.com/1io/example-todo-list)
- [Documentation](https://www.cokit.org/docs/)

## Development

### Setup

Dependencies:
- `rust-1.91` (MSRV)
- `rustfmt`
- `wasm32-unknown-unknown` to build cores.
- `toolchain nightly` to use `rustfmt +nightly`

Commands:
```shell
rustup component add rustfmt
rustup target add wasm32-unknown-unknown
rustup toolchain install nightly
rustup component add --toolchain nightly rustfmt
```

## Licensing

COKIT is the open-source platform core and is released under **AGPL-3.0-only**.

GUARD is maintained in a separate repository (`guard`) under separate licensing terms.
It is **not** part of the open-source licensing of this repository.

Commercial licensing, support and enterprise terms may be available from **1io BRANDGUARDIAN GmbH**, any **successor in title to the relevant rights**, or any **affiliate expressly authorized by the relevant rights holder**.

Commercial contact: `license@1io.com`  
More information: <https://www.cokit.org>

Additional repository-specific context for this first public AGPL release is provided in:
- `LICENSE.md`
- `NOTICE.md`
- `AI-TRAINING-POLICY.md`

Copyright (C) 2020-2026 1io BRANDGUARDIAN GmbH
