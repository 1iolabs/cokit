# API Overview apps (co-sdk)

The `co-sdk` is the foundation package to create CO-kit-based applications.

## `Application`

The `Application` is the main entry point into CO-kit for an app.
Use the `ApplicationBuilder` to initialize a `Application`.

```rust
use co_sdk::ApplicationBuilder;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let application = ApplicationBuilder::new_with_path("example", "/tmp/example")
        .build()
        .await?;
    // ...
    Ok(())
}
```

For further information see:
- [co-sdk: Application](/crate/co_sdk/struct.Application.html)
- [co-sdk: ApplicationBuilder](/crate/co_sdk/struct.ApplicationBuilder.html)

### Network

Use `Application::create_network` to start the peer-to-peer networking.

```rust
use co_sdk::{ApplicationBuilder, NetworkSettings};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let application = ApplicationBuilder::new_with_path("example", "/tmp/example")
        .build()
        .await?;
    application.create_network(NetworkSettings::default()).await?;
    Ok(())
}
```

### `CoContext`

`CoContext` is a clonable handle to most CO related operations.

```rust
use co_sdk::{ApplicationBuilder, NetworkSettings};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let application = ApplicationBuilder::new_with_path("example", "/tmp/example")
        .build()
        .await?;
    application.create_network(NetworkSettings::default()).await?;
    let context = application.context();
    let local_co = context.local_co_reducer().await?;
    Ok(())
}
```

For further information see:
- [co-sdk: CoContext](/crate/co_sdk/struct.CoContext.html)

## `Identity`

A `Identity` represents a DID.
It allows to resolve and work with the DID Document.
`CoContext::identity_resolver` can be used to obtain the applications identity resolver.

```rust
use co_sdk::{ApplicationBuilder, NetworkSettings, IdentityResolverBox};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let application = ApplicationBuilder::new_with_path("example", "/tmp/example")
        .build()
        .await?;
    application.create_network(NetworkSettings::default()).await?;
    let context = application.context();
    let identity_resolver = context.identity_resolver().await?;
    let identity = identity_resolver.resolve("did:key:z6MkqQYSRLzd5w7aeArR6cZ8LmyXnjN4W6W1PLX1oRk7g4VC").await?;
    Ok(())
}
```

## `PrivateIdentity`

A `PrivateIdentity` represents a DID you own.
It allows to sign and encrypt using the identity.
`CoContext::private_identity_resolver` can be used to obtain the applications private identity resolver.
The applications private identity resolver will resolve identities which are stored in the Local CO keystore core.

```rust
use co_sdk::{ApplicationBuilder, NetworkSettings, IdentityResolverBox};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let application = ApplicationBuilder::new_with_path("example", "/tmp/example")
        .build()
        .await?;
    application.create_network(NetworkSettings::default()).await?;
    let context = application.context();
    let private_identity_resolver = context.private_identity_resolver().await?;
    let private_identity = private_identity_resolver.resolve_private("did:key:z6MkqQYSRLzd5w7aeArR6cZ8LmyXnjN4W6W1PLX1oRk7g4VC").await?;
    Ok(())
}
```

## UnixFS

UnixFS is a IPFS Standard which allows to store files in a graph.
CO-kit got some built-in primitives to read and write UnixFS files.

