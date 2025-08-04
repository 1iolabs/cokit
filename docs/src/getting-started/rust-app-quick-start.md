# App Quick Start
As a very simple UI building tool, we use [dioxus](https://dioxuslabs.com/) for this quick start tutorial.
We will use TailwindCSS for styling the application.

## Requirements
- `dioxus-0.6`
- `npm`

## Setup
Here we install dioxus and setup the empty application crate.

### Setup Dioxus
Install the precompiled `dx` tool:
```shell
cargo binstall dioxus-cli
```

You can also head over to dioxus for further instructions: https://dioxuslabs.com/learn/0.6/getting_started/#install-the-dioxus-cli.

### Setup NodeJS
Head over to: https://nodejs.org/en/download
We need it to use TailwindCSS within our app.

### Application
We need to setup a new rust crate for the application:
1. Initialize dioxus application:
```sh
dx new my-todo-app --subtemplate Bare-Bones -o is_fullstack=false -o is_router=false -o default_platform=desktop -o is_tailwind=true
```
2. Install `co-sdk` and `co-dioxus` which is the dioxus integration as a dependencies:
```sh
cargo add co-sdk co-dioxus
```
3. Install our core as dependency:
```shell
cargo add ../my-todo-core
```
4. Setup tailwind
```sh
npm init -y
npm install -D tailwindcss @tailwindcss/cli daisyui
```

## Implementation
#todo

#### 

#### Tailwind
`tailwind.css`:
```css
@import "tailwindcss";
@source "./src/**/*.{rs,html,css}";
@plugin "daisyui";
```

## Description
- **`useCo(...)`** connects the component to a shared CO using its UUID. It returns:

    - `state`: the current reactive state of the object.

    - `actions`: a set of functions to mutate the state collaboratively.

- **`state.items.map(...)`** iterates over shared items stored in the CO (e.g., a shopping list).

- **`actions.markAsDone(...)`** is triggered when a list item is clicked, marking the item as completed across all peers.

- The component will automatically re-render when the shared state changes, enabling real-time collaboration.
