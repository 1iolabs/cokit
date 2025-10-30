## Overview of components
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
Implemented by the [`co-storage`](/crate/co_storage/index.html) package.

### [Log](../reference/log.md)
Merkle-CRDT log. Conflict free stream of transactions.
Implemented by the [`co-log`](/crate/co_log/index.html) package.

### [Network](../reference/network.md)
Various protocols for peer-to-peer networking.
Implemented by the [`co-network`](/crate/co_network/index.html) package.

### [Identity](../reference/identity.md)
Decentralized Identifiers using [DID](../glossary/glossary.md#did).
Implemented by the [`co-identity`](/crate/co_identity/index.html) package.

### [Permissions](../reference/permissions.md)
Permissions are checks for states.

### [Consensus](../reference/consensus.md)
Consensus is the validated state of a CO.

### [Architecture](../reference/architecture.md)
High level architecture overview.

## Project Structure

### Libraries to build on top of CO-kit
These are the main crates which are used to build on top of CO-kit:

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

- [co-identity](https://gitlab.1io.com/1io/co-sdk/-/tree/main/co-identity): [DID](../glossary/glossary.md#did) Integration, didcomm primitives and [DID](../glossary/glossary.md#did) methods supported by default.

### Built-in Cores
CO-kit ships with built-in cores that are either used to build/further develop CO-kit itself, or which are useful in general to build applications.

- [cores](https://gitlab.1io.com/1io/co-sdk/-/tree/main/cores): The built-in cores with an description file of the current CIDs.

### Internals
Packages for internals of CO-kit. These are used by contributors of CO-kit.

- [co-primitives](https://gitlab.1io.com/1io/co-sdk/-/tree/main/co-primitives): Primitives used throughout the [`co-sdk`](/crate/co_sdk/index.html) and [`co-api`](/crate/co_api/index.html) and [Core](../reference/core.md) implementations.
- [co-macros](https://gitlab.1io.com/1io/co-sdk/-/tree/main/co-macros): Marco implementations.
- [co-actor](https://gitlab.1io.com/1io/co-sdk/-/tree/main/co-actor): Very lightweight actor abstraction over [tokio](../glossary/glossary.md#tokio) channels. Used to model local services.
- [co-runtime](https://gitlab.1io.com/1io/co-sdk/-/tree/main/co-runtime): WebAssembly runtime implementation.
- [co-messaging](https://gitlab.1io.com/1io/co-sdk/-/tree/main/co-messaging): Matrix compatible messaging primitives. Used in [co-core-room](../reference/core.md#co-core-room).

### Documentation

- [docs](https://gitlab.1io.com/1io/co-sdk/-/tree/main/docs): We used the [mdBook](https://rust-lang.github.io/mdBook/) sources for this documentation.
