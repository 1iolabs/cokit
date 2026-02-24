# Consensus Modes

## Overview

Consensus in COkit is **optional and on-demand**. It provides finality -- an agreed-upon,
immutable state that no future events can alter before that point.

Without consensus, the Merkle-CRDT Log alone serves as the source of truth, and eventual
consistency is guaranteed by the CRDT properties.

## How Consensus Works

When heads from different participants are joined, the event order in the Log may change
(this is normal Merkle-CRDT behavior). Consensus produces a **checkpoint**: a combination
of specific heads + the materialized state at those heads.

After a successful consensus round, no events can be inserted that would alter sorting
before the checkpoint. This provides finality.

## Checkpoints

A checkpoint = heads + materialized state at those heads. Think of it like a block in
a blockchain, but more versatile -- not dependent on a mining period, works on demand.

## Consensus on Demand (CoD)

A consensus round can be triggered manually at any time. One participant requests finality;
others collaborate to sign the checkpoint. No continuous mining or voting is needed.

## Asynchronous

Consensus is asynchronous. Users only wait for it when explicitly needed. Normal operations
proceed without blocking on consensus. This differs from traditional databases where
every write must achieve finality immediately (adding latency).

## Available Consensus Modes

| Mode | Status | Description |
|------|--------|-------------|
| **none** | Available | No explicit agreement. Relies solely on CRDT merge guarantees. |
| **proof-of-authority** | Available | Updates signed by designated authority DIDs. Built-in via co-core-poa. |
| **manual** | Planned (issue #87) | Users/admins explicitly approve changes before commit. |
| **shared** | Planned (issue #88) | Quorum/team of peers must agree to apply state changes. |

## Proof of Authority (PoA) - Detail

The built-in consensus mechanism, implemented in the `co-core-poa` Core.

### Configuration
When creating a CO with PoA, specify a list of authority DIDs. These authorities are
responsible for voting on checkpoints.

### Process
1. A participant requests finality (triggers consensus round)
2. Authority nodes receive the request
3. Each authority evaluates the checkpoint (heads + state)
4. Authorities sign the checkpoint
5. When majority is reached, the checkpoint is finalized

### Majority Rule
Default: **2/3 majority** (Byzantine Fault Tolerant).
Configurable, but setting less than 2/3 reduces fault tolerance (documented pitfall).

### Guard Integration
PoA includes a Guard that checks new transactions against the latest consensus.
Non-conforming transactions (those that would alter history before the last checkpoint)
are rejected.

## Use Cases for Consensus

### Client/Server Pattern
Define the server(s) as the authority. Clients can trust that finalized state is
acknowledged by the server.

### Digital Receipts
Two parties in a transaction use a CO. After capturing data, both sign (finalize)
the CO as a verifiable proof of the transaction.

### Multi-Region Cloud
Authorities spread across continents (e.g., Europe, NA, Asia). Two of three regions
forming majority ensures global replication/acceptance.

### Custom Consensus
Consensus is implemented as a Core with a Guard. Developers can implement any custom
consensus mechanism by creating a new Core + Guard combination.

## Consensus and Offline

If too many authority peers are offline, consensus may be blocked. However:
- Work continues (actions can still be submitted, CRDT sync still works)
- Finality will recover when peers reconnect
- Consensus blocking only affects finalization, not day-to-day operations

## Source Map

- docs/src/reference/consensus.md (primary: all consensus concepts, PoA, configuration)
- docs/src/glossary/glossary.md (definitions: Consensus, PoA)
- docs/src/reference/core.md (co-core-poa description)
- docs/src/reference/guards.md (PoA Guard integration, sequence diagram)
- docs/src/faq/faq.md (Q&A: custom consensus, offline + consensus)
- docs/src/usage/best-practices.md (pitfall: PoA misconfiguration)
