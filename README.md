# CO

## Abstract
CO implementation using the rust progamming language.

## Usage

### Define a COre data structure

```rust
#[derive(CoreType)]
struct Todos {
    next_todo_id: u64,
    todos: CoreVec<Todo>,
}

#[derive(CoreType)]
struct Todo {
    id: u64,
    title: String,
    done: bool,
}

#[derive(CoreAction)]
enum TodosAction {
    /// Create TODO.
    Create { title: String },

    /// Set done state of a TODO.
    SetDone { id: u64, done: bool },
    
    /// Delete one TODO.
    Delete { id: u64 },

    /// Delete all TODOs which are done.
    DeleteDone,
}

impl CoreReducer<TodosAction> for Todos {
    type Action = TodosAction;

	fn reduce(mut self, event: &ReducerAction<Self::Action>, context: &mut dyn Context) -> Self {
        match event.action {
            TodosAction::Create { title } => {
                let id = self.next_todo_id;
                self.next_todo_id = self.next_todo + 1;
                self.todos.push(context, Todo { id, title, done: false })
            },
            TodosAction::SetDone { id, done } => {
                self.todos.update_one(
                    context,
                    |todo| todo.id == id,
                    |toto| {
                        todo.done = done;
                    }
                );
            },
            TodosAction::Delete { id } => {
               self.todos.delete_one(context, |todo| todo.id == id);
            },
            TodosAction::DeleteDone => {
               self.todos.delete_many(context, |todo| todo.done);
            },
        }
        self
    }
}
```

#### Possible Rust with API style
```rust
impl Todos {
    #[reducer]
    fn create(&mut self, title: String, context: &mut dyn Context) {
        let id = self.next_todo_id;
        self.next_todo_id = self.next_todo + 1;
        self.todos.push(context, Todo { id, title, done: false })
    }
    
    #[reducer]
    fn set_done(&mut self, id: u64, done: bool, context: &mut dyn Context) {
        self.todos.update_one(
            context,
            |todo| todo.id == id,
            |toto| {
                todo.done = done;
            }
        );
    }

    #[reducer]
    fn delete(&mut self,  id: u64, context: &mut dyn Context) {
        self.todos.delete_one(context, |todo| todo.id == id);
    },
    
    #[reducer]
    fn delete_done(&mut self, context: &mut dyn Context) {
        self.todos.delete_many(context, |todo| todo.done);
    }
}
```

#### Possible Assembly Script

#### 1

```typescript
@state
interface Todos {
    next_todo_id: number;
    todos: CoreVec<Todo>;
}

interface Todo {
    id: number;
    title: String;
    done: bool;
}

@action
function create(state: Todos, title: String) {
    let id = state.next_todo_id;
    state.next_todo_id = state.next_todo + 1;
    state.todos.push({id, title, done: false});
}

@action
function set_done(state: Todos, done: bool) {
    const id = state.next_todo_id;
    state.next_todo_id = state.next_todo + 1;
    state.todos.update_one(
        (todo) => todo.id == id,
        (toto) => {
            todo.done = done;
        }
    );
}
```

#### 2

Schema:

```typescript
import { CoList, Co } from "co/core";

export interface ShoppingListItem {
  id: string;
  title: string;
  done: boolean;
}

export interface ShoppingList extends Co {
  items: CoList<ShoppingListItem>;
}
```

Reducer:

```typescript
import { defineReducer } from "co/core";
import { ShoppingList } from "./schema";

export const actions = {
  addItem: defineReducer((state: ShoppingList, { id, title }) => {
    state.items.push({ id, title, done: false });
  }),
  markAsDone: defineReducer((state: ShoppingList, { id }) => {
    state.items.updateOne(
      (item) => item.id == id,
      (item) => item.done = true,
    );
  })
}
```

### Build an Application

```typescript
function Todos({co}) {
    // read
    const {title, todos} = useCoSelector(co, "todos", (storage, core) => {
        let state = await storage.get<Todos>(core);
        let todos = storage.entries<Todo>(state.todos);
        return {title: state.title, todos }
    });

    // write
    const api = useCoApi<Todos>();
    const [create, setCreate] = useState("");
    const onCreate = useCallback(
        () => {
            api.dispatch(TodosAction.Create { title: create });
            setCreate("");
        },
        [api, setCreate]
    );

    // render
    return (
        <section>
            <h2>TODOs: {title}</h2>
            {todos.map(todo => (
                <Todo key={todo.id} todo={todo} />
            ))}
            <form>
                <input name="title" value={create} onchange={title => setCreate(title)} />
                <button type="submit" onclick={onCreate}>Add</button>
            </form>
        </section>
    );
}
```

## Development

### Setup

Dependencies:
- `rust-1.82` (MSRV)
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

### Rust

#### Features (MSRV)
- `1.82`: https://blog.rust-lang.org/2024/10/17/Rust-1.82.0.html#precise-capturing-use-syntax

### Utility

fmt:
```shell
cargo +nightly fmt --check
```

## Log

```shell
tail -f data/log/co.log | bunyan -c '!/^(libp2p|hickory_proto|dioxus_core|log|quinn|tower|tonic|h2|hyper|quinn_proto|tokio_util::codec::framed_impl)/.test(this.target)'
```
