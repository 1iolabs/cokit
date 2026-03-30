---
name: cokit
description: >-
  Provides accurate domain knowledge about COkit (also known as CO-kit), the Rust-based SDK
  by 1iO for building decentralized, local-first, peer-to-peer collaborative applications.
  Covers architecture (COs, Cores, Log, Storage, Network, Identity, Permissions, Consensus),
  data model (content-addressed DAG, Merkle-CRDT, CID, IPLD, DAG-CBOR, WASM reducers),
  developer workflows (co-api, co-sdk, co-cli, co-dioxus, tauri-plugin-co-sdk, React),
  security (DID, DIDComm, encryption, Guards), and licensing (AGPLv3 + linking exception).
  Auto-loads when user mentions COkit, CO-kit, co-sdk, co-api, co-cli, co-dioxus,
  tauri-plugin-co-sdk, co-js, or 1iO in the context of building apps. Also loads when
  CO (Collaborative Object), Core (CO Reducer), Merkle-CRDT, or Guards are discussed
  alongside other COkit terms.
---

# COkit Domain Knowledge

## What COkit Is

COkit is a software development kit written in Rust for building decentralized, local-first,
peer-to-peer collaborative applications. It enables developers to create apps where data lives
on users' devices, syncs directly between peers, and requires no backend servers or cloud
infrastructure by default (though it can optionally integrate with servers/cloud).

COkit provides built-in components for identity (via DIDs), encryption, permissions, conflict-free
data synchronization (via Merkle-CRDTs), and optional consensus mechanisms. Applications built
with COkit are cross-platform (iOS, Android, macOS, Windows, Linux) and support real-time
collaboration with automatic offline/online conflict resolution.

COkit is developed by 1iO BRANDGUARDIAN GmbH.

## When This Skill Should Activate

Load this skill when the user:
- Mentions COkit, CO-kit, co-sdk, co-api, co-cli, or 1iO SDK
- Asks about Collaborative Objects (COs) or CO Reducers (Cores)
- Discusses building local-first or decentralized collaborative apps with this SDK
- References Merkle-CRDT logs, content-addressed storage, or DAG-CBOR in COkit context
- Asks about DID-based identity, Guards, or Permissions in COkit
- Works with co-dioxus, tauri-plugin-co-sdk, or co-js integrations
- Asks about COkit licensing, contributing, or architecture

## Non-Goals / Do Not Assume

- Do NOT invent API names, function signatures, or package names not found in sources.
  If unsure, say "TBD / needs verification against the COkit API docs."
- Do NOT conflate COkit with other CRDT frameworks (e.g., Yjs, Automerge) or other
  decentralized platforms (e.g., IPFS directly, blockchain platforms).
- Do NOT assume features marked "coming soon" in the docs are available. Flag them.
- Do NOT reproduce confidential source code verbatim. Paraphrase and summarize.
- Mark anything uncertain with "TBD / needs verification."

## Mental Model

### CO (Collaborative Object)

A CO is a virtual room for collaboration. Think of it as a lightweight, ad-hoc container
(like a per-task database) that encapsulates Cores, participants, network settings, and
encryption settings. COs are cheap to create (millions possible) and each runs locally
on every participant's device.

**CO Types:**
- **Local CO** - Device-only, encrypted, stores identities/keys/memberships/settings. One per storage path.
- **Private CO** - Encrypted, participant-restricted, syncs between known DIDs.
- **Public CO** - Unencrypted, open to read by anyone.
- **Personal CO** - Like Private but typically single-user, used as a wallet for syncing across own devices.

### Core (CO Reducer)

A Core combines a data model with business logic via a pure reducer function:
`(state, action) -> new_state`. Cores are compiled to WASM and executed in a sandbox,
ensuring deterministic, verifiable state transitions across all peers.

Key properties: passive (no side effects), atomic, serializable, composable (higher-order Cores).

**Built-in Cores:** co-core-co (root), co-core-keystore, co-core-membership, co-core-board,
co-core-storage, co-core-poa, co-core-room, co-core-file,
co-core-rich-text.

### Log (Merkle-CRDT)

Each CO is event-sourced by an immutable, cryptographically verifiable, eventually consistent
Log. The Log uses a Merkle-DAG-based logical clock to deterministically order events.
Think of it like a git graph where each commit is a transaction. Heads represent the
latest state; joining heads from different peers produces a deterministic merged order.

### Storage

Content-addressed block storage using CID/BLOB pairs. Default serialization: DAG-CBOR.
Storage layers: encryption layer (XChaCha20-Poly1305), network layer (on-demand fetch).
Built-in backends: filesystem and memory. Data is a DAG accessed top-down, enabling
partial data / on-demand fetching.

### Network

Built-in P2P via libp2p (Rust implementation). Entirely optional. Per-CO configurable.
Protocols: mDNS (local discovery), Noise (encryption), QUIC (default transport),
didcomm (DID-based messaging), didcontact (GossipSub discovery), bitswap (block exchange
with token auth), CoHeads (GossipSub head broadcasting), Rendezvous (peer discovery).
NAT traversal via circuit relay and DCUtR hole punching.

### Identity

DIDs (W3C DID Core spec) are the fundamental identifier. Every interaction is DID-signed.
Participants can have multiple pairwise-unique DIDs for privacy. Keys stored in Local CO
keystore Core. Default method: did:key. Extensible to other DID methods.

### Guards vs Permissions

**Guards** check transactions BEFORE they enter the Log (pre-CRDT). They are instant,
order-independent, compiled to WASM. Used for consensus enforcement and CO-wide rules.
Built-in: is-participant check, PoA conformance check.

