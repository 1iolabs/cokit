#todo #tech

The example below demonstrates how to connect a React component to a Collaborative Object (CO) using the `useCo` hook provided by the CO-kit SDK:

```rust
import { useCo } from "co";

const ShoppingList = () => {
  const [state, actions] = useCo(
    "3c085622-a175-4357-ace9-c59443404794"
  );

  return (
    <List>
      {state.items.map(({ item }) => (
        <ListItem onClick={actions.markAsDone({ id: item.id })}>
          {todo.title}
        </ListItem>
      ))}
    </List>
  );
}
```
- `useCo(...)` connects the component to a shared CO using its UUID. It returns:

    - `state`: the current reactive state of the object.

    - `actions`: a set of functions to mutate the state collaboratively.

- `state.items.map(...)` iterates over shared items stored in the CO (e.g., a shopping list).

- `actions.markAsDone(...)` is triggered when a list item is clicked, marking the item as completed across all peers.

- The component will automatically re-render when the shared state changes, enabling real-time collaboration.

And you've created a cool list! Now what's next?
