# Log, Sync, and Conflict Resolution

## The Log

Each CO is event-sourced by a single Log. The Log is:
- **Immutable**: Events are append-only.
- **Cryptographically verifiable**: Each event is content-addressed (CID) and DID-signed.
- **Eventually consistent**: All peers converge to the same state given the same events.
- **Sorted by Merkle-DAG logical clock**: Deterministic ordering based on DAG structure.

## How the Log Works

Think of the Log as a git graph where each commit is a transaction/action. Events form
a DAG (Directed Acyclic Graph). Each event links to its parent event(s).

**Heads** are the tips of the DAG -- the latest known events. Multiple heads indicate
concurrent changes from different peers.

**Joining heads** means merging branches. When heads from different peers are joined:
1. The Merkle-DAG structure provides a logical clock.
2. Events are deterministically sorted based on this clock.
3. When concurrent events share the same logical clock position, they are sorted
   deterministically (by CID or similar tie-breaking).
4. The resulting sorted event list is the same for all peers who have the same events.

## Merkle-CRDT Design

COkit's Log is a Merkle-CRDT:
- CRDT payloads (actions) are embedded within Merkle-DAG nodes.
- Each update is a self-verifying event in a content-addressed history.
- Merkle-Clocks function as logical clocks for causality tracking.
- Convergence is ensured through immutable DAG history + CRDT semantics.

This eliminates the need for consensus or strict messaging protocols for basic sync.

## Conflict Resolution

Conflicts in COkit are resolved by the Log's deterministic sorting:
1. When peers work concurrently (offline or in parallel), their Logs diverge.
2. When they reconnect and share heads, the Logs are joined.
3. The merged Log reorders ALL events deterministically.
4. Cores re-reduce the entire event stream to produce the new state.

Because actions (not states) are stored, and reducers are deterministic pure functions,
every peer arrives at the same state after processing the same set of events.

**Key implication**: Actions should be designed as order-independent as possible.
The more order-independent, the better the CRDT handles conflicts. A "move" operation
should be a single action, not split into "delete from source" + "add to destination."

## Sync Mechanism

1. A CO's heads change (new action submitted or remote heads received).
2. New heads are broadcast to connected peers (via network protocols like CoHeads/GossipSub).
3. Peers receive the new heads.
4. Peers fetch referenced data blocks on demand (via bitswap or other block exchange).
5. Peers join the new heads into their local Log.
6. The Log deterministically reorders events.
7. Cores recompute state.

Only heads are distributed initially -- participants fetch referenced data lazily and
in parallel. This enables partial data and efficient bandwidth usage.

## Network Partitions / Offline

Each peer continues working locally by appending events to its own Log branch.
Heads diverge during the partition. When connectivity resumes:
1. Heads are shared.
2. The Log joins all heads, sorting events deterministically.
3. State is recalculated.

No special offline handling is required by the developer.

## Reference: Merkle-CRDTs paper

The approach is based on: "Merkle-CRDTs: Merkle-DAGs meet CRDTs" (arXiv: 2004.00107).

## Source Map

- docs/src/reference/log.md (primary: Log concept, Merkle-CRDT, example with diagrams)
- docs/src/glossary/glossary.md (definitions: Merkle-CRDT, CRDT, Merkle-DAG, Heads)
- docs/src/reference/core.md (context: how Cores consume Log events)
- docs/src/faq/faq.md (Q&A: offline/sync, conflict resolution, network partition)
- docs/src/introduction/features.md (feature: Flexible Sync and Data Integrity)
