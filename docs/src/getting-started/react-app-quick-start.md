# React App Quick Start

In this tutorial we will build our To-do List App in Typescript, using React with Tailwind and DaisyUI, alongside CO-kit and our [To-do List Core](rust-core-quick-start.md).  
We will use [Tauri](https://tauri.app/) to create a Desktop app that runs using the OS-specific browser.

## Table of Contents

<!-- toc -->

## Requirements

- `tauri-2.7`
- `npm`

## Setup

### Setup NodeJS
We need NodeJS in order to use TailwindCSS within our app.   
Head over to [NodeJS](https://nodejs.org/en/download) for download instructions.

### Setup Tauri

Run the following at the location where the Tauri project should be created:
```sh
npm create tauri-app@latest
```  
The install script will ask for extra information. 

We used the following:
- Project name: `my-todo-app-tauri` 
  - **NOTE**: This will affect the name of the workspace folder, the name of the npm package, and the name of the Cargo package
- [Identifier](https://tauri.app/reference/config/#identifier): `com.1io.examples.todo`
- Frontend language: `Typescript / Javascript`
- Package manager: `npm`
- UI Template: `React`
- UI Flavor: `Typescript`

This creates a new Tauri workspace at this location using React, Typescript and Vite.  
The new directory is both an npm package and the workspace for your new App.  
You can now open it in you IDE of choice.

Browse to this new workspace, and run `npm install`:
```sh
cd my-todo-app-tauri
npm install
```

The following command should now start the bare-bones Tauri project:
```sh
npm run tauri dev
```

Tauri should open a new window that looks like this:

![App screenshot loading failed](../assets/tauri-app-scrrenshot.png)

You can now quit this app, and continue below.

***Important Folders:***

- `src` : This will contain all our frontend App components
- `src-tauri` : This contains all Rust-related code and is a Cargo package
- `public` : This is for resources that Vite might need to load at runtime. 
   - This is where the compiled `{core}.wasm` files should go.

### Application
Before we can write our app, we need to install some additional packages and tweak some configs.

1. Install the required npm packages:

```sh
npm i @1io/compare @1io/tauri-plugin-co-sdk-api co-js multiformats react-error-boundary uuid web-streams-polyfill
npm i -D @tailwindcss/cli @tailwindcss/vite @types/node daisyui tailwindcss vite-plugin-wasm
```

2. Initialize Tailwind by adding a `tailwind.css` file to the root:

```css
@import "tailwindcss";
@source "./src/**/*.{rs,html,css}";
@plugin "daisyui";
```

3. Edit the Vite config file (`vite.config.ts`) to work with Tailwind and WASM:

```Typescript
// add these imports
import tailwindcss from "@tailwindcss/vite";
import wasm from "vite-plugin-wasm";

export default defineConfig(() => ({
	plugins: [react(), tailwindcss(), wasm()], // <--- Add these plugins
	// ... leave the rest
	}));
```

4. Add the CO-kit Tauri plugin:

```sh
cd src-tauri
cargo add tauri-plugin-co-sdk --git https://gitlab.1io.com/1io/co-sdk.git --branch main
```

5. The CO-kit Tauri plugin uses async code, so we also need an async runtime.  
We can use Tokio for this:

```sh
cargo add tokio@1.48
```

6. Edit `src-tauri/src/lib.rs` so that it looks like this:

```rust
// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use tauri_plugin_co_sdk::library::co_application::CoApplicationSettings;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub async fn run() {
	tauri::async_runtime::set(tokio::runtime::Handle::current());
	let co_settings = CoApplicationSettings::cli("todo-example-tauri");

	tauri::Builder::default()
		.plugin(tauri_plugin_opener::init())
		.plugin(tauri_plugin_co_sdk::init(co_settings).await)
		.run(tauri::generate_context!())
		.expect("error while running tauri application");
}
```

7. Change the main function in `src-tauri/src/main.rs` to be async:

```rust
// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[tokio::main]
async fn main() {
	my_todo_app_tauri_lib::run().await;
}
```

8. Add `co-sdk:default` to the `permissions` field.
   - Tauri has a permissions system. We need to add the `tauri-plugin-co-sdk` permissions to `src-tauri/capabilities/default.json`.  

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Capability for the main window",
  "windows": ["main"],
  "permissions": ["core:default", "co-sdk:default", "opener:default"]
}
```

9. (**Optional**) There are a lot of possible settings in the `src-tauri/tauri.conf.json` config file. You can define the starting conditions of the app under `app.windows`: 
   - Adjust `title`, `width` and `height` to your heart's content.
   - Add `"devtools": true` if you are a developer, and you want the app to start with devtools opened.  

## Implementation

This example App is the same as in [the rust App example](rust-app-quick-start.md#implementation) but using react instead of dioxus.

We use the [MyTodoCore](rust-core-quick-start.md).

Upon first starting the application, a `did:key:` identity is created locally.  
We name it `my-todo-identity`.

The first view is where we create to-do lists, and respond to invites.  
The second view is where we manage tasks and participants.

### Application
Instead of the single file approach we use in the [rust app](rust-app-quick-start.md) we split the applications into extra files for each components. We also create an extra folder for the types like in a classic react app.

```admonish info
You can delete all generated files from the `src` folder except `vite-env.d.ts`.
```

#### Setup
There is no need to inittialize CO-kit here because the tauri plugin does that for us. Instead we just need to write code for the frontend.

##### Main
We start with the `main.tsx`:

```typescript
import React from "react";
import ReactDOM from "react-dom/client";
import { App } from "./components/app";
import "../tailwind.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <div className="bg-base-300 w-screen h-screen">
      <App />
    </div>
  </React.StrictMode>,
);
```

##### Types
Add a `types` folder.

In that folder we add a file `todo.ts` that contains all the types we need from our Todo Core:

```typescript 
import { CID } from "multiformats";

export type TodoTask = {
  id: string;
  title: string;
  done: boolean;
};

export type TodoCoreState = {
  // CoMap
  tasks: CID;
};

export type TodoAction =
  | { TaskCreate: TodoTask }
  | { TaskDone: { id: string } }
  | { TaskUndone: { id: string } }
  | { TaskSetTitle: { id: string; title: string } }
  | { TaskDelete: { id: string } }
  | "DeleteAllDoneTasks";
```

Next is the file `consts.ts` that contains all const variables like the core name or identity name:

```typescript
import { fetchBinary } from "@1io/tauri-plugin-co-sdk-api";

export const TODO_CORE_NAME: string = "todo";
export const TODO_IDENTITY_NAME: string = "my-todo-identity";

export async function fetchTodoCoreBinary(): Promise<
  ReadableStream<Uint8Array>
> {
  return await fetchBinary("my_todo_core.wasm");
}
```

Because we cannot just include the Core WASM bytes like in rust, we need to fetch it. This only works if the WASM file is in the `public` folder, i.e. included in the vite environment.

Add an `index.ts` file for exports:

```typescript
export * from "./todo";
export * from "./consts";
```

##### Components
Add a `components` folder under `src`.

Now we add an `app.tsx` file to that folder. The App component handles whether the Overview or a specific Todo list should be shown:

```typescript
import { ErrorBoundary } from "react-error-boundary";
import React from "react";
import { TodoList } from "./todo-list";
import { TodoOverview } from "./todo-overview";
import { CoId } from "@1io/tauri-plugin-co-sdk-api";
import "web-streams-polyfill/polyfill";

function Fallback(props: { error: unknown }) {
  return <pre className="p-4">Oops, we encountered an error: {String(props.error)}</pre>;
}

export function App() {
  const [activeCo, setActiveCo] = React.useState<CoId | undefined>();
  const onBack = React.useCallback(() => setActiveCo(undefined), []);
  const onOpen = React.useCallback((coId: CoId) => setActiveCo(coId), []);
  return (
    <ErrorBoundary FallbackComponent={Fallback}>
      {activeCo !== undefined ? <TodoList onBack={onBack} coId={activeCo} /> : <TodoOverview onOpen={onOpen} />}
    </ErrorBoundary>
  );
}
```

The `import "web-streams-polyfill/polyfill"` import is needed in this root file so all functions from the WASM wrappers (at the moment only CoMap but also CoList and CoSet in the future) work on native safari browsers. Tauri opens a webview using the native browser which under MacOS is Safari where unfortunately some features aren't implemented. Therefore we need the polyfill. 

#### Overview
Next, we want to display a list of Todo Lists and possible invites.

##### Memberships/invites
We use the tauri hooks to fetch the membership state in the `todo-overview.tsx` file:

```typescript
~import { useCallback } from "react";
~import { NavBar } from "./nav-bar";
~import { TodoListCreate } from "./todo-list-create";
~import { TodoListElement } from "./todo-list-element";
~import { TODO_IDENTITY_NAME } from "../types/consts";
~import {
~  createCo,
~  pushAction,
~  CoId,
~  Memberships,
~  MembershipsAction,
~  MembershipState,
~  CO_CORE_NAME_MEMBERSHIP,
~  useCo,
~  useCoCore,
~  useCoSession,
~  useDidKeyIdentity,
~  useResolveCid,
~} from "@1io/tauri-plugin-co-sdk-api";
~import { TodoListJoin } from "./todo-list-join";
~
~export type TodoOverviewProps = {
~  onOpen: (coId: string) => void;
~};
~
export function TodoOverview(props: TodoOverviewProps) {
  const localCoSession = useCoSession("local");
  const [localCoCid] = useCo("local");
  const membershipCoreCid = useCoCore(localCoCid, "membership", localCoSession);
  let memberships = useResolveCid<Memberships>(membershipCoreCid, localCoSession)?.memberships;
  const identity = useDidKeyIdentity(TODO_IDENTITY_NAME);

~  // TODO can probably do this better
~  // memberships can be undefined if there is no state yet but we want an emnpty array in that case
~  if (membershipCoreCid === null) {
~    memberships = [];
~  }
  const onCreateCo = useCallback(
    async (name: string) => {
      if (identity !== undefined) {
        await createCo(identity, name, false);
      }
    },
    [identity],
  );

  const onJoin = useCallback(
    async (coId: CoId) => {
      if (identity !== undefined && localCoSession !== undefined) {
        const action: MembershipsAction = {
          ChangeMembershipState: { did: identity, id: coId, membership_state: MembershipState.Join },
        };
        await pushAction(localCoSession, CO_CORE_NAME_MEMBERSHIP, action, identity);
      }
    },
    [identity],
  );

  // render
~  return (
~    <div className="flex flex-col h-full">
~      <NavBar left={null} center={<>Todo App</>} right={null} />
~      <div className="grow shrink flex flex-col p-4 gap-4">
~        <div className="grow shrink overflow-y-auto bg-base-100 border-base-300 shadow-sm collapse border">
~          {memberships !== undefined ? (
~            <TodoListCreate showInitially={memberships.length === 0} onCreateCo={onCreateCo} />
~          ) : null}
~          <ul className="list min-h-0">
~            {memberships?.map((membership) => {
~              if (
~                membership.membership_state === MembershipState.Invite ||
~                membership.membership_state === MembershipState.Join
~              ) {
~                return (
~                  <TodoListJoin
~                    key={membership.id}
~                    coId={membership.id}
~                    pending={membership.membership_state !== MembershipState.Invite}
~                    onJoin={onJoin}
~                  />
~                );
~              }
~              if (membership.membership_state === MembershipState.Active) {
~                return <TodoListElement key={membership.id} coId={membership.id} onOpen={props.onOpen} />;
~              }
~              return null;
~            })}
~          </ul>
~        </div>
~
~        <div className="flex-none card bg-base-100 shadow-sm p-2">Your identity: {identity}</div>
~      </div>
~    </div>
~  );
}
```

We open a new session on the local CO. This causes fetched and pushed data to retain in the memory while the session is open.
The `useCoSession` hook opens a session the first time it is called and returns the same session afterwards. It automatically closes the session if the component unmounts. Many other hooks need this session ID to function properly.


The `useCo` hook returns `[stateCid, heads]` of a given CO.
In this case we only take interest in the state. This is a Cid and we can use it with the `useCoCore` hook. It takes the CO state Cid, a core name and CO session to fetch the Core state Cid.

The Cid can be resolved using the `useResolveCid` hook. The returned object is of the type `Memberships` which contains information about all the COs we can interact with. Depending on this state we render different [list items](#list-items).

##### Identity
To push an action or create a CO we need our identity. We use the `useDidKeyIdentity` hook for that. It takes an identity name and creates a new `did:key` identity if none were found. It then returns the Did in string form.

##### Creating a CO
We can simply use the `createCo` function to create a CO. Creating a CO is a bit special so there is a specific command for it.
We need our identity, a name for the CO and whether it's a public CO. In our example we only create private COs.

##### Joining a CO
We have a handler for joining a CO that we set as prop for the `TodoListJoin` Element. We call the `pushAction` function using the session string, our identity the core name we get as a constant from the `@1io/tauri-plugin-co-sdk-api` and an action. This will then push the given action to the specified Core.

The `ChangeMembershipState` action comes from CO-kit and we have ts types for it in the `@1io/tauri-plugin-co-sdk-api` package. In our case we want to set our membership status from `Invite` to `Join`.

##### List items
The possible membership states that are of interest to us are:
- Active: Normal active membership
- Invite: We were invited to join a [CO](../reference/co.md) by someone else
- Join: We accepted an invite and are waiting for it to complete

If state is Invite or Join, we show a list element that either has a Join button or is marked as pending.

If state is Active, we render a list item that shows the number of unone tasks. We set a prop that contains the CO id that we get from the membership state.

`todo-list-element.ts`:

```typescript
~import { useMemo } from "react";
~import { TodoCoreState, TodoTask } from "../types";
~import { TODO_CORE_NAME } from "../types/consts";
~import { CoMap } from "co-js";
~import {
~  CoId,
~  useCo,
~  useCoSession,
~  useResolveCid,
~  Co,
~  useCoCore,
~  useBlockStorage,
~  useCollectCoMap,
~} from "@1io/tauri-plugin-co-sdk-api";
~
~export type TodoListElementProps = {
~  coId: CoId;
~  onOpen: (coId: CoId) => void;
~};
~
export function TodoListElement(props: TodoListElementProps) {
  const [coCid] = useCo(props.coId);
  const coSession = useCoSession(props.coId);
  const coState = useResolveCid<Co>(coCid, coSession);
  const todoCoreCid = useCoCore(coCid, TODO_CORE_NAME, coSession);
  const todoState = useResolveCid<TodoCoreState>(todoCoreCid, coSession);

  const storage = useBlockStorage(coSession);
  const taskMap = useMemo(() => {
    if (todoState?.tasks !== undefined) {
      return new CoMap(todoState.tasks.bytes);
    }
    return undefined;
  }, [todoState?.tasks]);
  const tasks = useCollectCoMap<TodoTask>(taskMap, storage);

  const undoneCount = useMemo(() => {
    let count = 0;
    for (const t of tasks.values()) {
      if (!t.done) {
        count++;
      }
    }
    return count;
  }, [tasks]);

  // render
~  return (
~    <li
~      className="list-row flex hover:bg-base-300 rounded-none cursor-pointer after:opacity-5"
~      onClick={() => props.onOpen(props.coId)}
~    >
~      <span className="font-bold flex-1">
~        {coState?.n ?? <span className="loading loading-spinner loading-xs"></span>}
~      </span>
~      {undoneCount > 0 && <div className="badge badge-soft badge-secondary">{undoneCount}</div>}
~    </li>
~  );
}

```

### Build the App
These commands will build your app:

```sh
npx tailwindcss -i ./tailwind.css -o ./assets/tailwind.css
npm run build
```

### Start the App
Start your app with:
 
```sh
npm run tauri dev
```

(**Optional**) You can set the following environment variables when using the CO-kit Tauri plugin:  
- `CO_NO_KEYCHAIN=true` : Set this to `true` if you don't want to save keys to your keychain. 
  - **NOTE**: While this can improve handling during development by skipping the pop-ups that ask for permission to save the keys, it is **highly unsafe** in production.
- `CO_BASE_PATH={path}` : Change the path where the data is stored.


## Full example

To have a better structural overview we split the code so that each component lives in its own file.
As copying the code into this documentation would unnecessarily bloat it, we instead link to a repository where all the code can be viewed.
You can find the full examples as a git project here:
- [1io / example-todo-list - GitLab](https://gitlab.1io.com/1io/example-todo-list.git)

If you followed the [earlier steps](#setup) to create your example app workspace, these last steps will complete your app:

1. Delete all files from your `src` folder.

2. Copy all files from the `src` folder of the [existing react Todo List example repository](https://gitlab.1io.com/1io/example-todo-list/-/tree/main/my-todo-app-tauri) into your `src` folder.

3. Copy the WASM from your Core into the `public` folder as `my_todo_core.wasm`.

Otherwise you can instead just copy the complete `my-todo-app-tauri` folder from the repository.
