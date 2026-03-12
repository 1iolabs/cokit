# Identity, Permissions, and Guards

## Identity (DIDs)

### DID as Foundation
COkit uses Decentralized Identifiers (DIDs) as the fundamental identity mechanism.
Aligned with the W3C DID Core specification. Format: `did:<method>:<method-specific-id>`.

Every interaction in COkit is signed by a DID, making everything cryptographically verifiable.

### Why DIDs
- Decentralized, self-sovereign, flexible
- Cryptographically verifiable, self-administered
- No centralized authority needed
- Pairwise-unique: participants can have thousands of context-specific DIDs
  (prevents cross-context tracking)

### Default DID Method
`did:key` -- a simple key-based DID method. Developers can extend supported methods.

### Where DIDs Appear
- **CO Participants**: Every participant (human or machine) is identified by DID(s).
- **Wallet (Keystore)**: DIDs stored in Local CO's `co-core-keystore` Core.
- **Networking**: didcomm and didcontact protocols use DIDs for discovery and communication.
- **Log**: Every event is signed by the submitting participant's DID.

### Creating an Identity (Rust)
Use `DidKeyIdentity::generate()` and store via `DidKeyProvider` in the Local CO keystore.

### Creating an Identity (React/Tauri)
Use the `useDidKeyIdentity(name)` hook, which finds or creates a `did:key` identity
with the given name.

## Guards

### What Guards Are
Guards are checks for transactions. They decide which transactions make it into the Log.
Think of them as a "police" for transactions.

### How Guards Work
1. New transaction received (e.g., from network)
2. All configured Guards of the CO are evaluated
3. If ANY Guard rejects, the transaction is rejected and does NOT enter the Log
4. Rejection happens BEFORE CRDT conflict resolution

### Guard Properties
- Pure functions, compiled to WASM
- Should be **order-independent** (return same result regardless of transaction order)
- Registered per-CO

### Guard Trait (Rust)
```
impl<S: BlockStorage> Guard<S> for MyGuard {
    async fn verify(
        storage: &S,
        guard: String,
        state: Cid,          // current state CID
        heads: BTreeSet<Cid>, // current heads
        next_head: Cid,       // transaction to check
    ) -> Result<bool, anyhow::Error>
}
```
Annotate with `#[co(guard)]`. WASM must export `fn guard()`.

### Built-in Guards
1. **Is Participant**: Peer must be a CO participant to write transactions.
   Implemented in `co-core-co`.
2. **PoA Consensus Conformance**: Checks new transactions against the latest
   Proof-of-Authority consensus state. Rejects non-conforming transactions.

## Permissions

### What Permissions Are
Permissions are checks for state (not transactions). They define what makes it into
the materialized state of a Core.

### How Permissions Work
- Evaluated AFTER transactions enter the Log and after conflict resolution
- Implemented as conditional logic inside the Core's reducer
- Permanent: re-evaluated whenever conflicts cause event reordering
- More granular than Guards (e.g., "can comment but not create posts")

### Guard vs Permission Comparison

| Property | Guard | Permission |
|----------|-------|------------|
| When evaluated | Before Log entry | After conflict resolution |
| Applies to | Transactions | State |
| Instant (evaluated once) | Yes | No |
| Permanent (re-evaluated) | No | Yes |
| Order-independent | Should be | Can be order-dependent |
| Where implemented | Separate WASM module | Inside Core reducer |

### Permission Example
To restrict task deletion to the creator, add a `creator: Did` field to the task
and check `event.from != task.creator` in the reducer's delete handler.

## CO Participant Management

Participants are managed via the co-core-co root Core:
- `CoAction::ParticipantInvite { participant: DID, tags }` - invite a DID
- Participant states: Active, Invite, Join, etc.
- Tags can store metadata (e.g., display name)

### CO Join/Invite Configuration
Per-CO tags control join behavior:
- `invite` - Only accept joins from invited participants
- `accept` - Auto-accept all join requests
- `did` - Accept when DID meets verification criteria
- `manual` - Add as pending participant
- `disable` - Reject all requests

## Source Map

- docs/src/reference/identity.md (primary: DID concept, usage, wallet, networking)
- docs/src/reference/guards.md (primary: Guard concept, built-in Guards, PoA diagram)
- docs/src/reference/permissions.md (primary: Permission concept, Guard vs Permission comparison)
- docs/src/getting-started/next-steps.md (example: permission implementation in Core)
- docs/src/usage/api-overview-core.md (Guard trait signature)
- docs/src/usage/configuration.md (join/invite settings)
