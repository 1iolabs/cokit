# Glossary and Mental Model

## Core Terminology

### COkit
The SDK itself. Written in Rust. Used to build decentralized collaborative applications.
Sometimes written as "CO-kit" in older references. The canonical current name is **COkit**.

### CO (Collaborative Object)
A virtual room for collaboration. A lightweight container for Cores, participants, network
settings, and encryption settings. Each CO is unique (like a receipt). COs are cheap to
create -- millions can exist. Each runs locally on every participant's device. Think of
it as a per-task or per-context database.

### Core (CO Reducer)
Combines data model with business logic. A pure reducer function: takes current state +
action, returns new state. Compiled to WASM, executed in sandbox. "Core" = "CO Reducer."

### Log
The conflict-free replicated event stream that event-sources each CO. Immutable,
cryptographically verifiable, eventually consistent. Sorted using a Merkle-DAG-based
logical clock. Analogous to a git graph where each commit is a transaction.

### Heads
The tips of the Log DAG. Represent the latest known state. Multiple heads = concurrent
changes from different peers. Joining heads produces deterministic merged order.

### Guard
A check for transactions. Guards gate what enters the Log. Evaluated BEFORE conflict
resolution (pre-CRDT). Compiled to WASM, pure functions. Used for consensus enforcement
and CO-wide invariant checks.

### Permission
A check for state. Evaluated AFTER conflict resolution. Implemented inside Core logic.
Permanent (re-evaluated when conflicts reorder events). More granular than Guards.

### Storage
Content-addressed block storage. Reads/writes CID/BLOB pairs (Blocks). Layered:
encryption layer, network layer. Backends: filesystem, memory.

### Consensus
Optional mechanism for reaching a validated (finalized) CO state. Produces checkpoints.
Built-in: Proof-of-Authority (PoA). Without consensus, Merkle-CRDT is sole truth.

## Data Format Terms

### CID (Content Identifier)
Self-describing, content-addressed identifier. Refers to data by its cryptographic hash
(what the data IS), not where it is stored. Any change yields a different CID.
COkit uses CIDs throughout for tamper-evident data referencing.

### IPLD (InterPlanetary Linked Data)
A single namespace for all hash-inspired protocols. CIDs and content-addressed links
traverse across protocols. COkit uses the IPLD data model.

### DAG-CBOR
The IPLD Data Model as a subset of CBOR. The default/recommended serialization format
throughout COkit. Supports content-addressed links (CIDs) natively.

### Merkle-DAG
A content-addressed Directed Acyclic Graph. Each node is cryptographically hashed based
on its payload and links to children. Self-verifying, immutable, supports deduplication.
Foundation for COkit's Log and storage.

### Merkle-CRDT
Combines Merkle-DAGs with CRDTs. CRDT payloads are embedded in Merkle-DAG nodes.
Each update is a self-verifying event. Provides Merkle-Clocks for causality/ordering.
Enables per-object causal consistency without strict messaging protocols.

### CRDT (Conflict-Free Replicated Data Type)
Data structure allowing independent concurrent updates that deterministically merge.
COkit's built-in Merkle-CRDT uses this to ensure eventual consistency.

### WASM (WebAssembly)
Portable binary format for sandboxed execution. COkit compiles Cores to WASM for
deterministic, cross-peer execution. Uses LLVM-18 for native-speed execution.

## Identity Terms

### DID (Decentralized Identifier)
W3C standard for verifiable, decentralized digital identity. Format: `did:<method>:<id>`.
Every COkit interaction is DID-signed. Users can have many pairwise-unique DIDs.

### PeerID
Unique reference to a peer in the P2P network. Derived from hashing the node's public key.
Can be rotated for privacy.

### DIDComm
DID-based messaging protocol. COkit uses DIDComm for peer messaging and discovery.
Spec: DIDComm Messaging v2.1.

## Framework Terms

### Dioxus
Rust-based cross-platform UI framework. COkit provides `co-dioxus` integration with hooks
(`use_co`, `use_selector`, `use_did_key_identity`).

### Tauri
Cross-platform desktop/mobile app framework (Rust backend + web UI). COkit provides
`tauri-plugin-co-sdk` for React/TypeScript integration.

## Source Map

- docs/src/glossary/glossary.md (primary source for all terms)
- docs/src/reference/*.md (detailed per-topic definitions)
- docs/src/introduction/about-co-kit.md (CO and Core mental models)