For further information see:
- [UnixFS](https://specs.ipfs.tech/unixfs/)
- [File systems | IPFS Docs](https://docs.ipfs.tech/concepts/file-systems/#unix-file-system-unixfs)

### `unixfs_add`

Add stream as unixfs file to a block storage.
The last CID in the result is the root.

```rust
use co_sdk::{MemoryBlockStorage, unixfs_add};
use futures::io::Cursor;

#[tokio::main]
async fn main() {
    let storage = MemoryBlockStorage::default();
    let mut stream = Cursor::new("hello world".as_bytes().to_vec());
		let cids = unixfs_add(&storage, &mut stream).await.unwrap();
		println!("CIDs: {:?}", cids);
}
```

### `unixfs_add_file`

Add a file from path as unixfs file to a block storage.

```rust
use co_sdk::{MemoryBlockStorage, unixfs_add};

#[tokio::main]
async fn main() {
    let storage = MemoryBlockStorage::default();
		let cids = unixfs_add_file(&storage, "/tmp/image.png").await.unwrap();
		println!("CIDs: {:?}", cids);
}
```

### `unixfs_cat_buffer`

Read unixfs file into buffer.

```rust
use co_sdk::{MemoryBlockStorage, unixfs_add, unixfs_cat_buffer};
use futures::io::Cursor;

#[tokio::test]
async fn main() {
	let storage = MemoryBlockStorage::default();

	// create UnixFS blocks
	let data = "hello world";
	let mut stream = Cursor::new(data.as_bytes().to_vec());
	let cids = unixfs_add(&storage, &mut stream).await.unwrap();

	// read
	let buffer = unixfs_cat_buffer(&storage, cids.last().unwrap()).await.unwrap();
	assert_eq!(data.as_bytes().to_vec(), buffer);
}
```

### `unixfs_encode_buffer`

Encode buffer into blocks in memory.

```rust
use co_sdk::{Block, unixfs_encode_buffer};

#[tokio::test]
async fn main() {
	let data = "hello world";
	let blocks: Vec<Block<P>> = unixfs_encode_buffer(data.as_bytes()).await.unwrap();
}
```

### `unixfs_stream`

Read unixfs file as a chunked futures stream.

```rust
use co_sdk::{unixfs_add, unixfs_stream, MemoryBlockStorage};
use futures::{io::Cursor, TryStreamExt};

#[tokio::test]
async fn main() {
	let storage = MemoryBlockStorage::default();

	// create UnixFS blocks
	let data = "hello world";
	let mut stream = Cursor::new(data.as_bytes().to_vec());
	let cids = unixfs_add(&storage, &mut stream).await.unwrap();

	// read
	let chunks: Vec<Vec<u8>> = unixfs_stream(&storage, cids.last().unwrap()).try_collect().await.unwrap();
	let buffer: Vec<u8> = chunks.concat();
	assert_eq!(data.as_bytes().to_vec(), buffer);
}
```

## `CoReducer`

CO handle.
This is used to interact with a CO.
It can be obtained from the `CoContext`.

```rust
use co_sdk::{ApplicationBuilder, NetworkSettings, IdentityResolverBox, DidKeyIdentity, CO_CORE_NAME_KEYSTORE, DidKeyProvider, CreateCo};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let application = ApplicationBuilder::new_with_path("example", "/tmp/example")
        .build()
        .await?;
    application.create_network(NetworkSettings::default()).await?;
    let context = application.context();

    // create a identity
    let identity = DidKeyIdentity::generate(None);
    let local_co = application.local_co_reducer().await?;
    let provider = DidKeyProvider::new(local_co.clone(), CO_CORE_NAME_KEYSTORE);
    provider.store(&identity, None).await?;

    // create a CO named "co" with a random CoId
    let co: CoReducer = application
    		.create_co(identity.clone(), CreateCo::generate("co".to_owned()))
    		.await?;

    Ok(())
}
```

### `reducer_state`

Get current reducer state and heads.

For further information see:
- [co-sdk: CoReducer: reducer_state](/crate/co_sdk/struct.CoReducer.html#method.reducer_state)

### `push`

Push a action into a Core.

For further information see:
- [co-sdk: CoReducer: push](/crate/co_sdk/struct.CoReducer.html#method.push)

### `join`

Join heads into the CO.

For further information see:
- [co-sdk: CoReducer: join](/crate/co_sdk/struct.CoReducer.html#method.join)

## `CoStorage`

A storage instance that belongs to a CO.
It can be obtained from a opened co instance.

For further information see:
- [Storage](../reference/storage.md)
- [co-sdk: CoStorage](/crate/co_sdk/struct.CoStorage.html)
- [co-sdk: CoReducer: storage](/crate/co_sdk/struct.CoReducer.html#method.storage)

## `Cores`

Registry for built-in cores.

For further information see:
- [co-sdk: Cores](/crate/co_sdk/struct.Cores.html)

### `Guards`

Registry for built-in guards.

For further information see:
- [co-sdk: Guards](/crate/co_sdk/struct.Guards.html)

## References
- [co-sdk](/crate/co_sdk/index.html)
- [Glossary: CID](../glossary/glossary.md#cid)
