# Core
Core stands for CO Reducer.

It combines data model with business logic.

A reducer is a function that takes the current state and an action as input.
It figures out how the state should change based on that action.
It returns a new state without directly modifying the old one.
Core reducers are pure functions, meaning it always gives the same output for the same state and action.
This pureness is necessary to make distributed state and validation possible.
For that reason they will be compiled to WebAssembly and executed in a sandbox.

## Design choices
Cores are reducer-based to allow for easy reasoning and observability. They are predictable and easily testable.
Their clear interface allows for composition.
All changes are automatically atomic, meaning each reduce operation guarantees that it is treated as a single "unit".
As Cores provide strict separation of concerns, they are executed in isolation which allows for verifiability and parallel execution.

## Structure
A overview of how Cores are structured and implemented.

### Schema
The core schema is data model of the core and are the actual data types that form the state.

In Rust, these types are usually represented by `structs` and `enums` and need to be serializable.
The `co-api` provides the `#[co]` macro which will automatically implement required traits.

The `co-api` package provides advanced data types like maps, sets and lists with serialization support.
These data types provide content-addressed serialization into blocks.

For further informations see:
- [Core Quick Start](../getting-started/rust-core-quick-start.md#1-define-your-data-model-in-a-core)
- [Serialization](#serialization)
- [Core API Overview](../usage/api-overview-core.md)

### Actions
Core actions are operations on state that will be reduced/applied using the model.

They should be designed as logical tasks, as they get sorted by the [Log](../reference/log.md) and should be as order-independent as possible.
This means logical operations (like a move) are not split into two actions, but rather represented as single action.
The more order-independent actions are, the better the CRDT can handle conflicts.

Each action sees a consistent state and will be applied as all or nothing.
Actions needs to be serializable into content-addressed blocks.

For further informations see:
- [Core Quick Start](../getting-started/rust-core-quick-start.md#2-define-how-the-state-can-be-modified)
- [Serialization](#serialization)

### State
The core state is the schema's resulting root state of actions that are applied sequentially by the [Log's](../reference/log.md) order.
States needs to be serializable into content-addressed blocks.

For further informations see:
- [Serialization](#serialization)

## Characteristics
### Passive
As cores are reducers, they only materialize/calculate new states based on inputs.
They have no facilities to react to state changes or perform any side effects.

### Serialization
The core is the description of how state will be serialized to a persistable format.
CO-kit uses content addressed blocks through [IPLD](https://ipld.io/) with a default block size limit of 1MiB.
We recommend to use the [DAG-CBOR](../glossary/glossary.md#dag-cbor) format because it is optimized for content addressed data and directly supports content addressed links (via [CIDs](../glossary/glossary.md#cid)).
However, any format, even plain binary, is usable as long as it can be adapted to the block size.

### Validatability
The pure, deterministic reducer is compiled to WebAssembly so that all peers can compute the same state transition, enabling a mechanism where everyone reaches the same result.
In addition, [Consensus](../reference/consensus.md) cores allow to finalize a state and therefore produce trust among all CO participants.

### Atomicity
Each reducer operation is one "unit" and either, by design, succeeds completely or fails completely.

## Permissions
Permissions are usually implemented in the data model and logic.
Therefore, they are inherent to the cores.
Some examples:
- Someone is allowed to comment on blog entries but not to create new blog entries.
- Someone is allowed to post new messages but not to delete them.
These checks are implemented as simple checks or conditions in the Core.

For an implementation example click [here](../getting-started/next-steps.md#permissions).

## Features

### Compiled to WebAssembly
To provide maximum flexibility to developers, cores are compiled to WebAssembly, allowing custom logic and supporting arbitrarily complex data models so cores can be structured in any way needed.

### Migrations
A migration of a state (for example from version 1.0 to version 2.0) is just another operation which can be supplied when updating a core binary.
Therefore it can be programmed just like any other reducer operation.
This approach is highly flexible and leverages the simple yet effective characteristics of cores.
These migrations can be used for schema and data alike.


## Higher order cores
Existing cores can be easily composed into a new core making more complex data models possible.
In other words: Don’t mutate the original Core, rather use composition since it's got a well-specified interface.
You can either pass relevant data on outwards or specifically handle it the way you need it.
This is maximizing composability of Cores.

For example: It is easily possible to create a Markdown document management-core which uses multiple rich-text states; one for each document.

## Built-in cores
We provide a set of cores.
We are constantly working on bringing you even more built-in cores. 
They are the following:

#### co-core-co
Root core which manages the COs core instances, guards and participants.

#### co-core-keystore
Stores credentials.
Used internally to store DID and PeerID private keys.

#### co-core-membership
Stores memberships informations of COs.
Used internally in the Local CO to track which COs one of out identities is a member of.

#### co-core-board
Kanban board core.
Used internally to coordinate pending network requests.

#### co-core-storage
Stores reference informations about storage blocks which exist on disk.
Used internally to free blocks from disk which are not referenced anymore.

#### co-core-poa
Proof-of-authority core.
Provides POA [consensus](../reference/consensus.md) for a CO.

#### co-core-room
Messaging core. Stores messages in matrix compatible format.

#### co-core-file
Stores hierarchical file structures like a file system.

#### co-core-data-series
Stores data series, counters and aggregations on the data.

#### co-core-rich-text
Stores conflict free rich text.

#### co-core-role
Basic role-based access rules. As a goto data model for daily permission management.

