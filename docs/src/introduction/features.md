# Features

## Fully Decentralized
- No centralized coordinator or server dependency.
- Designed for sovereign and federated setups where each participant retains control.

## Flexible Storage
- Fully File-Based Architecture: CO-kit uses a file-centric model. All operations and data structures are persisted in files.
- Storage Agnostic: Compatible with any local or cloud-based storage systems (e.g., local FS, NFS, S3, etc.).
- Each node can operate with partial local datasets, enabling efficient sync and low network and storage overhead.
- Nodes selectively fetch only the data they need.

## Flexible Networking Model
CO-kit provides a pluggable networking layer with optional components:
- Builtin Peer-to-Peer (P2P): Native P2P capabilities are integrated, but entirely optional.
	- When devices are able to connect locally (LAN, Wifi, Bluetooth[^issue-79], ...) no internet is used.
- Configurable connectivity per CO:
	- DidContact: Gossipsub based mesh networking discovery.
	- Rendezvous: Use of shared discovery services or coordinators.
	- Direct: Explicitly configured endpoints (IP/DNS).
	- PubSub: For broadcasting and subscription-based connectivity.
	- HTTP[^issue-78]: Use classical client/server connectivity.

[^issue-78]: [Network: HTTP (#78)](https://gitlab.1io.com/1io/co-sdk/-/issues/78)
[^issue-79]: [Network: Bluetooth (BLE) (#79)](https://gitlab.1io.com/1io/co-sdk/-/issues/79)

## Flexible Sync and Data Integrity
- Conflict-Free Collaboration
- Merkle-CRDT Log-Based Sync: Built-in Conflict-Free Replicated Data Types (CRDTs) using Merkle trees and append-only logs.
- Ensures deterministic merges and high traceability across replicas.
- [Content addressing](/glossary/glossary.md#CID) which ensures validity of data.

## Flexible Consensus
Each CO can define its own consensus mechanism:
- Optional: No consensus; CRDT-only.
- Proof-of-authority: Signed checkpoints by trusted nodes.
- Manual: User-driven or admin-approved commits.
- Shared: All participants share responsibility; quorum or similar models.

## Flexible Data
Each CO may contain multiple [cores](/reference/core.md). A core defines a data model:
- Cores are compiled to WebAssembly (WASM) for safe, fast, and portable execution.
- Cores can be versioned, sandboxed, and upgraded.
- Cores may contain any data depending on your individual requirements.
