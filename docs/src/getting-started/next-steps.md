# Next Steps

Now that you know the basics of working with CO-kit, we introduce a few more concepts and show you some examples of the cool things you can build with it.

## Introducing CO
A [CO](../reference/co.md) is a virtual room for collaboration.
It organizes a set of Cores and allows participants to access and modify them.

## Introducing Core
A [Core](../reference/core.md) is a data model used within an [CO](../reference/co.md).
It models data, business logic and permissions.

## Permissions
A Core contains [permissions](../reference/permissions.md) as logic in the data model.
As an example we change the [todo list core](../getting-started/rust-core-quick-start.md) to only allow todo task deletion for the creator of the todo task.
We need to store the creator of a task and compare it in the `TaskDelete` against each other.

Add `creator` to state: 
```rust
#[co]
pub struct TodoTask {
	pub id: String,
	pub title: String,
	pub done: bool,
	pub creator: Did,
}
```

Check if our condition is fulfilled:
```rust
TodoAction::TaskDelete { id } => {
	let task = tasks.get(id).await?.ok_or(anyhow!("Task not found"))?;
	if event.from != task.creator {
		return Err(anyhow("Only the creator is allowed to delete tasks"));
	}
	tasks.remove(id).await?;
},
```

CO-kit then verifies if everyone got the same state.

## More examples
### Real-time counter
This example shows how a simple counter can be shared and synchronized across peers using CO-kit:

```js
import { useCo, useSelector } from "co";

const Counter = () => {
	const co = useCo("co-uuid");
	const count = useSelector(
		co,
		"counter",
		(_storage, counter_state) => counter_state.counter
	);
	return (
	<div>
		<p>Count: {count}</p>
		<button onClick={() => co.dispatch("counter", {increment: 1})}>+</button>
		<button onClick={() => co.dispatch("counter", {decrement: 1})}>-</button>
	</div>
	);
};
```
Here, `count`, `increment`, and `decrement` are defined in the Core. The state updates are [CRDT-backed](../glossary/glossary.md##CRDT) and instantly reflect across all connected users.

### Nested COs
This example showcases using multiple [COs](../reference/co.md) — e.g. in a project list, where each project has its own [CO](../reference/co.md):

```js
import { useCo, useSelector } from "co";

const ProjectsDashboard = () => {
	const co = useCo("project-list");
	const projects = useSelector(
		co,
		"projects",
		(_storage, state) => state.projects
	);
	return (
	    <div>
			{projects.map(({ coId, title }) => (
				<ProjectView key={coId} title={title} coId={coId} />
			))}
	    </div>
	);
};

const ProjectView = ({ coId, title }) => {
	const co = useCo(coId);
	const todos = useSelector(
		co,
		"todo",
		(_storage, state) => state.todos
	);
	return (
		<section>
			<h3>{title}</h3>
			<ul>
				{todos.map((todo) => <li>{todo.name}</li>)}
			</ul>
		</section>
	);
};
```
Each project lives as a standalone [CO](../reference/co.md), making the structure scalable and naturally modular.

### Schema-based form editing
Here we bind a form to a [CO](../reference/co.md) that holds user profile data. Changes propagate live, but validation logic is handled by the [Core](../reference/core.md) (data model compiled to WASM):

```js
const ProfileForm = () => {
	const co = useCo(coId);
	const state = useSelector(
		co,
		"user-profile",
		(_storage, state) => {name: state.name, email: state.email},
	);
	return (
		<form>
			<label>
				Name:
				<input
					value={state.name}
					onChange={(e) => co.dispatch("user-profile", {setName: e.target.value}})
				/>
			</label>
			<label>
				Email:
				<input
					value={state.email}
					onChange={(e) => co.dispatch("user-profile", {setEmail: e.target.value}})
				/>
			</label>
		</form>
	);
};
```
The [Core](../reference/core.md) ensures the email format is correct, and optional constraints like uniqueness or required fields can be enforced at runtime through WASM-based validation.

### Peer-to-Peer Messaging Application
One obvious, cool thing that you can use CO-kit for is building a messaging application. We have already built a demo for such a use case that you can check out here: [Gitlab](https://gitlab.1io.com/1io/co-sdk/-/tree/tauri-messenger-demo/tauri-plugin-co-sdk/examples/messenger)
