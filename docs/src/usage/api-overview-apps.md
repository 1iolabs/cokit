# App API Overview (co-sdk)

The [`co-sdk`](/crate/co_sdk/index.html) is the foundation package to create COKIT-based applications.

## `Application`

The `Application` is the main entry point into COKIT for an app.  
Use the `ApplicationBuilder` to initialize an `Application`.

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

For further information, see:
- [co-sdk: Application](/crate/co_sdk/struct.Application.html)
- [co-sdk: ApplicationBuilder](/crate/co_sdk/struct.ApplicationBuilder.html)

### Network

Use [`Application::create_network`](/crate/co_sdk/struct.Application.html#method.create_network) to start the peer-to-peer networking.

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

For further information, see:
- [Network](../reference/network.md)
- [co-sdk: NetworkSettings](/crate/co_sdk/struct.NetworkSettings.html)
- [co-sdk: CoContext: network](/crate/co_sdk/struct.CoContext.html#method.network)

### `CoContext`

[`CoContext`](/crate/co_sdk/struct.CoContext.html) is a clonable handle to most CO related operations.

```rust
use co_sdk::{ApplicationBuilder, NetworkSettings};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let application = ApplicationBuilder::new_with_path("example", "/tmp/example")
        .build()
        .await?;
    application.create_network(NetworkSettings::default()).await?;
    let context: CoContext = application.context();
    let local_co = context.local_co_reducer().await?;
    Ok(())
}
```

For further information, see:
- [co-sdk: CoContext](/crate/co_sdk/struct.CoContext.html)

## `Identity`

An[`Identity`](/crate/co_sdk/trait.Identity.html) represents a DID.  
It allows you to resolve and work with the DID Document.  
[`CoContext::identity_resolver`](/crate/co_sdk/struct.CoContext.html#method.identity_resolver) can be used to obtain the application's identity resolver.

```rust
use co_sdk::{ApplicationBuilder, NetworkSettings, IdentityResolverBox};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let application = ApplicationBuilder::new_with_path("example", "/tmp/example")
        .build()
        .await?;
    application.create_network(NetworkSettings::default()).await?;
    let context: CoContext = application.context();
    let identity_resolver = context.identity_resolver().await?;
    let identity = identity_resolver.resolve("did:key:z6MkqQYSRLzd5w7aeArR6cZ8LmyXnjN4W6W1PLX1oRk7g4VC").await?;
    Ok(())
}
```

For further information, see:
- [Identity](../reference/identity.md)
- [co-identity: Identity](/crate/co_identity/trait.Identity.html)

## `PrivateIdentity`

A [`PrivateIdentity`](/crate/co_sdk/trait.PrivateIdentity.html) represents a DID you own.  
It allows you to sign and encrypt using the identity.  
[`CoContext::private_identity_resolver`](/crate/co_sdk/struct.CoContext.html#method.private_identity_resolver) can be used to obtain the application's private identity resolver.  
The application's private identity resolver will resolve identities that are stored in the Local CO keystore core.

```rust
use co_sdk::{ApplicationBuilder, NetworkSettings, IdentityResolverBox};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let application = ApplicationBuilder::new_with_path("example", "/tmp/example")
        .build()
        .await?;
    application.create_network(NetworkSettings::default()).await?;
    let context: CoContext = application.context();
    let private_identity_resolver = context.private_identity_resolver().await?;
    let private_identity = private_identity_resolver.resolve_private("did:key:z6MkqQYSRLzd5w7aeArR6cZ8LmyXnjN4W6W1PLX1oRk7g4VC").await?;
    Ok(())
}
```

For further information, see:
- [Identity](../reference/identity.md)
- [co-identity: PrivateIdentity](/crate/co_identity/trait.PrivateIdentity.html)

## UnixFS

UnixFS is an IPFS Standard that allows you to store files in a graph.  
COKIT has some built-in primitives to read and write UnixFS files.

For further information, see:
- [UnixFS](https://specs.ipfs.tech/unixfs/)
- [File systems | IPFS Docs](https://docs.ipfs.tech/concepts/file-systems/#unix-file-system-unixfs)

### `unixfs_add`

Add a stream as UnixFS file to a block storage.  
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

For further information, see:
- [co-storage: unixfs_add](/crate/co_storage/fn.unixfs_add.html)

### `unixfs_add_file`

Add a file from a path as a UnixFS file to a block storage.

```rust
use co_sdk::{MemoryBlockStorage, unixfs_add};

#[tokio::main]
async fn main() {
    let storage = MemoryBlockStorage::default();
		let cids = unixfs_add_file(&storage, "/tmp/image.png").await.unwrap();
		println!("CIDs: {:?}", cids);
}
```

For further information, see:
- [co-storage: unixfs_add_file](/crate/co_storage/fn.unixfs_add_file.html)

### `unixfs_cat_buffer`

Read a UnixFS file into a buffer.

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

For further information, see:
- [co-storage: unixfs_cat_buffer](/crate/co_storage/fn.unixfs_cat_buffer.html)

### `unixfs_encode_buffer`

Encode a buffer into blocks in memory.

```rust
use co_sdk::{Block, unixfs_encode_buffer};

#[tokio::test]
async fn main() {
	let data = "hello world";
	let blocks: Vec<Block<P>> = unixfs_encode_buffer(data.as_bytes()).await.unwrap();
}
```

For further information, see:
- [co-storage: unixfs_encode_buffer](/crate/co_storage/fn.unixfs_cat_buffer.html)

### `unixfs_stream`

Read UnixFS file as a chunked futures stream.

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

For further information, see:
- [co-storage: unixfs_stream](/crate/co_storage/fn.unixfs_cat_buffer.html)

## `CoReducer`

CO handle.  
This is used to interact with a CO.  
It can be obtained from the [`CoContext`](/crate/co_sdk/struct.CoContext.html).

```rust
use co_sdk::{ApplicationBuilder, NetworkSettings, IdentityResolverBox, DidKeyIdentity, CO_CORE_NAME_KEYSTORE, DidKeyProvider, CreateCo};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let application = ApplicationBuilder::new_with_path("example", "/tmp/example")
        .build()
        .await?;
    application.create_network(NetworkSettings::default()).await?;
    let context: CoContext = application.context();

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

For further information, see:
- [co-sdk: CoReducer](/crate/co_sdk/struct.CoReducer.html)

### `reducer_state`

Get current reducer state and heads.

For further information, see:
- [co-sdk: CoReducer: reducer_state](/crate/co_sdk/struct.CoReducer.html#method.reducer_state)

### `push`

Push an action into a Core.

For further information, see:
- [co-sdk: CoReducer: push](/crate/co_sdk/struct.CoReducer.html#method.push)

### `join`

Join heads into the CO.

For further information, see:
- [co-sdk: CoReducer: join](/crate/co_sdk/struct.CoReducer.html#method.join)

## `CoStorage`

A storage instance that belongs to a CO.  
It can be obtained from an opened CO instance.

For further information, see:
- [Storage](../reference/storage.md)
- [co-sdk: CoStorage](/crate/co_sdk/struct.CoStorage.html)
- [co-sdk: CoReducer: storage](/crate/co_sdk/struct.CoReducer.html#method.storage)

## `Cores`

Registry for built-in Cores.

For further information, see:
- [co-sdk: Cores](/crate/co_sdk/struct.Cores.html)

### `Guards`

Registry for built-in Guards.

For further information, see:
- [co-sdk: Guards](/crate/co_sdk/struct.Guards.html)

## References
- [co-sdk](/crate/co_sdk/index.html)
- [Glossary: CID](../glossary/glossary.md#cid)
