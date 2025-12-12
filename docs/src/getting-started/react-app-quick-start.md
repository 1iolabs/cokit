#todo: Document the actual code

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

In this new workspace, first run:
```sh
npm install
```

The following command should now start the bare-bones Tauri project:
```sh
npm run tauri dev
```

Tauri should open a new window that looks like this:

![App screenshot loading failed](../assets/tauri-app-scrrenshot.png)

***Important Folders:***

- `src` : This will contain all our frontend App components
- `src-tauri` : This contains all Rust-related code and is a Cargo package
- `public` : This is for resources that Vite might need to load at runtime. 
   - This is where the compiled `{core}.wasm` files should go.

### Application
Before we can write our app, we need to install some additional packages and tweak some configs.

1. Install required npm packages:

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

1. Delete all files from your `src` folder.

2. Copy all files from the `my-todo-app-tauri/src` folder of the [existing To-do List example repository](https://gitlab.1io.com/1io/example-todo-list.git) into your `src` folder.

3. Copy the WASM from your Core into the `public` folder as `my_todo_core.wasm`.

4. Run Tailwind:  
```sh
npx tailwindcss -i ./tailwind.css -o ./assets/tailwind.css
```

5. Build the frontend:  
```sh 
npm run build
```

6. Start the App: 
```sh
npm run tauri dev
```

7. (**Optional**) You can set the following environment variables when using the CO-kit Tauri plugin:  
   - `CO_NO_KEYCHAIN=true` : Set this to `true` if you don't want to save keys to your keychain. 
     - **NOTE**: While this can improve handling during development by skipping the pop-ups that ask for permission to save the keys, it is **highly unsafe** in production.
   - `CO_BASE_PATH={path}` : Change the path where the data is stored.
