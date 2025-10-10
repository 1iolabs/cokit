# CO-kit
A software development kit to build decentralized applications with, written in Rust.

## CO-kit for ...
#todo
### Frontend developers
#todo

### Backend developers
#todo

### Database nerds
#todo
- Atomicity: Core
- Consistency: Core
- Isolation: Core
- Durability: Reproducibly anytime with Cores.
- Concurrency Control
- Oplog
- Like a Master-Master replication where your logic can decide what happens on conflicts whereas with transactions only the database can

## Overview
### [CO](../reference/co.md)
A CO is a virtual room for collaboration.

### [Core](../reference/core.md)
A Core is a CO Reducer.

### [Guards](../reference/guards.md)
Guards are checks for transactions.

### [Storage](../reference/storage.md)
Content addressed block storage.
Implemented by the `co-storage` package.

### [Log](../reference/log.md)
Merkle-CRDT log. Conflict free stream of transactions.
Implemented by the `co-log` package.

### [Network](../reference/network.md)
Peer-to-Peer networking.
Implemented by the `co-network` package.

### [Identity](../reference/identity.md)
Decentralized Identifiers.
Implemented by the `co-identity` package.

### [Permissions](../reference/permissions.md)
Permissions are checks for states.

### [Consensus](../reference/consensus.md)
Consensus is the validated state of a CO.

### [Architecture](../reference/architecture.md)
High level architecture overview.

## Project Structure

### Libraries to build on top of CO-kit

These are the main crates which are used to build on top of CO-kit.

- [co-sdk](https://gitlab.1io.com/1io/co-sdk/-/tree/main/co-sdk): The main package to develop CO-kit based applications.
- [co-api](https://gitlab.1io.com/1io/co-sdk/-/tree/main/co-api): The main package to develop Cores.

### Command line

Utilities to accelerate development and integrations.

- [co-cli](https://gitlab.1io.com/1io/co-sdk/-/tree/main/co-cli): `co` command line to interact with COs from command line.
- [daemon](https://gitlab.1io.com/1io/co-sdk/-/tree/main/daemon): HTTP daemon which exposes COs as HTTP API.

### Framework integrations

Ready-to-use CO-kit integrations for different platforms and frameworks.

- [co-dioxus](https://gitlab.1io.com/1io/co-sdk/-/tree/main/co-dioxus): Integration and hooks for dioxus.
- [tauri-plugin-co-sdk](https://gitlab.1io.com/1io/co-sdk/-/tree/main/tauri-plugin-co-sdk): Integrations and hooks for tauri and react.
- Coming soon:
  - co-swift: Integrations for macOS and iOS application development.
  - co-android: Integrations for Android application development.

### Network and Sync

These are the necessary components to enable distributed use of COs.

- [co-network](https://gitlab.1io.com/1io/co-sdk/-/tree/main/co-network): Peer-to-Peer networking implementation.
- [co-log](https://gitlab.1io.com/1io/co-sdk/-/tree/main/co-log): Merkle-CRDT log (event stream) implementation. 

### Storage and Encryption

Storage in CO-kit is content-addressed.

- [co-storage](https://gitlab.1io.com/1io/co-sdk/-/tree/main/co-storage): BlockStorage implementations. Including filesystem, memory and encryption.

### Identity

Identities in CO-kit are fully decentralized.

- [co-identity](https://gitlab.1io.com/1io/co-sdk/-/tree/main/co-identity): [DID](../glossary/glossary.md#DID) Integration, didcomm primitives and [DID](../glossary/glossary.md#DID) methods supported by default.

### Built-in Cores

CO-kit ships with built-in cores that are either used to build/further develop CO-kit itself, or which are useful in general to build applications.

- [cores](https://gitlab.1io.com/1io/co-sdk/-/tree/main/cores): The built-in cores with an description file of the current CIDs.

### Internals

Packages for internals of CO-kit. These are used by contributors of CO-kit.

- [co-primitives](https://gitlab.1io.com/1io/co-sdk/-/tree/main/co-primitives): Primitives used throughout the `co-sdk` and `co-api` and [Core](../reference/core.md) implementations.
- [co-macros](https://gitlab.1io.com/1io/co-sdk/-/tree/main/co-macros): Marco implementations.
- [co-actor](https://gitlab.1io.com/1io/co-sdk/-/tree/main/co-actor): Very lightweight actor abstraction over [tokio](../glossary/glossary.md#Tokio) channels. Used to model local services.
- [co-runtime](https://gitlab.1io.com/1io/co-sdk/-/tree/main/co-runtime): WebAssembly runtime implementation.
- [co-messaging](https://gitlab.1io.com/1io/co-sdk/-/tree/main/co-messaging): Matrix compatible messaging primitives. Used in [co-core-room](../reference/core.md#co-core-room).

### Documentation

- [docs](https://gitlab.1io.com/1io/co-sdk/-/tree/main/docs): We used the [mdBook](https://rust-lang.github.io/mdBook/) sources for this documentation.

