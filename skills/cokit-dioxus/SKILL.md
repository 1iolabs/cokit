---
name: cokit-dioxus
description: >-
  Guides planning and implementation for Dioxus applications built with COKIT's co-dioxus
  crate. Covers initialization (CoContext, CoSettings), all co-dioxus hooks (use_co,
  use_cos, use_selector, use_selector_state, use_selectors, use_selector_states,
  use_co_reducer_state, use_co_id, use_did_key_identity, use_co_context), the reactive
  data-flow model, hierarchical DAG state access, dispatching actions, creating COs,
  and cross-platform feature flags.
  Enforces COKIT-specific Dioxus rules: no optimistic rendering, no copied state, no
  global AppState, COKIT as single source of truth, hooks for all state, derive state
  reactively.
  Trigger this skill whenever the user is building, planning, or modifying a Dioxus
  application that uses co-dioxus, or asks about using COKIT with Dioxus, even if they
  just say "dioxus component" or "add a view" in a COKIT project context.
---

# COKIT Dioxus Integration

For general COKIT concepts (COs, Cores, Log, Identity, Permissions, etc.) see the
[cokit skill](../cokit/SKILL.md). This skill focuses on the Dioxus integration layer.

## Architecture Overview

`co-dioxus` bridges COKIT and Dioxus 0.7. It runs the COKIT `Application` on a
dedicated thread (native) or via `wasm_bindgen_futures` (web), communicates through
an actor-based message channel, and exposes CO state as reactive Dioxus signals.

```
Dioxus UI thread                    COKIT thread
  +-----------+    mpsc channel     +-------------+
  | CoContext  | =================> | Application |
  | (hooks)   | <--- signals ---   | (Cores,     |
  +-----------+                     |  Storage,   |
                                    |  Network)   |
                                    +-------------+
```

The `CoContext` is a Dioxus context provider. Hooks subscribe to CO state via
`SyncSignal`s that the COKIT actor updates whenever the underlying Log changes.

## Initialization

Create a `CoContext` with `CoSettings` and provide it as Dioxus context:

```rust
use co_dioxus::{CoContext, CoSettings};
use dioxus::prelude::*;

fn main() {
    let context = CoContext::new(CoSettings::cli("com.example.my-app", "my-app"));
    LaunchBuilder::desktop().with_context(context).launch(App);
}
```

`CoSettings::cli(bundle_id, instance_id)` parses CLI args (base path, log level,
networking, etc.) via `clap`. Use `CoSettings::new(bundle_id, instance_id)` for
programmatic configuration, then chain builders:

- `.with_path(path)` / `.with_memory()` / `.with_indexeddb(secret)` - storage backend
- `.with_network(network_settings)` - enable P2P networking
- `.without_keychain()` - skip OS keychain (dev only)
- `.with_local_secret(secret)` - encryption secret
- `.with_access_guard(policy)` - CO access guard
- `.with_contact_handler(handler)` - handle contact requests
- `.with_core(cid, core)` / `.with_guard(cid, guard)` - register Cores/Guards
- `.with_log(co_log)` / `.with_log_level(level)` - logging configuration

### Feature Flags

`co-dioxus` uses platform-specific feature flags. The consuming app must enable the
correct target feature:

```toml
[features]
default = ["desktop"]
web = ["dioxus/web", "co-dioxus/web"]
desktop = ["dioxus/desktop", "co-dioxus/desktop"]
mobile = ["dioxus/mobile", "co-dioxus/mobile"]
```

Target features and what they enable:
- `desktop` - filesystem storage, networking, tracing, native async runtime
- `mobile` - filesystem storage, networking, oslog tracing, native async runtime
- `web` - IndexedDB storage, networking, WASM async runtime, console logging

## Reactivity Rules

These rules are fundamental to how co-dioxus applications work. They stem from how
COKIT handles data: COs are local-first and always available, so the patterns common
in client-server apps (optimistic updates, caching layers, global state stores) are
unnecessary and actively harmful.

### 1. No optimistic rendering

COs live locally. When you dispatch an action, the reducer runs immediately on the
local Log. The state update flows back through the reactive signal pipeline within
milliseconds. There is no server round-trip to "optimistically" skip.

