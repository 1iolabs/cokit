#todo: Document the actual code

# React App Quick Start

In this tutorial we are using React with Tailwind and DaisyUI to build a Todo App in Typescript that is using CoKit in the background.
We use [Tauri](https://tauri.app/) to build a Desktop app that runs using the OS specific browser.

## Table of Contents

<!-- toc -->

## Requirements

- `tauri-2.7`
- `npm`

## Setup

### Setup tauri

At the location where the tauri project should be created, run `npm create tauri-app@latest`. The install script will ask for extra information. We used the following:
- Project name: `my-todo-app-tauri` (This will affect the name of the workspace folder, the name of the npm package and the name of the cargo package)
- [Identifier](https://tauri.app/reference/config/#identifier): `com.1io.examples.todo`
- Frontend language: `Typescript / Javascript`
- Package manager: `npm`
- UI Template: `React`
- UI Flavor: `Typescript`

This created a new tauri workspace at your location using React, Typescript and vite.
That new directory is an npm package and the workspace for your new App. You can now open it in you IDE of choice.

In the new workspace, first run `npm install`, then the command `npm run tauri dev` should start the bare bones tauri project.

You should see that tauri opened a new window that looks like this:

![App screenshot loading failed](../assets/tauri-app-scrrenshot.png)

***Important Folders:***

The `src` folder later contains all our frontend App components.

The `src-tauri` folder contains all rust related code and is a cargo package.

The `public` folder is for resources that vite might need to load at runtime. This is where the compiled `{core}.wasm` files should go.

### Setup NodeJS
We need NodeJS to use TailwindCSS within our app.  
Head over to [NodeJS](https://nodejs.org/en/download) for download instructions.

### Application

Before we can write our app, we need to install some additional packages and tweak some configs.

1. Install needed npm packages:

``` sh
npm i @1io/compare @1io/tauri-plugin-co-sdk-api co-js multiformats react-error-boundary uuid web-streams-polyfill
npm i -D @tailwindcss/cli @tailwindcss/vite @types/node daisyui tailwindcss vite-plugin-wasm
```

2. Init Tailwind by adding file `tailwind.css` to the root:

``` css
@import "tailwindcss";
@source "./src/**/*.{rs,html,css}";
@plugin "daisyui";
```

3. Edit vite config file `vite.config.ts` to work with tailwind and wasm:

``` Typescript
// add these imports
import tailwindcss from "@tailwindcss/vite";
import wasm from "vite-plugin-wasm";

export default defineConfig(() => ({
	plugins: [react(), tailwindcss(), wasm()], // <--- Add these plugins
	// ... leave rest
	}));
```

4. Add Tauri plugin:

``` sh
cd src-tauri
cargo add tauri-plugin-co-sdk --git https://gitlab.1io.com/1io/co-sdk.git --branch main
```

5. Our tauri plugin uses async code, so we also need an async runtime. We use tokio:

```sh
cargo add tokio@1.48
```

6. Edit `src-tauri/src/lib.rs` so it looks like this:

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

7. Turn main function in `src-tauri/src/main.rs` async:

```rust
// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[tokio::main]
async fn main() {
	my_todo_app_tauri_lib::run().await;
}
```

8. Tauri has a permission system. We need to add the `tauri-plugin-co-sdk` permissions to `src-tauri/capabilities/default.json`.
Add `co-sdk:default` to the `permissions` field:

``` json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Capability for the main window",
  "windows": ["main"],
  "permissions": ["core:default", "co-sdk:default", "opener:default"]
}
```

9. (Optional) There are a lot of possible settings in the `src-tauri/tauri.conf.json` config file. You can define the starting conditions of the app under `app.windows`: 

- Adjust `title`, `width` and `height` to your hearts content.
- If you are a developer, you may want the app to open with devtools opened: Add `"devtools": true`.

## Implementation

1. Delete all files from `src` folder.

2. Copy all files from [Existing todo example repository](https://gitlab.1io.com/1io/example-todo-list.git) `my-todo-app-tauri/src` folder into this `src` folder.

3. Copy the wasm from your core into the `public` folder as `my_todo_core.wasm`.

4. Run tailwind: `npx tailwindcss -i ./tailwind.css -o ./assets/tailwind.css --watch`

5. Build frontend: `npm run build`

6. Start App: `npm run tauri dev`

7. (Optional) The tauri plugin checks environment variables. You can set these to change behaviour:
- Set `CO_NO_KEYCHAIN=true` if you don't want to save the keys to your keychain. This improves handling in dev mode as it skips the pop ups to ask for permission to save but is highly unsafe in production.
- Set `CO_BASE_PATH={path}` to change the path where the data is stored.
