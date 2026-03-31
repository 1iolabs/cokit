# Advanced co-dioxus Operations

These operations go beyond the standard hook-based workflow. Most apps will never need
them — prefer `use_co` + selectors + `dispatch` for all normal state access.

## Table of Contents

- [CoContext Direct Access](#cocontext-direct-access)
- [Joining Unrelated COs](#joining-unrelated-cos)
- [Contact Requests](#contact-requests)
- [Waiting for Initialization](#waiting-for-initialization)
- [Custom CoSettings](#custom-cosettings)
- [Networking Configuration](#networking-configuration)
- [CoBlockStorage Direct Usage](#coblockstorage-direct-usage)

## CoContext Direct Access

`CoContext::try_with_application` gives direct access to the COKIT `Application` on
the COKIT thread. This is a last resort — it bypasses the reactive hook system and
requires manual error handling.

```rust
let context = use_co_context();
let result = context
    .try_with_application(move |application| async move {
        // You now have full Application access on the COKIT thread
        // Do something that hooks can't do
        Ok(some_result)
    })
    .await;
```

The closure runs on the COKIT thread, not the Dioxus UI thread. Results come back
through a oneshot channel.

`CoContextError` variants:
- `CoContextError::Execute(E)` - the closure returned an error
- `CoContextError::Shutdown` - the COKIT Application has shut down

## Joining Unrelated COs

When a CO was not discovered through the standard membership/invite flow (e.g., via
a QR code or deep link containing a DID + CoId):

```rust
let context = use_co_context();
context.join_unrelated_co(
    from_identity,      // your Identity
    to_did,             // the other participant's DID
    to_co_id,           // the CO to join
    to_networks,        // BTreeSet<Network> - how to reach them
).await?;
```

This initiates a join. The membership state will change to `Active` once completed.

## Contact Requests

Send a DIDComm contact request to another DID:

```rust
let context = use_co_context();
context.contact(
    from_identity,      // your Identity
    to_did,             // recipient DID
    to_subject,         // Option<String> - message subject
    to_headers,         // BTreeMap<String, String> - custom headers
    to_networks,        // BTreeSet<Network> - how to reach them
).await?;
```

To handle incoming contact requests, configure a `ContactHandler` on `CoSettings`:

```rust
let settings = CoSettings::new("com.example.app", "my-app")
    .with_contact_handler(MyContactHandler);
```

## Waiting for Initialization

The COKIT Application starts asynchronously. Normally hooks handle this transparently
(selectors suspend until data is ready). For cases where you need to ensure the
Application is ready before proceeding:

```rust
let context = CoContext::new(settings);
// Async wait
context.ready().await?;
// Or blocking wait (native only, not available on web)
context.ready_blocking()?;
```

## Custom CoSettings

Beyond `cli()` and `new()`, `CoSettings` can be constructed from parsed CLI args:

```rust
let cli = Cli::parse();
let settings = CoSettings::from_cli("com.example.app".into(), cli);
```

### Storage backends

```rust
// Filesystem (desktop/mobile)
settings.with_path("/custom/storage/path")

// In-memory (testing, ephemeral)
settings.with_memory()

// IndexedDB (web) - requires a LocalSecret for encryption
settings.with_indexeddb(my_secret)
```

### Registering custom Cores and Guards

```rust
let settings = CoSettings::new("com.example.app", "my-app")
    .with_core(core_cid, core)
    .with_guard(guard_cid, guard_reference);
```

### Access Guards

```rust
let settings = CoSettings::new("com.example.app", "my-app")
    .with_access_guard(MyAccessGuard);
```

The `AccessGuard` trait controls which COs the Application will accept and process.

## Networking Configuration

Enable P2P networking with custom settings:

```rust
use co_sdk::NetworkSettings;

let settings = CoSettings::new("com.example.app", "my-app")
    .with_network(NetworkSettings::default().with_force_new_peer_id(true));
```

Networking is per-feature:
- `desktop` and `mobile` features include networking support
- `web` feature includes networking via WebRTC (if supported)
- Networking can be disabled at runtime via `--no-network` CLI flag

## CoBlockStorage Direct Usage

`CoBlockStorage` implements the `BlockStorage` trait and can be used directly for
low-level block operations:

```rust
use co_sdk::BlockStorage;

let storage = co.storage();

// Get a raw block by CID
let block = storage.get(&cid).await?;

// Store a block
let stored_cid = storage.set(block).await?;

// Check block metadata
let stat = storage.stat(&cid).await?;

// Remove a block
storage.remove(&cid).await?;
```

For typed access, use `BlockStorageExt` from `co_primitives`:

```rust
use co_primitives::BlockStorageExt;

// Resolve a typed Link
let value: MyType = storage.get_value(&link).await?;

// Resolve an OptionLink with default
let value: MyType = storage.get_value_or_default(&option_link).await?;

// Store a value and get a typed Link back
let link: Link<MyType> = storage.set_value(&my_value).await?;
```