Attempting optimistic rendering creates two sources of truth (the optimistic guess
and the actual CO state), leading to flicker and consistency bugs.

### 2. Do not modify data while rendering

This is a core Dioxus principle (Pillar 3 of reactivity). Rendering must be a pure
function of state. Side effects and state mutations belong in event handlers and
callbacks, not in the render path.

### 3. COKIT is the single source of truth

All application state that matters lives in COs. The UI reads state through selectors
and writes state through dispatched actions. This is the only data flow.

### 4. Use hooks for all state-related operations

Every interaction with CO state goes through co-dioxus hooks. Do not reach around
them to access the Application directly from components. `CoContext::try_with_application`
is a last resort for advanced use cases only — simple apps should never need it.

### 5. Do not copy state into memory

Do not extract data from a selector and store it in a local `use_signal`. This creates
a stale copy that diverges from the CO when other participants (or other parts of your
own app) push actions. Read from selectors, write through dispatch.

### 6. Do not invent global AppStates

There is no need for a global `AppState` struct or a Redux-like store. Each CO is its
own reactive unit. Components subscribe to exactly the COs they need via `use_co` +
`use_selector`. This gives you fine-grained reactivity without manual wiring.

### 7. Do not use spawn to calculate derived states

Derived state belongs in `use_selector` / `use_selector_state`, which automatically
re-run when the CO changes. Using `spawn` or `use_future` to compute derived state
bypasses the reactive pipeline and creates race conditions.

## Hierarchical State and the DAG

COKIT state is organized as a DAG (Directed Acyclic Graph) of content-addressed blocks.
This maps naturally to how most apps structure data hierarchically. The two selector
hooks are designed around this:

- **`use_selector_state`** - Top-level entry point. Receives the full `CoReducerState`,
  from which you can access the CO root and navigate down the DAG. Use this at the top
  of a component tree to get the initial state and extract `Link<T>` references to
  pass down.

- **`use_selector`** - For child components. Receives only `CoBlockStorage`. Use this
  when a parent has already given you a `Link<T>` (or `Cid`) and you just need to
  resolve it from storage. Since a `Link<T>` is content-addressed, it changes only
  when the underlying data changes, giving you precise reactivity.

The pattern: parent components use `use_selector_state` to get top-level state, extract
typed `Link<T>` values, and pass them as `ReadSignal<Link<T>>` props to children. Each
child calls `use_selector` with that link to resolve its own data. This way, when data
deep in the DAG changes, only the affected child re-renders.

```rust
// Parent: get top-level state, extract links
#[component]
fn TodoList(co_id: ReadOnlySignal<CoId>) -> Element {
    let co = use_co(co_id);
    let task_links_resource = use_selector_state(&co, move |storage, co_state| async move {
        let todo: Todo = state::core_or_default(&storage, co_state.co(), "todo").await?;
        // Return the links, not the resolved data
        let links: Vec<Link<TodoTask>> = todo.tasks
            .stream(&storage)
            .map_ok(|(_id, link)| link)
            .try_collect()
            .await?;
        Ok(links)
    });
    let task_links: Option<Vec<Link<TodoTask>>> = task_links_resource().transpose()?;
    rsx! {
        if let Some(task_links) = task_links {
            for link in task_links {
                // Pass Co + link down, child resolves its own data
                TodoItem { co: co.clone(), task: link }
            }
        }
    }
}

// Child: resolve a single link from storage
#[component]
fn TodoItem(co: Co, task: ReadOnlySignal<Link<TodoTask>>) -> Element {
    let task_resource = use_selector(&co, move |storage| {
        // Read signal BEFORE async to register reactive subscription
        let task = task.read();
        async move {
            Ok(storage.get_value(&*task).await?)
        }
    });
    let task_data: Option<TodoTask> = task_resource().transpose()?;
    rsx! {
        if let Some(task_data) = task_data {
            div { "{task_data.title}" }
        }
    }
}
```

`BlockStorageExt` (from `co_primitives`) provides methods for resolving links:
- `storage.get_value(&link)` - resolve a `Link<T>` or `OptionLink<T>` to its value
- `storage.get_value_or_default(&option_link)` - resolve with default fallback
- `storage.get_value_or_none(&option_link)` - resolve to `Option<T>`
- `storage.get_deserialized(&cid)` - resolve a raw `Cid` to a typed value

