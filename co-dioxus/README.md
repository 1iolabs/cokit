# CO Dioxus

High level components for using COKIT in dioxus applications.

## Usage

### Initialize the application

To initialize COKIT create a new `CoContext` and provide it to the app as context.

```rust
use co_dioxus::{CoContext, CoSettings};
use dioxus::prelude::*;

fn main() {
	// co
	let context = CoContext::new(co_dioxus::CoSettings::cli("my-todo-app"));
	
	// dioxus
	LaunchBuilder::desktop().with_context(context).launch(App);
}
```

### Select data from a CO

Most interactions with COKIT are done using hooks.

```rust
use co_dioxus::{use_co, use_did_key_identity, use_selector, CoContext};

#[component]
pub fn App() -> Element {
	// open the "local" CO.
	let local_co_id = use_signal(|| CoId::new(CO_ID_LOCAL));
	let local_co = use_co(local_co_id.into());
	
	// read memberships from the local co
	let memberships = use_selector(&local_co, move |storage, co_state| async move {
		Ok(state::memberships(storage, co_state.co())
			.try_filter(move |item| ready(item.0.as_str() != CO_ID_LOCAL))
			.try_collect::<Vec<_>>()
			.await?)
	}).suspend()?;
	
	// use a identity
	let identity = use_did_key_identity("my-identity")?;
	
	// render
	rsx! {
		"..."
	}
}
```

### Push actions to a CO

To change a CO just push actions into it.
This will cause all selectors to update once the action is applied.

```rust
use co_dioxus::{use_co, use_did_key_identity, use_selector, CoContext};

#[component]
pub fn MyComponent(co_id: ReadonlySignal<CoId>) -> Element {
	// use the co
	let co = use_co(co_id);
	
	// use a identity
	let identity = use_did_key_identity("my-identity")?;
	
	// render
	rsx! {
		button {
			onclick: {
				let identity = identity.clone();
				let co = co.clone();
				move || {
					co.dispatch(identity, "my-core", MyCoreAction::Increment(1));
				}
			},
		}
	}
}
```
