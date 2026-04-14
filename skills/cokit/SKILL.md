---
name: cokit
description: >
  High-level overview of COKIT — the local-first, decentralized Rust SDK for collaborative apps.
  Use this skill when the user asks about COKIT concepts, architecture, design decisions, or how
  pieces fit together. Trigger on: "what is COKIT", "how does COKIT work", "COKIT architecture",
  "CO vs Core", planning a new COKIT app, choosing between COs, or any question about COKIT
  fundamentals that isn't specifically about writing a core reducer or building a Dioxus component.
  For core/reducer development use `cokit-core`. For Dioxus UI use `cokit-dioxus`.
---

# COKIT

COKIT is a local-first, decentralized Rust SDK for building collaborative applications.
Data lives on the user's device, syncs peer-to-peer, and resolves conflicts automatically — no central server required.

## Core Abstractions

### CO (Collaborative Object)

A CO is a virtual room for collaboration. It bundles:

- **Cores** — data models with business logic
- **Participants** — identified by DIDs, from zero to millions
- **Networking** — how this CO syncs (P2P, relay, or none)
- **Encryption** — per-CO key management

CO types: **Local** (device-only, always encrypted), **Private** (shared among participants, encrypted), **Public** (unencrypted, open read), **Personal** (single-owner, wallet-like).

A CO is identified by a `CoId`. You interact with a CO by dispatching actions to its cores and reading derived state.

### Core (CO Reducer)

A Core is a pure, deterministic Rust function compiled to WASM. It receives `(state, action) → new state`. All business logic, validation, and permission checks live inside cores.

Cores are passive — no network calls, no clocks, no side effects. This guarantees every peer computes identical state from the same actions.

> Use the **cokit-core** skill for writing reducers, designing actions, collections, guards, and testing.

### Log (Merkle-CRDT)

Every CO is backed by an immutable, append-only event log built as a Merkle-DAG. Each entry references its parents by content hash (CID), forming a cryptographically verifiable causal history.

Conflict resolution is automatic: peers that diverge converge when they exchange heads, without consensus. This is why cores must accept actions in any order — action design is critical.

**Heads** = tips of the log = current state of a CO.

### Identity (DIDs)

Every participant is a W3C DID (Decentralized Identifier). Self-sovereign — users generate their own, no authority needed. Every log entry is cryptographically signed by the actor's DID.

### Storage (Content-Addressed)

All data is CID-addressed blocks. Storage is layered: base (filesystem or memory) → encryption (XChaCha20-Poly1305) → network (on-demand fetch). Partial data is first-class — you never need a full replica.

### Guards

Optional WASM functions that gate which transactions enter the log. The built-in "Is Participant" guard checks that the action author is a CO participant.

## Architecture

```
App (Dioxus / Tauri / CLI)
    ↓ hooks or API
CO SDK (co-sdk) — manages COs
    ↓
COs → Log → Storage
         ↘ Network (P2P sync)
    ↓ dispatches to
Cores (WASM) → deterministic state transitions
```

Key crates and their roles:

| Crate | Role |
|-------|------|
| `co-sdk` | App-facing SDK — managing COs, identity, networking |
| `co-api` | Core development — Reducer trait, collections, types |
| `co-dioxus` | Dioxus integration — reactive hooks, signals |
| `co-actor` | Actor system for business logic services |
| `co-cli` | CLI tooling (`co core build`) |

## Best Practices

These principles prevent the most common mistakes when building COKIT apps.

### Reading is cheap — never cache CO state

Reading state from cores costs very little. Copying it into local variables, signals, or caches creates stale duplicates that diverge from truth.

- Pass `Co`, `Link<T>`, or `CoId` references — resolve them when you need data
- Never mirror CO state into `use_signal` or local structs
- Derive views on read, don't maintain shadow state

### COs are the single source of truth

All meaningful application state lives in COs. There is no separate "app state" layer.
If something must persist or sync, it belongs in a CO.
Purely ephemeral UI state (dropdown open, scroll position) can live in Dioxus signals — but nothing in between.

### Reactive, not polling

COKIT is event-driven. State changes propagate through subscriptions and reactive signals.
Never use timers or polling loops.

- In Dioxus: `use_co`, `use_selector_state`, `use_selector` react to changes automatically
- In services: `heads_action_stream` provides event streams
- Between components: pass `ReadSignal<Link<T>>` as props, children subscribe themselves

### Extract complexity into services

UI components should be thin: read state, render, dispatch actions. Complex async logic, multi-CO coordination, or side effects belong in **services** built with `co_actor`:

- **Reducer** (sync) — handles incoming messages, updates local actor state
- **Epics** (async) — react to actions, perform async CO operations via `Application` API, yield follow-up actions
- The UI sends messages to the actor via a typed API (e.g. `MessengerApi`), never drives complex workflows itself

This pattern keeps UI declarative and business logic testable — epics work directly with `Application`, not with Dioxus hooks.

### Action design is order-independent

Actions may arrive in any order across peers, so they must converge regardless of sequence.
Name actions by intent (`Invite`, `Archive`, `Move`), not CRUD.
Keep logical operations atomic — never split a "move" into "remove" + "add".

> The **cokit-core** skill covers action design, stable identifiers, and the 1 MiB block limit in detail.

## Typical App Architecture

For non-trivial apps, a layered architecture keeps things clean:

```
Dioxus UI
  ├── reads state:  use_co + use_selector_state
  ├── simple writes: co.dispatch / co.push
  └── complex logic: sends messages to Actor
          ↓
Actor (co_actor)
  ├── Reducer — sync message handling
  └── Epics — async CO operations via Application
          ↓
CO SDK — COs, Cores, Storage, Sync, Identity
```

The UI never reaches into storage or network directly. Simple reads and writes go through CO hooks. Anything involving multiple COs, async coordination, or side effects routes through the Actor layer.

## Common Patterns

### Settings CO

A single CO holding all user preferences, with typed entry variants in a dedicated core. One CO, not one per setting type — keeps things discoverable and consistent.

### Profile COs

User profile data lives in dedicated Profile COs (public/protected/private tiers), not stuffed into participant tags. Resolve a DID to its Profile CO, then subscribe reactively.

## Skill Reference

| I want to... | Skill |
|---|---|
| Write a core (reducer, state, actions, collections, guards) | **cokit-core** |
| Build a Dioxus UI that reads/writes COs | **cokit-dioxus** |
| Understand COKIT concepts, architecture, and design | **this skill** |