## Hooks Reference

All hooks are re-exported from `co_dioxus`.

### use_co

```rust
fn use_co(co: ReadSignal<CoId>) -> Co
```

Opens a CO and returns a `Co` handle. This is the entry point for interacting with
any CO. The handle provides:

- `co.co()` -> `CoId` - the CO's identifier
- `co.storage()` -> `CoBlockStorage` - block storage for reading content-addressed data
- `co.dispatch(identity, core_name, action)` - fire-and-forget action dispatch
- `co.push(identity, core_name, action).await` - async action dispatch (use with `use_action`)
- `co.create_co(identity, create_co)` - create a new CO (only on Local CO)
- `co.create_core(identity, name, type, cid)` - add a Core to this CO
- `co.create_core_binary(identity, name, type, bytes)` - add a Core from WASM bytes
- `co.last_error()` - check for dispatch errors
- `co.clear_last_error()` - reset error state

The `Co` handle is `Clone` and can be passed into callbacks and child components.

**Common pattern - opening the Local CO:**
```rust
let local_co_id = use_signal(|| CoId::new(CO_ID_LOCAL));
let local_co = use_co(local_co_id.into());
```

### use_cos

```rust
fn use_cos(cos: ReadSignal<Vec<CoId>>) -> Cos
```

Opens multiple COs at once. Returns `Cos`, which derefs to `Vec<Co>`.

### use_selector_state (top-level CO state)

```rust
fn use_selector_state<F, Fut, T>(co: &Co, f: F) -> Resource<Result<T, CoError>>
where
    F: Fn(CoBlockStorage, CoReducerState) -> Fut + Clone + 'static,
```

The primary selector for top-level state access. The closure receives both
`CoBlockStorage` and `CoReducerState`. Use `co_state.co()` to access the CO's root
state, then navigate the DAG from there.

This is a Dioxus `use_resource` under the hood — it re-executes whenever the CO's
reducer state signal changes.

```rust
let resource = use_selector_state(&co, move |storage, co_state| async move {
    let co = state::co(&storage, co_state.co()).await?;
    let todo: Todo = state::core_or_default(&storage, co_state.co(), "todo").await?;
    let tasks = todo.tasks
        .stream(&storage)
        .map_ok(|(_id, task)| task)
        .try_collect::<Vec<_>>()
        .await?;
    Ok((co.name, tasks))
});
let (name, tasks) = resource().transpose()?.unwrap_or_default();
```

### Handling Resource results

Do NOT use `.suspend()?` — it causes UI flickering because it triggers the
`SuspenseBoundary` fallback on every state change. Since CO state is local, loading
takes only 1-10ms, so a brief `None` is invisible to users.

Instead, call the resource as a function and use `.transpose()?` to propagate errors
while treating the loading state as `None`:

```rust
let resource = use_selector_state(&co, move |storage, co_state| async move {
    // ...
    Ok(data)
});
// None while loading, Some(data) when ready, propagates errors via ?
let data: Option<MyData> = resource().transpose()?;
```

This keeps the component mounted and rendering instead of unmounting into a suspense
fallback. The `Option` lets you distinguish loading from loaded-but-empty:

```rust
match data {
    None => rsx! { /* loading: show nothing, spinner, or skeleton */ },
    Some(items) if items.is_empty() => rsx! { "No items yet." },
    Some(items) => rsx! { for item in items { /* render */ } },
}
```

Do NOT use `.unwrap_or_default()` if the default would show misleading empty-state
UI (like "No items found") during the brief loading period. Keep it as `Option` and
render accordingly — show nothing or a skeleton while `None`, show the real empty
state only when `Some(empty_collection)`.

### use_selector (resolve from storage)

```rust
fn use_selector<F, Fut, T>(co: &Co, f: F) -> Resource<Result<T, CoError>>
where
    F: Fn(CoBlockStorage) -> Fut + Clone + 'static,
```

Selects derived state using only the CO's block storage. Use this in child components
when a parent has already provided a `Link<T>` or `Cid` to resolve. The closure
receives `CoBlockStorage` and re-executes when the CO state changes.

