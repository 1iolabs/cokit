# App Quick Start

As a very simple UI building tool, we use dioxus for this quick start tutorial.

## Requirements
- `dioxus-0.6`

## Setup

### Dioxus
Install the precompiled `dx` tool:
```shell
cargo binstall dioxus-cli
```

### Application
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

## Implementation
#todo

