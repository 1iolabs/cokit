# CO

## Abstract
CO implementation using the rust progamming language.

## Usage

### Define a COre data structure

```rust
#[derive(CoreType)]
struct Todos {
    title: String,
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

impl Reducer<TodosAction> for Todos {
    type Action = TodosAction;

	fn reduce(mut self, event: &ReducerAction<Self::Action>, context: &mut dyn Context) -> Self {
        match event.action {
            TodosAction::Create { title } => {
                let id = self.next_todo_id;
                self.next_todo_id = self.next_todo + 1;
                self.todos.push(Todo { id, title, done: false })
            },
            TodosAction::SetDone { id, done } => {
                self.todos.update_one(
                    |todo| todo.id == id,
                    |toto| {
                        todo.done = done;
                    }
                );
            },
            TodosAction::Delete { id } => {
               self.todos.delete_one(|todo| todo.id == id); 
            },
            TodosAction::DeleteDone => {
               self.todos.delete_many(|todo| todo.done);
            },
        }
        self
    }
}
```

### Build an Application

```typescript
function Todos({co}) {
    // read
    const {title, todos} = useCoSelector(co, "todos", (storage, core_state) => {
        let state = await storage.read<Todos>(core_state);
        let todos = storage.entries<Todo>(state.todos).limit(100);
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
            <h2>{title}</h2>
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
- `rust-1.76` (MSRV)
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

### Utility

fmt:
```shell
cargo +nightly fmt --check
```

## Log

```shell
tail -f data/log/co.log | bunyan -c '!/^(libp2p|hickory_proto|dioxus_core|log|quinn|tower|tonic|h2|hyper|quinn_proto|tokio_util::codec::framed_impl)/.test(this.target)'
```
