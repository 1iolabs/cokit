# Features

## Fully Decentralized
- No centralized coordinator or server/cloud/infrastructure dependency.
- Designed for sovereign and federated setups where each participant retains control.

## Flexible Storage
- Fully file-based architecture: CO-kit, by default, uses a file-centric model. All operations and data structures are persisted in files.
- Storage Agnostic: compatible with any local or cloud-based file systems (e.g., local FS, NFS, S3, etc.).
- Each node can operate with partial local datasets, enabling efficient syncing and low network and storage overhead.
- Nodes selectively fetch _only_ the data they need.

## Flexible Networking Model
CO-kit provides a pluggable networking layer with optional components:
- Builtin Peer-to-Peer (P2P): Native P2P capabilities are integrated, but entirely optional.
	- When devices are able to connect locally (LAN, Wifi, Bluetooth[^issue-79], ...) no internet is used.
- [Configurable connectivity](../reference/network.md#network-configuration) per [CO](../reference/co.md)

[^issue-79]: [Network: Bluetooth (BLE) (#79)](https://gitlab.1io.com/1io/co-sdk/-/issues/79)

## Flexible Sync and Data Integrity
- Conflict-Free Collaboration
- [Merkle-CRDT](../glossary/glossary.md#merkle-crdt) Log-Based Sync: Built-in Conflict-Free Replicated Data Types (CRDTs) using Merkle trees and append-only logs.
- Ensures deterministic merges and high traceability across replicas.
- [Content addressing](../glossary/glossary.md#cid) which ensures validity of data.

## Flexible Consensus
Each CO can define its own consensus mechanism:
- Optional: no consensus – CRDT-only.
- [Proof-of-authority](../glossary/glossary.md#proofofauthority-poa-consensus-mechanism): signed checkpoints by trusted nodes.
- Manual: user-driven or admin-approved commits.
- Shared: all participants share responsibility – quorum model.

## Flexible Data
Each CO may contain multiple [cores](../reference/core.md). A core defines a data model:
- Cores are compiled to [WebAssembly](../glossary/glossary.md#wasm) (WASM) for safe, fast, and portable execution.
- Cores are versioned, sandboxed, and upgradable.
- Cores may contain any data depending on your individual requirements.
