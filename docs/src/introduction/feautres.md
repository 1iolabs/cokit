# Features
## Core Features

### File-Based Architecture

- **Fully File-Based**: CoKit uses a file-centric model. All operations and data structures are persisted in files.
- **Storage Agnostic**: Compatible with any local or cloud-based storage systems (e.g., local FS, S3, IPFS, etc.).

---

###  Fully Decentralized

- No centralized coordinator or server dependency.
- Designed for sovereign and federated setups where each participant retains control.

---

## Networking Capabilities

### Flexible Networking Model

CoKit provides a pluggable networking layer with optional components:

- **Builtin Peer-to-Peer (P2P)**: Native P2P capabilities are integrated, but entirely optional.
- **Configurable Connectivity per CO (Collaborative Object)**:
  - `peer-to-peer`: direct mesh networking.
  - `rendezvous`: use of shared discovery services or coordinators.
  - `direct`: explicitly configured endpoints.
  - `pubsub`: for broadcasting and subscription-based sync.

---

## Data Integrity and Sync

### Conflict-Free Collaboration

- **Merkle-CRDT Log-Based Sync**: Built-in **Conflict-Free Replicated Data Types (CRDTs)** using Merkle trees and append-only logs.
- Ensures deterministic merges and high traceability across replicas.

---

## Consensus and Coordination

### Flexible Consensus Options

Each CO can define its own consensus mechanism:

- `none`: no consensus; last-write-wins or CRDT-only.
- `proof-of-authority`: signed updates by trusted nodes.
- `manual`: user-driven or admin-approved commits.
- `shared`: all members share responsibility; quorum or similar models.

---

## Data Modeling

### Flexible Data Schemas

- Schemas are compiled to **WebAssembly (WASM)** for safe, fast, and portable execution.
- Schema logic can be versioned, sandboxed, and upgraded.

---

## Storage Optimization

###  Partial Local Data

- Each node can operate with **partial local datasets**, enabling efficient sync and low storage overhead.
- Nodes selectively fetch only the data they need.




