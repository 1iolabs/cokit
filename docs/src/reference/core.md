# Core
Core stands for CO Reducer.  
It combines data model with business logic.

A reducer is a function that takes the current state and an action as input. It figures out how the state should change based on that action, and returns that new state without directly modifying the old one.

Core reducers are "pure" functions, meaning they always give the same output for the same state and action. This pureness is necessary to make distributed state and validation possible.  
For that reason they will be compiled to WebAssembly and executed in a sandbox.

## Design choices
Cores are reducer-based to allow for easy reasoning and observability. They are predictable and easily testable. Their clear interface allows for composition.  

All changes are automatically atomic, meaning each reduce operation guarantees that it is treated as a single "unit".  

Because Cores provide strict separation of concerns, they are executed in isolation, which allows for verifiability and parallel execution.

## Structure
Below is an overview of how Cores are structured and implemented.
### Schema
The Core schema is the data model of the Core and the actual data types that form the state.

In Rust, these types are usually represented by `structs` and `enums` and need to be serializable.
The [`co-api`](/crate/co_api/index.html) provides the [`#[co]`](/crate/co_api/attr.co.html) macro which will automatically implement required traits.

The [`co-api`](/crate/co_api/index.html) package provides advanced data types like maps, sets and lists with serialization support.
These data types provide content-addressed serialization into blocks.

For further information, see:
- [Core Quick Start](../getting-started/rust-core-quick-start.md#1-define-your-data-model-in-a-core)
- [Serialization](#serialization)
- [Core API Overview](../usage/api-overview-core.md)

### Actions
Core actions are operations on state that will be reduced/applied using the model.

They should be designed as logical tasks, as they get sorted by the [Log](../reference/log.md) and should be as order-independent as possible.

This means logical operations (like a move) should not be split into two actions, but rather represented as single action.  
The more order-independent the actions are, the better the CRDT can handle conflicts.

Each action sees a consistent state and will be applied as all or nothing.  
Actions needs to be serializable into content-addressed blocks.

For further information, see:
- [Core Quick Start](../getting-started/rust-core-quick-start.md#2-define-how-the-state-can-be-modified)
- [Serialization](#serialization)

### State
The Core state is the schema's resulting root state from actions that are applied sequentially by the [Log's](../reference/log.md) order.

States needs to be serializable into content-addressed blocks.

For further information, see:
- [Serialization](#serialization)

## Characteristics
### Passive
As Cores are reducers, they only materialize/calculate new states based on inputs.  
They have no facilities to react to state changes or perform any side effects.

### Serialization
The Core is the description of how state will be serialized to a persistable format.  
CO-kit uses content-addressed blocks through [IPLD](https://ipld.io/) with a default block size limit of 1MiB.  

We recommend using the [DAG-CBOR](../glossary/glossary.md#dag-cbor) format because it is optimized for content-addressed data, and it directly supports content-addressed links (via [CIDs](../glossary/glossary.md#cid)).  

However, any format, even plain binary, is usable as long as it can be adapted to the block size.

### Validatability
The pure, deterministic reducer is compiled to WebAssembly so that all peers can compute the same state transition, enabling a mechanism whereby everyone reaches the same result.

In addition, [Consensus](../reference/consensus.md) Cores can be used to check (with a Guard) and produce the finalized state, thereby producing trust among all CO participants. Producing finality is optional and on-demand.

### Atomicity
Each reducer operation is one "unit" and, by design, either succeeds completely, or fails completely.

## Permissions
Permissions are usually implemented in the data model and logic.  
Therefore, they are inherent to the Cores.

For further information, see:
- [Permissions](../reference/permissions.md)

## Features

### Compiled to WebAssembly
To provide maximum flexibility to developers, Cores are compiled to WebAssembly.  
This allows custom logic and supports arbitrarily-complex data models, so Cores can be structured in any way needed.

### Migrations
A migration of a state (for example from version 1.0 to version 2.0) is just another operation that can be supplied when updating a core binary.  
Therefore it can be programmed just like any other reducer operation.  

This approach is highly flexible and leverages the simple-yet-effective characteristics of cores.  
These migrations can be used for schema and data alike.

For further information, see:
- [co-core-co: CoAction: CoreUpgrade](/crate/co_core_co/enum.CoAction.html#variant.CoreUpgrade)
- [example-counter-upgraded: CounterAction: MigrateFromV1](/crate/example_counter_upgraded/enum.CounterAction.html#variant.MigrateFromV1)

## Higher order Cores
Existing Cores can be easily composed into a new Core, making more complex data models possible.  

In other words: Don’t mutate the original Core. Instead use composition since it has a well-specified interface.

You can either pass relevant data on, or specifically handle it the way you need.  
This maximizes the composability of Cores.

For example: You could easily create a Core for managing Markdown documents that uses multiple rich-text states: one for each document.

## Built-in Cores
We provide a set of built-in Cores ("core" Cores, if you will), and are constantly working on bringing you even more.  

The following is a list of the current built-in Cores:

#### `co-core-co`
Root Core that manages the COs Cores, Guards and Participants.

For further information, see:
- [`co-core-co`](/crate/co_core_co/index.html)

#### `co-core-keystore`
Stores credentials.  
Used internally in the [Local CO](../reference/co.md#local-co) to store DID and PeerID private keys.

For further information, see:
- [`co-core-keystore`](/crate/co_core_keystore/index.html)

#### `co-core-membership`
Stores membership information of COs.  
Used internally in the [Local CO](../reference/co.md#local-co) to track which COs our identities are a member of.

For further information, see:
- [`co-core-membership`](/crate/co_core_membership/index.html)

#### `co-core-board`
Kanban Board Core.  
Used internally in the [Local CO](../reference/co.md#local-co) to coordinate pending network requests.

For further information, see:
- [`co-core-board`](/crate/co_core_board/index.html)

#### `co-core-storage`
Stores reference information about existing storage blocks.  
Used internally in the [Local CO](../reference/co.md#local-co) to free blocks from storage that are no longer referenced.

For further information, see:
- [`co-core-storage`](/crate/co_core_storage/index.html)

#### `co-core-poa`
Proof-of-authority (PoA) Core.  
Provides PoA [consensus](../reference/consensus.md) for a CO.

For further information, see:
- [`co-core-poa`](/crate/co_core_poa/index.html)

#### `co-core-room`
Messaging Core.  
Stores messages in Matrix-compatible format.

For further information, see:
- [`co-core-room`](/crate/co_core_room/index.html)

#### `co-core-file`
Stores hierarchical file structures like a file system.

For further information, see:
- [`co-core-file`](/crate/co_core_file/index.html)

#### `co-core-data-series`
Stores data series, counters, and aggregations on the data.

For further information, see:
- [`co-core-data-series`](/crate/co_core_data_series/index.html)

#### `co-core-rich-text`
Stores conflict-free rich text.

For further information, see:
- [`co-core-rich-text`](/crate/co_core_rich_text/index.html)

#### `co-core-role`
Basic role-based access rules.  
Used as a go-to data model for daily permission management.

For further information, see:
- [`co-core-role`](/crate/co_core_role/index.html)
