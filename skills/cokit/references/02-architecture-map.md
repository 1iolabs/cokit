# Architecture Map

## High-Level Component Hierarchy

```
Device
  |-- Filesystem (file-based persistence)
  |-- Network (platform network interface, optional)
  |-- App (application using COkit)
        |-- Storage (content-addressed block storage)
        |-- CO (virtual collaboration room)
        |     |-- Log (Merkle-CRDT event stream)
        |     |     |-- Heads (tips of the DAG)
        |     |-- Core (WASM reducer, one or more per CO)
        |           |-- Actions (change operations)
        |           |-- State (materialized result)
        |-- Network connections
```

## Component Roles

**Device** is the platform host that provides filesystem and network interfaces.

**Storage** is the content-addressed block store (CID/BLOB pairs). Supports layering:
- Base layer: filesystem or memory backend
- Encryption layer: encrypts blocks before writing (XChaCha20-Poly1305 default)
- Network layer: fetches unknown blocks on-demand from peers

**CO** is the collaboration container. Each CO has:
- One or more Cores (data model + logic)
- A Log (event history)
- Participants (identified by DIDs)
- Network settings (per-CO connectivity config)
- Encryption settings (encrypted or public)

**Log** is the Merkle-CRDT event stream. Each event is an action submitted by a
participant. The Log deterministically sorts events using a logical clock derived
from the Merkle-DAG structure.

**Core** is the WASM reducer. It materializes state by applying the Log's ordered
actions sequentially. `reduce(state, action) -> new_state`.

## Data Flow

1. User performs an action in the app
2. Action is serialized as a content-addressed block and appended to the Log
3. The Log event is signed by the user's DID
4. Guards validate the transaction (if configured)
5. The Core reducer applies the action to produce new state
6. Heads are broadcast to connected peers via the network
7. Peers receive heads, fetch referenced blocks, join heads into their Log
8. The merged Log deterministically reorders all events
9. Cores recompute state from the merged event order

## Project Crate Structure

### Libraries for building on COkit
- **co-sdk** - Main application-level package (Application, ApplicationBuilder, CoReducer, CoContext, CoStorage)
- **co-api** - Main Core development package (Reducer trait, Guard trait, `#[co]` macro)

### CLI and services
- **co-cli** - `co` CLI tool (commands: co, network, core, ipld, did, storage, file, room, pin, schemars)
- **daemon** - HTTP daemon exposing COs as HTTP API

### Framework integrations
- **co-dioxus** - Dioxus hooks (use_co, use_selector, use_did_key_identity)
- **tauri-plugin-co-sdk** - Tauri plugin for React/TypeScript apps
- **co-js** - JavaScript WASM wrappers (CoMap, BlockStorage, etc.)
- **co-swift** - iOS/macOS bindings (coming soon, issue #95)
- **co-android** - Android bindings (coming soon, issue #96)

### Infrastructure
- **co-network** - P2P networking implementation (libp2p-based)
- **co-log** - Merkle-CRDT log implementation
- **co-storage** - BlockStorage backends (FsStorage, MemoryBlockStorage, encryption)
- **co-identity** - DID integration, DIDComm primitives, DID methods

### Internals
- **co-primitives** - Shared types: CoMap, CoSet, CoList, Block, BlockStorage trait,
  BlockSerializer, NodeBuilder, NodeStream, CID helpers, etc.
- **co-macros** - Proc macros for `#[co]`, `#[co(state)]`, `#[co(guard)]`
- **co-actor** - Lightweight actor abstraction over tokio channels
- **co-runtime** - WASM runtime for Core execution
- **co-messaging** - Matrix-compatible messaging primitives (used in co-core-room)
- **co-bindings** - Language binding generation

### Built-in Cores (under cores/)
- co-core-co, co-core-keystore, co-core-membership, co-core-board, co-core-storage,
  co-core-poa, co-core-room, co-core-file, co-core-rich-text, co-core-role

## Source Map

- docs/src/reference/architecture.md (primary: component diagram and roles)
- docs/src/reference/co-kit.md (primary: project structure and crate listing)
- docs/src/reference/co.md, core.md, log.md, storage.md, network.md (component details)