**Important:** Read any `ReadSignal` props *before* the `async move` block. Dioxus
tracks reactive subscriptions synchronously — if you read a signal only inside the
async block, Dioxus won't know this selector depends on that signal.

```rust
let item = use_selector(&co, move |storage| {
    // Read signal BEFORE async — this registers the reactive subscription
    let link = my_link.read();
    async move {
        Ok(storage.get_value(&*link).await?)
    }
})?;
```

### use_selectors / use_selector_states

```rust
fn use_selectors<F, Fut, T>(cos: &Cos, f: F) -> Resource<Result<T, CoError>>
fn use_selector_states<F, Fut, T>(cos: &Cos, f: F) -> Resource<Result<T, CoError>>
```

Multi-CO variants. The closure receives `Vec<CoSelector>` or `Vec<CoSelectorState>`,
each containing `co: CoId`, `storage: CoBlockStorage`, and optionally `state: CoReducerState`.

### use_co_reducer_state

```rust
fn use_co_reducer_state(co: &Co) -> Resource<Result<CoReducerState, CoError>>
```

Subscribes to the raw reducer state of a CO as a resource. Useful when you need the
`CoReducerState` directly rather than derived data.

### use_co_id

```rust
fn use_co_id(co: String) -> ReadSignal<CoId>
```

Converts a `String` CO identifier into a reactive `ReadSignal<CoId>`. Useful when
receiving CO IDs as component props.

### use_did_key_identity

```rust
fn use_did_key_identity(name: &str) -> Result<ReadSignal<Identity>, RenderError>
```

Gets or creates a `did:key:` identity with the given name. On first call, generates
a new key pair and stores it in the Local CO keystore. On subsequent calls, returns
the existing identity. Returns `Result` because it suspends until the identity is ready.

```rust
let identity = use_did_key_identity("my-app-identity")?;
// identity.read().did -> "did:key:z6Mk..."
```

### use_co_context

```rust
fn use_co_context() -> CoContext
```

Returns the `CoContext` from Dioxus context. Rarely needed directly since other hooks
use it internally. Only use as a last resort for advanced operations like
`context.join_unrelated_co()` or `context.contact()`. Simple apps should never need this.

## Data Flow Patterns

### Reading state

```
CoId -> use_co -> Co -> use_selector_state -> derived data -> rsx!
                    \-> use_selector (with Link<T> from parent) -> child data -> rsx!
```

Always read through selectors. Selectors re-run automatically when the CO changes.

### Writing state

```
User event -> use_callback -> co.dispatch(identity, core, action)
```

Actions flow into the CO's Log, the reducer produces new state, and all selectors
watching that CO automatically re-evaluate.

### Async writing with error handling

```rust
let action = use_action(move |(identity, action): (Identity, MyAction)| {
    let co = co.clone();
    async move {
        co.push(identity, "my-core", action).await
    }
});

// In a callback:
action.call((identity, MyAction::DoSomething));
```

`Co::push` is the async variant of `dispatch` that returns `Result<CoReducerState, CoError>`.

## Common Patterns

### App skeleton with error and suspense boundaries

```rust
#[component]
fn App() -> Element {
    rsx! {
        ErrorBoundary {
            handle_error: |errors: ErrorContext| rsx! {
                pre { "Error: {errors:#?}" }
            },
            SuspenseBoundary {
                fallback: |context: SuspenseContext| rsx! {
                    if let Some(placeholder) = context.suspense_placeholder() {
                        {placeholder}
                    } else {
                        "Loading..."
                    }
                },
                // App content here
                MainView {}
            }
        }
    }
}
```

### Embedding a Core WASM binary

```rust
const MY_CORE_NAME: &str = "my-core";
const MY_CORE_BINARY: &[u8] = include_bytes!(
    "../../my-core/target-wasm/wasm32-unknown-unknown/release/my_core.wasm"
);
```

### Creating a CO with a Core

```rust
local_co.create_co(
    identity,
    CreateCo::generate(name)
        .with_core_bytes(MY_CORE_NAME, "my-core-type", MY_CORE_BINARY),
);
```

### Joining an invited CO

