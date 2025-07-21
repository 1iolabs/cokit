# Minimal Coding Example

```
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

And you've created a cool list! Now what's next?