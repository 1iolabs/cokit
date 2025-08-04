# Next Steps

Now that you know the basics of working with CO-kit, here are some examples of the cool things you can build with it:

## Permissions
As an example we could change the todo list core to only allow todo task deletion for the creator of the todo task.
We need to store the creator of a task and in the delete just compare it.

Add creator to state: 
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

CO-kit then makes sure and verifies everyone got the same state.

## More examples
### Real-time counter with shared state
This example shows how a simple counter can be shared and synchronized across peers using CO-kit:

```js
import { useCo } from "co";

const Counter = () => {
  const [state, actions] = useCo("counter-uuid");

  return (
    <div>
      <p>Count: {state.value}</p>
      <button onClick={actions.increment}>+</button>
      <button onClick={actions.decrement}>-</button>
    </div>
  );
};
```
Here, `value`, `increment`, and `decrement` are defined in the CO schema. The state updates are CRDT-backed and instantly reflect across all connected users.

### Nested collaborative objects
This example showcases using multiple COs — such as a project list, where each project has its own shared object:

```js
const ProjectsDashboard = () => {
  const [projectList] = useCo("project-list");

  return (
    <div>
      {projectList.projects.map(({ coId, title }) => (
        <ProjectView key={coId} title={title} coId={coId} />
      ))}
    </div>
  );
};

const ProjectView = ({ coId, title }) => {
  const [state, actions] = useCo(coId);

  return (
    <section>
      <h3>{title}</h3>
      <ul>
        {state.tasks.map(t => <li>{t.name}</li>)}
      </ul>
    </section>
  );
};
```
Each project lives as a **standalone CO**, making the structure scalable and naturally modular.

### Schema-based form editing
Here we bind a form to a CO that holds user profile data. Changes propagate live, but validation logic is handled by the schema compiled to WASM:

```js
const ProfileForm = () => {
  const [state, actions] = useCo("user-profile");

  return (
    <form>
      <label>
        Name:
        <input
          value={state.name}
          onChange={e => actions.setName(e.target.value)}
        />
      </label>
      <label>
        Email:
        <input
          value={state.email}
          onChange={e => actions.setEmail(e.target.value)}
        />
      </label>
    </form>
  );
};
```
The schema ensures the email format is correct, and optional constraints like uniqueness or required fields can be enforced at runtime through WASM-based validation.

### Peer-to-Peer Messaging Application
One obvious, cool thing that you can use CO-kit for is building a messaging application. We have already built a demo for such a use case that you can check out here: https://gitlab.1io.com/1io/co-sdk/-/tree/tauri-messenger-demo/tauri-plugin-co-sdk/examples/messenger