```rust
local_co.dispatch(
    identity.clone(),
    CO_CORE_NAME_MEMBERSHIP,
    MembershipsAction::ChangeMembershipState {
        did: identity.did.clone(),
        id: co_id,
        membership_state: MembershipState::Join,
    },
);
```

### Inviting a participant

```rust
co.dispatch(
    identity,
    CO_CORE_NAME_CO,
    CoAction::ParticipantInvite {
        participant: did,
        tags: tags!("name": display_name),
    },
);
```

## Common Anti-Patterns

### Copying selector data into signals

```rust
// WRONG: creates stale copy
let items = use_signal(Vec::new);
let selector = use_selector_state(&co, ...);
// then manually syncing items from selector -> diverges, bugs

// CORRECT: use selector result directly in rsx!
let items_resource = use_selector_state(&co, move |storage, co_state| async move {
    // ...derive items here...
    Ok(items)
});
let items: Option<Vec<Item>> = items_resource().transpose()?;
rsx! {
    if let Some(items) = items {
        for item in items { /* ... */ }
    }
}
```

### Global state store

```rust
// WRONG: inventing a global state
static APP_STATE: GlobalSignal<AppState> = GlobalSignal::new(|| AppState::default());

// CORRECT: each component subscribes to the COs it needs
let co = use_co(co_id);
let data = use_selector_state(&co, |storage, state| async move { ... })?;
```

### Spawning for derived state

```rust
// WRONG: spawn to compute derived state
spawn(async move {
    let count = compute_count_from_co().await;
    count_signal.set(count);
});

// CORRECT: derive in selector
let count = use_selector_state(&co, move |storage, co_state| async move {
    // ...compute count here...
    Ok(count)
})?;
```

### Optimistic rendering

```rust
// WRONG: update UI before dispatch confirms
items_signal.push(new_item); // optimistic
co.dispatch(identity, "core", AddItem(new_item));

// CORRECT: just dispatch, the selector handles the rest
co.dispatch(identity, "core", AddItem(new_item));
// selector automatically re-evaluates and UI updates
```

### Using CoContext directly in components

```rust
// WRONG: reaching around hooks
let context = use_co_context();
context.try_with_application(|app| async move {
    // manually accessing Application...
}).await;

// CORRECT: use hooks
let co = use_co(co_id);
let data = use_selector_state(&co, |storage, state| async move { ... })?;
```

## Key Imports

```rust
// co-dioxus hooks and types
use co_dioxus::{
    use_co, use_cos, use_co_context, use_co_id,
    use_co_reducer_state, use_did_key_identity,
    use_selector, use_selector_state,
    use_selectors, use_selector_states,
    CoContext, CoSettings, CoError, CoBlockStorage,
    CoSelector, CoSelectorState,
};

// co-sdk state utilities
use co_sdk::{
    state, CoId, CreateCo, Did, Identity,
    tags, CO_ID_LOCAL, CO_CORE_NAME_CO, CO_CORE_NAME_MEMBERSHIP,
};

// For resolving links in selectors
use co_primitives::{BlockStorageExt, Link, OptionLink};

// Built-in Core actions
use co_core_co::CoAction;
use co_core_membership::{MembershipState, MembershipsAction};

// Dioxus
use dioxus::prelude::*;

// Stream utilities for selectors
use futures::TryStreamExt;
use std::future::ready;
```

## Dependency Setup

COKIT crates are NOT on crates.io. Add via git:

```sh
cargo add co-sdk co-dioxus co-core-membership co-core-co \
  --git https://github.com/1iolabs/cokit.git
```

Additional useful dependencies:
```sh
cargo add futures
cargo add uuid --features v7
```

## Advanced Topics

For advanced operations beyond the standard hook-based workflow, see
[references/advanced.md](references/advanced.md). This covers:
- `CoContext::join_unrelated_co` - joining COs not discovered through memberships
- `CoContext::contact` - sending contact requests to other DIDs
- `CoContext::ready` / `ready_blocking` - waiting for initialization
- Custom `CoSettings` configuration for special deployment scenarios

## For Deeper Reference

- Full todo app example: `https://gitlab.1io.com/1io/example-todo-list`
- co-dioxus source: `co-dioxus/src/` in the cokit repository
- COKIT domain concepts: see [cokit skill](../cokit/SKILL.md) and its references
