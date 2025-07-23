# Next Steps

Now that you know the basics of working with cokit, here are some expamples of the cool things you can build with it:

### Messenger?

### Real-time counter with shared state
This example shows how a simple counter can be shared and synchronized across peers using CoKit:

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

