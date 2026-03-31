# Rust App Quick Start

As a very simple UI building tool, we use [Dioxus](https://dioxuslabs.com/) in this tutorial.
We also use TailwindCSS for styling the application.

## Table of Contents

<!-- toc -->

## Requirements
- `dioxus-0.7.3`
- `npm`

## Setup
Install Dioxus and setup the empty application crate.

### Setup Dioxus
Install the precompiled `dx` tool:
```shell
cargo binstall dioxus-cli@0.7.3
```

You can also head over to [Dioxus](https://dioxuslabs.com/learn/0.7/getting_started/#install-the-dioxus-cli) for further instructions.

### Setup NodeJS
We need NodeJS to use TailwindCSS within our app.  
Head over to [NodeJS](https://nodejs.org/en/download) for download instructions.

### Application
We need to setup a new Rust crate for the application.  
This next command will create a `my-todo-app` folder.

```admonish info
Ideally, both `my-todo-core` and `my-todo-app` should be located under the same parent folder.
```

1. Initialize Dioxus application:
```sh
dx new my-todo-app --subtemplate Bare-Bones -o is_fullstack=false -o is_router=false -o default_platform=desktop -o is_tailwind=true
```
2. In the `my-todo-app` folder, install `co-sdk`, `co-core-membership`, `co-core-co` and `co-dioxus` (the dioxus integration) as dependencies:
```sh
cd my-todo-app
cargo add co-sdk co-dioxus co-core-membership co-core-co --git https://github.com/1iolabs/cokit.git
```
3. Install our to-do list Core as a dependency:
```sh
cargo add my-todo-core --path ../my-todo-core
```
4. Install the `futures` crate as a dependency:
```sh
cargo add futures
```
5. Install the `uuid` crate as a dependency:
```sh
cargo add uuid --features v7
```
6. Setup Tailwind
```sh
npm init -y
npm install -D tailwindcss @tailwindcss/cli daisyui
```

## Implementation

For this example we use the MyTodoCore.

Upon first starting the application, a `did:key:` identity is created locally.  
We name it `my-todo-identity`.

The first view is where we create to-do lists, and respond to invites.  
The second view is where we manage tasks and participants.

### Application

For this quick-start example, we only use a single file: `src/main.rs`.

```admonish info
The steps below detail how to use COKIT to create an App with Rust.  

However, if you wish to try out the complete Rust App with Dioxus, please see the [Full Example Code](#full-example-code) at the end of this page.
```

#### Setup
In the main function we initialize COKIT and pass it as context to dioxus:

```rust,noplayground
fn main() {
    // co
    let context = co_dioxus::CoContext::new(co_dioxus::CoSettings::cli("my-todo-app"));

    // app
    LaunchBuilder::desktop().with_context(context).launch(App);
}
```

#### Overview
Next, we want to display a list of to-do lists and possible invites.  


##### Memberships/Invites
We use the COKIT built-in [memberships Core](/crate/co_core_membership/struct.Memberships.html), and show to-do lists according their state.  
We're working with the following states of interest:  
- Active: Normal active membership
- Invite: We were invited to join a [CO](../reference/co.md) by someone else
- Join: We accepted an invite and are waiting for it to complete

The Dioxus bindings mostly work by using hooks.  
We are using the following:
- [`use_co`](/crate/co_dioxus/fn.use_co.html) : Opens a CO for read/write
- [`use_selector`](/crate/co_dioxus/fn.use_selector.html) : Selects relevant state from an opened CO
- [`use_did_key_identity`](/crate/co_dioxus/fn.use_did_key_identity.html) : Uses a `did:key:` identity

We read the memberships that are stored in the [Local CO](../reference/co.md#local-co):

```rust,noplayground
let local_co_id = use_signal(|| CoId::new(CO_ID_LOCAL));
let local_co = use_co(local_co_id.into());
let lists = use_selector(&local_co, move |storage, co_state| async move {
	Ok(state::memberships(storage, co_state.co())
		.try_filter(move |item| ready(item.0.as_str() != CO_ID_LOCAL))
		.try_collect::<Vec<_>>()
		.await?)
})?;
```

The [`co_sdk::state`](/crate/co_sdk/state/index.html) export contains utilities to access states of built-in Cores.

Here we use [`co_sdk::state::memberships`](/crate/co_sdk/state/fn.memberships.html).  
This contains all memberships, including the virtual one of the [Local CO](../reference/co.md#local-co) that we'll filter out:

We also use [`co_sdk::state::co_info`](/crate/co_sdk/state/fn.co_info.html) to return info about the CO.

##### Counter
For every To-do List, we want to show a counter of undone to-dos:

```rust,noplayground
let co = use_co(co_id);
let (co_info, undone) = use_selector(&co, move |storage, co_state| async move {
	let info = state::co_info(&storage, co_state.co()).await?;
	let todo: Todo = state::core_or_default(&storage, co_state.co(), TODO_CORE_NAME).await?;
	let undone = todo
		.tasks
		.stream(&storage)
		.try_fold(0usize, |state, item| ready(Ok(state + if item.1.done { 0 } else { 1 })))
		.await?;
	Ok((info, undone))
})?;
```

We use the CO, and use the selector again to count the items.  
Since all the states are available locally, we can iterate through the items while counting them.  
In a more sophisticated example, the values could be pre-calculated using the Core.

Later, we need to add handlers to create and join a CO.

#### To-do List
Now we want to show all the to-do items and the participants of a specific CO.  
We use the selector again and extract the values we need:

```rust,noplayground
let co = use_co(co_id);
let (name, participants, tasks_core_exists, tasks) = use_selector(&co, move |storage, co_state| async move {
	let co = state::co(&storage, co_state.co()).await?;
	let (tasks_core_exists, tasks) = match state::core::<Todo>(&storage, co_state.co(), "todo").await {
		Ok(todo) => Ok((
			true,
			todo.tasks
				.stream(&storage)
				.map_ok(|(_id, task)| task)
				.try_collect::<Vec<_>>()
				.await?,
		)),
		Err(state::QueryError::NotFound(_)) => Ok((false, Default::default())),
		Err(err) => Err(err),
	}?;
	let participants = co
		.participants
		.stream(&storage)
		.map_ok(|(_key, item)| item)
		.try_collect::<Vec<_>>()
		.await?;
	Ok((co.name, participants, tasks_core_exists, tasks))
})?;
```

We select:
- the basic information about the CO (like its name) for display.
- the participants of the CO.
- if the to-do Core has already been added to this CO. If not we will create it on-the-fly later.
- the tasks from the to-do Core.

##### Handlers
We now need to define the handlers that respond to user actions.

First, we define `on_todo_action`, which will push an action into the to-do Core.  
It will also create the to-do Core on-the-fly, if it does not yet exist.

```rust,noplayground
let on_todo_action = use_callback({
	let identity = identity.clone();
	let co = co.clone();
	move |action| {
		let identity = identity.cloned();
		if !tasks_core_exists {
			co.create_core_binary(identity.clone(), TODO_CORE_NAME, "my-todo-core", TODO_CORE_BINARY);
		}
		co.dispatch(identity, "todo", action);
	}
});
```

Now we define the different handlers:

```rust,noplayground
let on_create_task = use_callback(move |title| {
	on_todo_action(TodoAction::TaskCreate(TodoTask { id: uuid::Uuid::new_v4().to_string(), title, done: false }));
});
let on_done = use_callback(move |(id, done)| {
	on_todo_action(if done { TodoAction::TaskDone { id } } else { TodoAction::TaskUndone { id } });
});
let on_delete = use_callback(move |id| {
	on_todo_action(TodoAction::TaskDelete { id });
});
let on_edit = use_callback(move |(id, title)| {
	on_todo_action(TodoAction::TaskSetTitle { id, title });
});
let on_delete_all_done = use_callback(move |_| {
	on_todo_action(TodoAction::DeleteAllDoneTasks);
});
```

##### Render
Next, we can render the task list using Dioxus.

```rust,noplayground
for task in tasks {
	TodoItem { task: task.clone(), on_done: on_done, on_delete: on_delete, on_edit: on_edit }
}
```

### Run the App

Follow the instructions in `my-todo-app/README.md` to run the Tailwind CSS compiler and serve your App.



## Full Example Code
- Here is a full example of the code parts in `main.rs` that integrate with COKIT.  
- Below that is the content for your `tailwind.css` file.  
- And, at the bottom of this page, you will find a link to the entire git project.

```admonish info
Please be sure to 'unhide' all of the code if you are copy/pasting into your `main.rs` file.
```

`src/main.rs`:

```rust,noplayground
# use co_core_co::{CoAction, Participant, ParticipantState};
# use co_core_membership::{MembershipState, MembershipsAction};
# use co_dioxus::{use_co, use_did_key_identity, use_selector, CoContext, CoSettings};
# use co_sdk::{
# 	state::{self, Identity},
# 	tags, CoId, CreateCo, Did, CO_CORE_NAME_CO, CO_CORE_NAME_MEMBERSHIP, CO_ID_LOCAL,
# };
# use dioxus::prelude::*;
# use futures::TryStreamExt;
# use my_todo_core::{Todo, TodoAction, TodoTask};
# use std::future::ready;
#
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");
const TODO_CORE_NAME: &str = "todo";
const TODO_IDENTITY_NAME: &str = "my-todo-identity";
const TODO_CORE_BINARY: &[u8] = include_bytes!("../../my-todo-core/target-wasm/wasm32-unknown-unknown/release/my_todo_core.wasm");

fn main() {
	// co
	let context = CoContext::new(CoSettings::cli("my-todo-app"));

	// app
	LaunchBuilder::desktop().with_context(context).launch(App);
}

#[component]
pub fn TodoOverview(on_open: EventHandler<CoId>) -> Element {
	// hooks
	let local_co_id = use_signal(|| CoId::new(CO_ID_LOCAL));
	let identity = use_did_key_identity(TODO_IDENTITY_NAME)?;
	let local_co = use_co(local_co_id.into());
	let lists = use_selector(&local_co, move |storage, co_state| async move {
		Ok(state::memberships(storage, co_state.co())
			.try_filter(move |item| ready(item.0.as_str() != CO_ID_LOCAL))
			.try_collect::<Vec<_>>()
			.await?)
	})?;
	let on_join = use_callback({
		let identity = identity.clone();
		let local_co = local_co.clone();
		move |co: CoId| {
			let identity = identity.cloned();
			local_co.dispatch(
				identity.clone(),
				CO_CORE_NAME_MEMBERSHIP,
				MembershipsAction::ChangeMembershipState {
					did: identity.did.clone(),
					id: co,
					membership_state: MembershipState::Join,
				},
			);
		}
	});
	let on_create_co = use_callback({
		let identity = identity.clone();
		let local_co = local_co.clone();
		move |name: String| {
			let identity = identity.cloned();
			local_co.create_co(
				identity,
				CreateCo::generate(name).with_core_bytes(TODO_CORE_NAME, "my-todo-core", TODO_CORE_BINARY),
			);
		}
	});

	// render
	let identity = identity.cloned();
	rsx! {
		div {
			class: "flex flex-col h-full",

			// render navigation
			NavBar { left: rsx! {}, center: rsx! {"Todo App"}, right: rsx! {} }

			// render todo lists
			div {
				class: "grow shrink flex flex-col p-4 gap-4",
				div {
					class: "grow shrink overflow-y-auto bg-base-100 border-base-300 shadow-sm collapse border",
					TodoListCreate { show_initially: lists.is_empty(), on_create_co }
					ul {
						class: "list min-h-0",
						for (co_id, _, _, membership_state) in lists {
							if membership_state == MembershipState::Invite || membership_state == MembershipState::Join {
								TodoListJoin { co_id, pending: membership_state != MembershipState::Invite, on_join }
							}
							else if membership_state == MembershipState::Active {
								TodoListElement { co_id, on_join, on_open }
							}
						}
					}
				}
				div { class: "flex-none card bg-base-100 shadow-sm p-2", "Your identity: {identity.did}" }
			}
		  }
	}
}

#[component]
pub fn TodoList(co_id: ReadOnlySignal<CoId>, on_back: EventHandler<()>) -> Element {
	let co = use_co(co_id);
	let (name, participants, tasks_core_exists, tasks) = use_selector(&co, move |storage, co_state| async move {
		let co = state::co(&storage, co_state.co()).await?;
		let (tasks_core_exists, tasks) = match state::core::<Todo>(&storage, co_state.co(), "todo").await {
			Ok(todo) => Ok((
				true,
				todo.tasks
					.stream(&storage)
					.map_ok(|(_id, task)| task)
					.try_collect::<Vec<_>>()
					.await?,
			)),
			Err(state::QueryError::NotFound(_)) => Ok((false, Default::default())),
			Err(err) => Err(err),
		}?;
		let participants = co
			.participants
			.stream(&storage)
			.map_ok(|(_key, item)| item)
			.try_collect::<Vec<_>>()
			.await?;
		Ok((co.name, participants, tasks_core_exists, tasks))
	})?;
	let identity = use_did_key_identity(TODO_IDENTITY_NAME)?;
	let on_todo_action = use_callback({
		let identity = identity.clone();
		let co = co.clone();
		move |action| {
			let identity = identity.cloned();
			if !tasks_core_exists {
				co.create_core_binary(identity.clone(), TODO_CORE_NAME, "my-todo-core", TODO_CORE_BINARY);
			}
			co.dispatch(identity, "todo", action);
		}
	});
	let on_create_task = use_callback(move |title| {
		on_todo_action(TodoAction::TaskCreate(TodoTask { id: uuid::Uuid::now_v7().to_string(), title, done: false }));
	});
	let on_done = use_callback(move |(id, done)| {
		on_todo_action(if done { TodoAction::TaskDone { id } } else { TodoAction::TaskUndone { id } });
	});
	let on_delete = use_callback(move |id| {
		on_todo_action(TodoAction::TaskDelete { id });
	});
	let on_edit = use_callback(move |(id, title)| {
		on_todo_action(TodoAction::TaskSetTitle { id, title });
	});
	let on_delete_all_done = use_callback(move |_| {
		on_todo_action(TodoAction::DeleteAllDoneTasks);
	});
	let on_invite = use_callback({
		let identity = identity.clone();
		let co = co.clone();
		move |(name, did): (String, Did)| {
			let identity = identity.cloned();
			co.dispatch(
				identity,
				CO_CORE_NAME_CO,
				CoAction::ParticipantInvite { participant: did, tags: tags!("name": name) },
			);
		}
	});

	// render
	rsx! {
		div {
			class: "flex flex-col h-full",

			// render navigation
			NavBar {
				left: rsx!{ button { class: "btn btn-ghost", onclick: move |_| on_back(()), "◀" } },
				center: rsx!{"{name}"},
				right: rsx!{
					TodoListParticipants { identity, participants }
					TodoListMenu { on_delete_all_done }
				}
			}

			// render todos
			div {
				class: "grow flex flex-col card bg-base-100 shadow-sm m-4 p-4 min-h-0",
				CreateTodoItem { on_create_task }
				ul {
					class: "list grow shrink overflow-y-auto min-h-0",
					for task in tasks {
						TodoItem { task: task.clone(), on_done: on_done, on_delete: on_delete, on_edit: on_edit }
					}
				}
			}

			// render invite dialog
			TodoListInvite { co_name: name, on_invite }
		}
	}
}

# /// Render application's current view.
# #[component]
# fn App() -> Element {
# 	let mut active_co_id = use_signal(|| Option::<CoId>::None);
# 	let on_open = use_callback(move |co: CoId| active_co_id.set(Some(co)));
# 	let on_back = use_callback(move |_| active_co_id.set(None));
# 	rsx! {
# 		document::Link { rel: "stylesheet", href: TAILWIND_CSS }
# 		AppContainer {
# 			if let Some(active_co_id) = active_co_id.cloned() {
# 				TodoList {
# 					co_id: active_co_id,
# 					on_back,
# 				}
# 			}
# 			else {
# 				TodoOverview {
# 					on_open,
# 				}
# 			}
# 		 }
# 	}
# }
#
# /// Render application with error and suspense fallback.
# #[component]
# fn AppContainer(children: Element) -> Element {
# 	rsx! {
# 		ErrorBoundary {
# 			handle_error: |errors: ErrorContext| rsx! {
# 				pre { "Oops, we encountered an error: {errors:#?}" }
# 			},
# 			SuspenseBoundary {
# 				fallback: |context: SuspenseContext| rsx! {
# 					if let Some(placeholder) = context.suspense_placeholder() {
# 						{placeholder}
# 					} else {
# 						"Please wait..."
# 					}
# 				},
# 				div {
# 					class: "bg-base-300 w-screen h-screen",
# 					{children}
# 				}
# 			}
# 		}
# 	}
# }
#
# /// Render text input to create a new Todo List.
# #[component]
# fn TodoListCreate(show_initially: bool, on_create_co: EventHandler<String>) -> Element {
# 	let mut create_co = use_signal::<Option<String>>(|| if show_initially { Some("".to_string()) } else { None });
# 	let on_create_co_toggle = use_callback(move |_| {
# 		create_co.set(if create_co.peek().is_some() { None } else { Some(String::new()) });
# 	});
# 	let on_create_change = use_callback(move |e: Event<FormData>| create_co.set(Some(e.value())));
# 	rsx! {
# 		div {
# 			class: "collapse collapse-plus",
# 			input { r#type: "checkbox", checked: create_co.read().is_some(), onclick: on_create_co_toggle }
# 			div { class: "collapse-title font-semibold", "Todo Lists" }
# 			div {
# 				class: "collapse-content",
# 				form {
# 					class: "join w-full flex",
# 					onsubmit: move |_| { if let Some(name) = create_co.cloned() { on_create_co(name); }} ,
# 					input { r#type: "text", class: "grow input join-item", onchange: on_create_change, value: create_co.cloned().unwrap_or_default(), placeholder: "Create # list ..." }
# 					button { class: "btn btn-neutral join-item", "Create" }
# 				}
# 			}
# 		}
# 	}
# }
#
# /// Render a Todo List element for an active CO membership in the Overview.
# #[component]
# pub fn TodoListElement(
# 	co_id: ReadOnlySignal<CoId>,
# 	on_join: EventHandler<CoId>,
# 	on_open: EventHandler<CoId>,
# ) -> Element {
# 	// hooks
# 	let co = use_co(co_id);
# 	let (co_info, undone) = use_selector(&co, move |storage, co_state| async move {
# 		let info = state::co_info(&storage, co_state.co()).await?;
# 		let todo: Todo = state::core_or_default(&storage, co_state.co(), TODO_CORE_NAME).await?;
# 		let undone = todo
# 			.tasks
# 			.stream(&storage)
# 			.try_fold(0usize, |state, item| ready(Ok(state + if item.1.done { 0 } else { 1 })))
# 			.await?;
# 		Ok((info, undone))
# 	})?;
#
# 	// render
# 	rsx! {
# 		SuspenseBoundary {
# 			fallback: {
# 				let co_id = co_id.to_string();
# 				move |_context: SuspenseContext| rsx! {
# 					li { class: "list-row", div { class: "font-bold text-neutral-content", "[{co_id}]" } }
# 				}
# 			},
# 			li {
# 				class: "list-row flex hover:bg-base-300 rounded-none cursor-pointer",
# 				onclick: move |_| on_open(co_id.cloned()),
# 				span { class: "font-bold flex-1", "{co_info.name}" }
# 				if undone > 0 {
# 					div { class: "badge badge-soft badge-secondary", "{undone}" }
# 				}
# 			}
# 		}
# 	}
# }
#
# #[component]
# fn TodoItem(
# 	task: TodoTask,
# 	on_done: EventHandler<(String, bool)>,
# 	on_delete: EventHandler<String>,
# 	on_edit: EventHandler<(String, String)>,
# ) -> Element {
# 	let mut editing = use_signal(|| None);
# 	let on_edit_submit = use_callback({
# 		let task_id = task.id.clone();
# 		move |_| {
# 			if let Some(new_title) = editing.cloned() {
# 				editing.set(None);
# 				on_edit.call((task_id.clone(), new_title));
# 			}
# 		}
# 	});
# 	let on_edit_cancel = use_callback({
# 		move |_| {
# 			editing.set(None);
# 		}
# 	});
# 	let on_click_edit = use_callback({
# 		let task_title = task.title.clone();
# 		move |_| {
# 			editing.set(Some(task_title.clone()));
# 		}
# 	});
# 	let on_edit_delete = use_callback({
# 		let task_id = task.id.clone();
# 		move |_| {
# 			editing.set(None);
# 			on_delete(task_id.clone());
# 		}
# 	});
# 	let on_click_done = use_callback({
# 		let task_id = task.id.clone();
# 		let task_done = task.done;
# 		let on_done = on_done.clone();
# 		move |_| {
# 			on_done((task_id.clone(), !task_done));
# 		}
# 	});
# 	let on_key_press = use_callback({
# 		let task_title = task.title.clone();
# 		move |event: KeyboardEvent| match event.key() {
# 			Key::Enter => {
# 				if editing.read().is_none() {
# 					editing.set(Some(task_title.clone()));
# 				}
# 			},
# 			Key::Escape => {
# 				editing.set(None);
# 			},
# 			_ => {},
# 		}
# 	});
# 	rsx! {
# 		li {
# 			class: "list-row flex items-center",
# 			onkeypress: on_key_press,
# 			if let Some(new_title) = editing.cloned() {
# 				form {
# 					class: "join w-full flex",
# 					onsubmit: on_edit_submit,
# 					input {
# 						value: "{new_title}",
# 						class: "grow input join-item",
# 						oninput: move |e| editing.set(Some(e.value().to_string())),
# 						onmounted: move |e| async move { let _ = e.set_focus(true).await; }
# 					}
# 					button { class: "btn join-item", type: "submit", "Save" }
# 					button { class: "btn join-item", onclick: on_edit_delete, "Delete" }
# 					button { class: "btn join-item", onclick: on_edit_cancel, "Cancel" }
# 				}
# 			} else {
# 				input { class: "checkbox", r#type: "checkbox", checked: "{task.done}", onclick: on_click_done }
# 				div {
# 					class: "grow shrink p-2",
# 					ondoubleclick: on_click_edit,
# 					span { style: if task.done { "text-decoration: line-through;" } else { "" }, "{task.title}" }
# 				}
# 			}
# 		}
# 	}
# }
#
# #[component]
# fn CreateTodoItem(on_create_task: EventHandler<String>) -> Element {
# 	let mut new_title = use_signal(String::new);
# 	let add_task = use_callback(move |_| {
# 		let title = new_title.read().trim().to_string();
# 		if !title.is_empty() {
# 			on_create_task(title);
# 			new_title.set(String::new());
# 		}
# 	});
# 	rsx! {
# 		form {
# 			class: "flex-none join w-full",
# 			onsubmit: add_task,
# 			input { class: "grow input join-item", placeholder: "New task...", value: "{new_title}", oninput: move |e| new_title.set(e.value().to_string()) }
# 			button { class: "btn btn-primary join-item text-xl font-bold", type: "submit", "+" }
# 		}
# 	}
# }
#
# /// Render a TodoList item for a invite.
# #[component]
# fn TodoListJoin(co_id: ReadOnlySignal<CoId>, pending: bool, on_join: EventHandler<CoId>) -> Element {
# 	rsx! {
# 		li {
# 			class: "list-row flex items-center hover:bg-base-300 cursor-pointer",
# 			div { class: "grow font-bold", "[{co_id}]" }
# 			button {
# 				class: "btn btn-square w-20 bg-warning",
# 				disabled: pending,
# 				onclick: move |_| on_join(co_id.cloned()),
# 				"Join"
# 			}
# 		}
# 	}
# }
#
# /// Render TodoList participants.
# #[component]
# fn TodoListParticipants(identity: MappedSignal<Identity>, participants: Vec<Participant>) -> Element {
# 	let participant_tag = {
# 		let you = identity.read().did.clone();
# 		move |participant: &co_core_co::Participant| {
# 			if participant.did == you {
# 				return "Y".to_owned();
# 			}
# 			if let Some(name) = participant.tags.string("name") {
# 				if let Some(tag) = name.chars().next() {
# 					return tag.to_string();
# 				}
# 			}
# 			if let Some(last) = participant.did.chars().last() {
# 				return last.to_string().to_uppercase();
# 			}
# 			"?".to_owned()
# 		}
# 	};
# 	let participant_bg = {
# 		let you = identity.read().did.clone();
# 		move |participant: &co_core_co::Participant| {
# 			if participant.did == you {
# 				return "bg-primary";
# 			}
# 			match participant.state {
# 				ParticipantState::Active => "bg-neutral",
# 				_ => "bg-warning",
# 			}
# 		}
# 	};
# 	let participant_name = {
# 		let you = identity.read().did.clone();
# 		move |participant: &co_core_co::Participant| {
# 			if participant.did == you {
# 				return "You".to_owned();
# 			}
# 			participant.tags.string("name").unwrap_or(&participant.did).to_owned()
# 		}
# 	};
# 	rsx! {
# 		div {
# 			class: "-space-x-2",
# 			for participant in participants {
# 				div {
# 					class: "dropdown dropdown-end",
# 					div {
# 						tabindex: "0",
# 						role: "button",
# 						class: "{participant_bg(&participant)} text-neutral-content w-6 rounded-full",
# 						span { class: "text-xs text-center align-[2px]", "{participant_tag(&participant)}" }
# 					}
# 					div {
# 						tabindex: "0",
# 						class: "dropdown-content card card-sm bg-base-100 z-1 p-2 shadow-md",
# 						div {
# 							tabindex: "0",
# 							class: "card-body",
# 							span { class: "font-bold", "{participant_name(&participant)}" }
# 							span { class: "text-xs", "{participant.did}" }
# 							if participant.state == ParticipantState::Invite {
# 								span { class: "font-bold font-xl text-warning", "Invite pending" }
# 							}
# 						}
# 					}
# 				}
# 			}
# 		}
# 	}
# }
#
# /// Render Invite Dialog.
# #[component]
# fn TodoListInvite(co_name: ReadOnlySignal<String>, on_invite: EventHandler<(String, Did)>) -> Element {
# 	let mut invite_name = use_signal(|| String::new());
# 	let mut invite_did = use_signal(|| String::new());
# 	rsx! {
# 		input { r#type: "checkbox", id: "dialog_invite", class: "modal-toggle" }
# 		div {
# 			class: "modal",
# 			role: "dialog",
# 			div {
# 				class: "modal-box",
# 				label { class: "btn btn-sm btn-circle btn-ghost absolute right-2 top-2", r#for: "dialog_invite", "✕" }
# 				h3 { class: "text-lg font-bold", "Invite participant to {co_name}" }
# 				p { class: "py-4", "Invite new participant using a DID." }
# 				fieldset {
# 					class: "fieldset",
# 					legend { class: "fieldset-legend", "Name" }
# 					input { r#type: "text", class: "input", placeholder: "Display name...", value: "{invite_name}", onchange: move |e| invite_name.set(e.value()) }
# 					p { class: "label", "The name of the participant in your address book." }
# 					legend { class: "fieldset-legend", "DID" }
# 					input { r#type: "text", class: "input", placeholder: "did:", value: "{invite_did}", onchange: move |e| invite_did.set(e.value()) }
# 					p { class: "label", "The identity of the participant to invite." }
# 				}
# 				div {
# 					class: "modal-action",
# 					label { r#for: "dialog_invite", class: "btn", "Cancel" }
# 					label { r#for: "dialog_invite", class: "btn btn-primary", onclick: move |_| on_invite((invite_name.cloned(), invite_did.cloned())), "Invite" }
# 				}
# 			}
# 			label { class: "modal-backdrop", r#for: "dialog_invite", "Close" }
# 		}
# 	}
# }
#
# /// Render Todo List Menu.
# #[component]
# fn TodoListMenu(on_delete_all_done: EventHandler<()>) -> Element {
# 	rsx! {
# 		div {
# 			class: "dropdown dropdown-end",
# 			div {
# 				tabindex: "0",
# 				class: "btn btn-ghost",
# 				role: "button",
# 				span { class: "text-2xl mb-[4px]", "☰" }
# 			}
# 			ul {
# 				tabindex: "0",
# 				class: "dropdown-content menu bg-base-100 rounded-box z-1 w-52 p-2 shadow-sm",
# 				li { label { r#for: "dialog_invite", "Invite" } }
# 				li { button { onclick: move |_| on_delete_all_done(()), "Delete all done" } }
# 			}
# 		}
# 	}
# }
#
# /// Render Nav Bar.
# #[component]
# fn NavBar(left: Element, center: Element, right: Element) -> Element {
# 	rsx! {
# 		div {
# 			class: "flex-none navbar bg-base-100 shadow-sm",
# 			div { class: "navbar-start", {left} }
# 			div { class: "navbar-center text-3xl font-extrabold", {center} }
# 			div { class: "navbar-end", {right} }
# 		}
# 	}
# }
```


#### Tailwind
`/tailwind.css`:
```css
@import "tailwindcss";
@source "./src/**/*.{rs,html,css}";
@plugin "daisyui";
```


### Full Example Project

You can find the full example as a git project here:
- [1io / example-todo-list - GitLab](https://gitlab.1io.com/1io/example-todo-list.git)
