# CO-kit
A software development kit to build decentralized applications with, written in Rust.

## CO-kit is like ...
#todo
CO-kit is like a database that combines blockchains zero trust and consensus (but optional) with git and CRDT then integrate all with verifiable identity.

## CO-kit for ...
#todo
### Frontend developers
#todo
- Like BaaS but for free
- Realtime database and collaboration
- Data which are used/produced on a device is automatically available locally
- Builtin identity management using DID
- No special handling for files as they are just data which can be stored using content addressing
- Existing cores can be used in your app without a single line of code

### Backend developers
#todo
- Cores work like containers which contain the business logic and data but shareable over network

### Database nerds
#todo
- Cores are like databases with builtin atomicity, consistency, isolation and durability
- The Log with its resulting states are a MVCC
- Like a Master-Master replication where your logic can decide what happens on conflicts whereas with transactions only the database can
	- With classical databases only transactions/operations can be reapplied/reordered
		- But without knowing the business logic behind
		- Cores reapply the whole business logic when reorder a transaction

## Overview
### [CO](../reference/co.md)
A CO is a virtual room for collaboration.
It is a container for cores and participants.

### [Core](../reference/core.md)
A Core is a CO Reducer.
It combines data model with business logic.

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