**Permissions** check state AFTER conflict resolution. They are permanent, re-evaluated
after conflicts. Implemented as logic in Core reducers. More granular but with storage overhead.

### Consensus

Optional and on-demand. Provides finality via checkpoints (heads + materialized state).
Built-in: Proof-of-Authority (PoA) via co-core-poa Core. PoA uses a list of authority DIDs;
2/3 majority by default (BFT). Other modes planned: manual, shared/quorum.
Without consensus, the Merkle-CRDT alone serves as the source of truth.

## How to Answer Questions Using This Skill

1. **Distinguish levels:** COkit (the SDK/toolkit) vs CO (the collaboration container)
   vs Core (the data model + reducer).
2. **Prefer source-backed statements.** If a claim cannot be traced to the docs, mark TBD.
3. **For architecture questions** -> see [references/02-architecture-map.md](references/02-architecture-map.md)
4. **For Core/reducer implementation** -> see [references/03-data-model-and-cores.md](references/03-data-model-and-cores.md)
5. **For sync/conflict questions** -> see [references/04-log-sync-conflicts.md](references/04-log-sync-conflicts.md)
6. **For networking** -> see [references/05-networking-and-discovery.md](references/05-networking-and-discovery.md)
7. **For identity/permissions** -> see [references/06-identity-permissions-guards.md](references/06-identity-permissions-guards.md)
8. **For consensus** -> see [references/07-consensus-modes.md](references/07-consensus-modes.md)
9. **For security** -> see [references/08-security-assumptions.md](references/08-security-assumptions.md)
10. **For dev workflows** -> see [references/09-dev-workflows-quickstarts.md](references/09-dev-workflows-quickstarts.md)
11. **For licensing** -> see [references/10-licensing-and-contributing.md](references/10-licensing-and-contributing.md)

## Quick Lookup Index

| Topic | Reference File |
|-------|---------------|
| Positioning, what COkit is | [00-overview-and-positioning.md](references/00-overview-and-positioning.md) |
| Glossary, mental model | [01-glossary-and-mental-model.md](references/01-glossary-and-mental-model.md) |
| Architecture diagram + components | [02-architecture-map.md](references/02-architecture-map.md) |
| Cores, schemas, actions, reducers | [03-data-model-and-cores.md](references/03-data-model-and-cores.md) |
| Log, Merkle-CRDT, sync, conflicts | [04-log-sync-conflicts.md](references/04-log-sync-conflicts.md) |
| Networking, libp2p, protocols | [05-networking-and-discovery.md](references/05-networking-and-discovery.md) |
| DID identity, Guards, Permissions | [06-identity-permissions-guards.md](references/06-identity-permissions-guards.md) |
| Consensus, PoA, finality | [07-consensus-modes.md](references/07-consensus-modes.md) |
| Security, cryptography, trust | [08-security-assumptions.md](references/08-security-assumptions.md) |
| Dev workflows, CLI, quick starts | [09-dev-workflows-quickstarts.md](references/09-dev-workflows-quickstarts.md) |
| Licensing, AGPLv3, CLA | [10-licensing-and-contributing.md](references/10-licensing-and-contributing.md) |

## Common Pitfalls

These are documented best practices and anti-patterns from the COkit docs:

1. **Use stable identifiers (UUIDs), not monotonic counters.** Counters may change
   during conflict resolution when the Log reorders events.
2. **Design actions as single logical tasks.** Do not split a logical operation
   (e.g., a "move") into multiple actions. Order-independent actions merge better.
3. **Do not reference previous states from Core logic.** This prevents garbage collection
   of old states, as all referenced states must be kept alive.
4. **PoA misconfiguration.** Do not configure Proof-of-Authority to allow minority
   consensus unless intentional. The default 2/3 majority provides BFT guarantees.
5. **Cores are passive.** They cannot trigger side effects or react to state changes.
   They only compute new state from (state, action) inputs.
6. **COkit crates are NOT on crates.io.** All Rust dependencies must be added via
   `--git https://gitlab.1io.com/1io/co-sdk.git`, not from the crates.io registry.
   Similarly, npm packages (`@1io/*`, `co-js`) are not on the public npm registry.

## Project Structure (Crate Map)

**Important: COkit packages are not published to crates.io or npm. Add them via git:**
```
cargo add co-sdk --git https://gitlab.1io.com/1io/co-sdk.git
cargo add co-api --git https://gitlab.1io.com/1io/co-sdk.git
```

**Application development:**
- `co-sdk` - Main package for building COkit-based applications
- `co-api` - Main package for developing Cores

**CLI and daemon:**
- `co-cli` - Command-line tool (`co`) for inspecting/interacting with COs
- `daemon` - HTTP daemon exposing COs as an HTTP API

**Framework integrations:**
- `co-dioxus` - Hooks and integrations for the Dioxus framework
- `tauri-plugin-co-sdk` - Plugin for Tauri + React applications
- `co-js` - JavaScript/TypeScript WASM wrappers
- `co-swift` - iOS/macOS bindings (coming soon)
- `co-android` - Android bindings (coming soon)

**Infrastructure:**
- `co-network` - P2P networking (libp2p-based)
- `co-log` - Merkle-CRDT log implementation
- `co-storage` - BlockStorage backends (filesystem, memory, encryption)
- `co-identity` - DID integration and DIDComm primitives

**Internals:**
- `co-primitives` - Shared types (CoMap, CoSet, CoList, BlockStorage, etc.)
- `co-macros` - Derive macros (`#[co]`, etc.)
- `co-actor` - Lightweight actor abstraction over tokio channels
- `co-runtime` - WASM runtime for executing Cores
- `co-messaging` - Matrix-compatible messaging primitives
- `co-bindings` - Language bindings
