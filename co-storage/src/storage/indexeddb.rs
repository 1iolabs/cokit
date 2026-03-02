// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{BlockStorageContentMapping, ExtendedBlockStorage};
use anyhow::anyhow;
use async_trait::async_trait;
use cid::Cid;
use co_actor::{ActorError, ActorHandle, JsLocalTaskSpawner, LocalActor, Response};
use co_primitives::{
	Block, BlockStat, BlockStorage, BlockStorageCloneSettings, BlockStorageStoreParams, CloneWithBlockStorageSettings,
	DefaultParams, StorageError, StoreParams,
};
use js_sys::Uint8Array;
use wasm_bindgen::{prelude::*, JsCast};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
	IdbDatabase, IdbFactory, IdbObjectStore, IdbOpenDbRequest, IdbRequest, IdbTransaction, IdbTransactionMode,
};

const OBJECT_STORE_NAME: &str = "blocks";
const DB_VERSION: u32 = 1;

/// IndexedDB-backed block storage for WASM.
#[derive(Debug, Clone)]
pub struct IndexedDbBlockStorage {
	handle: ActorHandle<IdbMessage>,
}
impl IndexedDbBlockStorage {
	/// Open (or create) an IndexedDB database and return a ready storage handle.
	///
	/// # Args
	/// - `db_name` is the IndexedDB database name, e.g. `"co::my-app"`
	pub async fn new(db_name: impl Into<String>) -> Result<Self, StorageError> {
		let db_name: String = db_name.into();

		// Open the database *before* the actor so we can report init errors directly.
		let db = open_database(&db_name)
			.await
			.map_err(|e| StorageError::Internal(anyhow!("IndexedDB open failed: {:?}", e)))?;

		let instance =
			LocalActor::spawn_with(JsLocalTaskSpawner::default(), Default::default(), IndexedDbActor { db }, ())
				.map_err(|e| StorageError::Internal(anyhow!("actor spawn failed: {:?}", e)))?;

		Ok(Self { handle: instance.handle() })
	}
}
#[async_trait]
impl BlockStorage for IndexedDbBlockStorage {
	async fn get(&self, cid: &Cid) -> Result<Block, StorageError> {
		self.handle
			.request(|response| IdbMessage::Get(*cid, response))
			.await
			.map_err(actor_err)?
	}

	async fn set(&self, block: Block) -> Result<Cid, StorageError> {
		self.handle
			.request(|response| IdbMessage::Set(block, response))
			.await
			.map_err(actor_err)?
	}

	async fn stat(&self, cid: &Cid) -> Result<BlockStat, StorageError> {
		self.handle
			.request(|response| IdbMessage::Stat(*cid, response))
			.await
			.map_err(actor_err)?
	}

	async fn remove(&self, cid: &Cid) -> Result<(), StorageError> {
		self.handle
			.request(|response| IdbMessage::Remove(*cid, response))
			.await
			.map_err(actor_err)?
	}

	fn max_block_size(&self) -> usize {
		<Self as BlockStorageStoreParams>::StoreParams::MAX_BLOCK_SIZE
	}
}
impl BlockStorageStoreParams for IndexedDbBlockStorage {
	type StoreParams = DefaultParams;
}
#[async_trait]
impl ExtendedBlockStorage for IndexedDbBlockStorage {
	async fn set_extended(&self, block: crate::ExtendedBlock) -> Result<Cid, StorageError> {
		self.set(block.block).await
	}

	async fn exists(&self, cid: &Cid) -> Result<bool, StorageError> {
		match self.stat(cid).await {
			Ok(_) => Ok(true),
			Err(StorageError::NotFound(..)) => Ok(false),
			Err(e) => Err(e),
		}
	}

	async fn clear(&self) -> Result<(), StorageError> {
		unimplemented!()
	}
}
impl CloneWithBlockStorageSettings for IndexedDbBlockStorage {
	fn clone_with_settings(&self, _settings: BlockStorageCloneSettings) -> Self {
		self.clone()
	}
}
#[async_trait]
impl BlockStorageContentMapping for IndexedDbBlockStorage {}

#[derive(Debug)]
enum IdbMessage {
	Get(Cid, Response<Result<Block, StorageError>>),
	Set(Block, Response<Result<Cid, StorageError>>),
	Stat(Cid, Response<Result<BlockStat, StorageError>>),
	Remove(Cid, Response<Result<(), StorageError>>),
}

#[derive(Debug)]
struct IndexedDbActor {
	db: IdbDatabase,
}
impl IndexedDbActor {
	fn store(&self, mode: IdbTransactionMode) -> Result<IdbObjectStore, StorageError> {
		let tx: IdbTransaction = self
			.db
			.transaction_with_str_and_mode(OBJECT_STORE_NAME, mode)
			.map_err(|e| StorageError::Internal(anyhow!("IDB transaction failed: {:?}", e)))?;
		tx.object_store(OBJECT_STORE_NAME)
			.map_err(|e| StorageError::Internal(anyhow!("IDB object_store failed: {:?}", e)))
	}

	async fn handle_get(&self, cid: &Cid) -> Result<Block, StorageError> {
		let store = self.store(IdbTransactionMode::Readonly)?;
		let key = JsValue::from_str(&cid.to_string());
		let request: IdbRequest = store
			.get(&key)
			.map_err(|e| StorageError::Internal(anyhow!("IDB get failed: {:?}", e)))?;
		let result = idb_request_await(&request).await?;

		if result.is_undefined() || result.is_null() {
			return Err(StorageError::NotFound(*cid, anyhow!("not found in IndexedDB")));
		}

		let bytes =
			bytes_from_js(&result).map_err(|e| StorageError::Internal(anyhow!("IDB decode failed: {:?}", e)))?;

		// result
		Ok(Block::new_unchecked(*cid, bytes))
	}

	async fn handle_set(&self, block: Block) -> Result<Cid, StorageError> {
		// Enforce block size limit.
		if block.data().len() > <IndexedDbBlockStorage as BlockStorageStoreParams>::StoreParams::MAX_BLOCK_SIZE {
			return Err(StorageError::InvalidArgument(anyhow!(
				"Block size {} exceeds max {}",
				block.data().len(),
				<IndexedDbBlockStorage as BlockStorageStoreParams>::StoreParams::MAX_BLOCK_SIZE
			)));
		}

		let store = self.store(IdbTransactionMode::Readwrite)?;
		let key = JsValue::from_str(&block.cid().to_string());
		let value = bytes_to_js(block.data());
		let request: IdbRequest = store
			.put_with_key(&value, &key)
			.map_err(|e| StorageError::Internal(anyhow!("IDB put failed: {:?}", e)))?;
		idb_request_await(&request).await?;

		// result
		Ok(*block.cid())
	}

	async fn handle_stat(&self, cid: &Cid) -> Result<BlockStat, StorageError> {
		let store = self.store(IdbTransactionMode::Readonly)?;
		let key = JsValue::from_str(&cid.to_string());
		let request: IdbRequest = store
			.get(&key)
			.map_err(|e| StorageError::Internal(anyhow!("IDB get (stat) failed: {:?}", e)))?;
		let result = idb_request_await(&request).await?;

		if result.is_undefined() || result.is_null() {
			return Err(StorageError::NotFound(*cid, anyhow!("not found in IndexedDB")));
		}

		let bytes =
			bytes_from_js(&result).map_err(|e| StorageError::Internal(anyhow!("IDB decode (stat) failed: {:?}", e)))?;

		// result
		Ok(BlockStat { size: bytes.len() as u64 })
	}

	async fn handle_remove(&self, cid: &Cid) -> Result<(), StorageError> {
		let store = self.store(IdbTransactionMode::Readwrite)?;
		let key = JsValue::from_str(&cid.to_string());
		let request: IdbRequest = store
			.delete(&key)
			.map_err(|e| StorageError::Internal(anyhow!("IDB delete failed: {:?}", e)))?;
		idb_request_await(&request).await?;

		// result
		Ok(())
	}
}
impl LocalActor for IndexedDbActor {
	type Message = IdbMessage;
	type State = ();
	type Initialize = ();

	async fn initialize(
		&self,
		_handle: &ActorHandle<Self::Message>,
		_tags: &co_primitives::Tags,
		_initialize: Self::Initialize,
	) -> Result<Self::State, ActorError> {
		Ok(())
	}

	async fn handle(
		&self,
		_handle: &ActorHandle<Self::Message>,
		message: Self::Message,
		_state: &mut Self::State,
	) -> Result<(), ActorError> {
		match message {
			IdbMessage::Get(cid, response) => response.respond(self.handle_get(&cid).await),
			IdbMessage::Set(block, response) => response.respond(self.handle_set(block).await),
			IdbMessage::Stat(cid, response) => response.respond(self.handle_stat(&cid).await),
			IdbMessage::Remove(cid, response) => response.respond(self.handle_remove(&cid).await),
		}
		Ok(())
	}
}

/// Open (or upgrade-create) an IndexedDB database.
async fn open_database(name: &str) -> Result<IdbDatabase, JsValue> {
	let factory: IdbFactory = web_sys::window()
		.ok_or_else(|| JsValue::from_str("no window"))?
		.indexed_db()?
		.ok_or_else(|| JsValue::from_str("indexedDB not available"))?;

	let open_request: IdbOpenDbRequest = factory.open_with_u32(name, DB_VERSION)?;

	// Handle upgrade: create object store if needed.
	let on_upgrade = Closure::once(move |event: web_sys::Event| {
		let request: IdbOpenDbRequest = event.target().unwrap().dyn_into::<IdbOpenDbRequest>().unwrap();
		let db: IdbDatabase = request.result().unwrap().dyn_into::<IdbDatabase>().unwrap();
		if !db.object_store_names().contains(OBJECT_STORE_NAME) {
			db.create_object_store(OBJECT_STORE_NAME).unwrap();
		}
	});
	open_request.set_onupgradeneeded(Some(on_upgrade.as_ref().unchecked_ref()));

	let db_js = idb_request_await(open_request.as_ref())
		.await
		.map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;

	Ok(db_js.dyn_into::<IdbDatabase>()?)
}

/// Await an IdbRequest by wrapping its onsuccess/onerror in a Promise.
async fn idb_request_await(request: &IdbRequest) -> Result<JsValue, StorageError> {
	let promise = js_sys::Promise::new(&mut |resolve, reject| {
		let resolve_cb = Closure::once(move |event: web_sys::Event| {
			let target: IdbRequest = event.target().unwrap().dyn_into().unwrap();
			resolve.call1(&JsValue::NULL, &target.result().unwrap()).unwrap();
		});
		let reject_cb = Closure::once(move |event: web_sys::Event| {
			let target: IdbRequest = event.target().unwrap().dyn_into().unwrap();
			let err: JsValue = target
				.error()
				.ok()
				.flatten()
				.map(JsValue::from)
				.unwrap_or_else(|| JsValue::from_str("unknown IDB error"));
			reject.call1(&JsValue::NULL, &err).unwrap();
		});
		request.set_onsuccess(Some(resolve_cb.as_ref().unchecked_ref()));
		request.set_onerror(Some(reject_cb.as_ref().unchecked_ref()));
		// prevent closures from being dropped before the callbacks fire.
		resolve_cb.forget();
		reject_cb.forget();
	});
	JsFuture::from(promise)
		.await
		.map_err(|e| StorageError::Internal(anyhow!("IDB request failed: {:?}", e)))
}

/// Convert a JS value (Uint8Array or ArrayBuffer) to `Vec<u8>`.
fn bytes_from_js(value: &JsValue) -> Result<Vec<u8>, JsValue> {
	if let Ok(arr) = value.clone().dyn_into::<Uint8Array>() {
		return Ok(arr.to_vec());
	}
	if let Ok(buf) = value.clone().dyn_into::<js_sys::ArrayBuffer>() {
		return Ok(Uint8Array::new(&buf).to_vec());
	}
	Err(JsValue::from_str("expected Uint8Array or ArrayBuffer"))
}

/// Convert a byte slice to a JS `Uint8Array`.
fn bytes_to_js(data: &[u8]) -> JsValue {
	Uint8Array::from(data).into()
}

fn actor_err(err: ActorError) -> StorageError {
	StorageError::Internal(anyhow!("IndexedDB actor error: {:?}", err))
}

#[cfg(test)]
mod tests {
	use super::*;
	use co_primitives::BlockStorageExt;
	use wasm_bindgen_test::*;

	wasm_bindgen_test_configure!(run_in_browser);

	#[wasm_bindgen_test]
	async fn set_get_roundtrip() {
		let storage = IndexedDbBlockStorage::new("co::test-roundtrip").await.expect("open db");
		let cid = storage.set_serialized(&42i32).await.expect("set");
		let value: i32 = storage.get_deserialized(&cid).await.expect("get");
		assert_eq!(value, 42);
	}

	#[wasm_bindgen_test]
	async fn stat_returns_size() {
		let storage = IndexedDbBlockStorage::new("co::test-stat").await.expect("open db");
		let data = vec![1u8, 2, 3, 4, 5];
		let block = Block::new_data(co_primitives::KnownMultiCodec::Raw, data);
		let cid = storage.set(block).await.expect("set");
		let stat = storage.stat(&cid).await.expect("stat");
		assert_eq!(stat.size, 5);
	}

	#[wasm_bindgen_test]
	async fn remove_deletes_block() {
		let storage = IndexedDbBlockStorage::new("co::test-remove").await.expect("open db");
		let block = Block::new_data(co_primitives::KnownMultiCodec::Raw, vec![42u8]);
		let cid = storage.set(block).await.expect("set");

		// Should exist.
		storage.get(&cid).await.expect("get before remove");

		// Remove.
		storage.remove(&cid).await.expect("remove");

		// Should be gone.
		let err = storage.get(&cid).await.unwrap_err();
		assert!(matches!(err, StorageError::NotFound(..)));
	}

	#[wasm_bindgen_test]
	async fn remove_missing_is_ok() {
		let storage = IndexedDbBlockStorage::new("co::test-remove-missing").await.expect("open db");
		let cid: Cid = "bafyr4igf663hpuvdpvque42uxmkbacg5ubd4cgageulmwmqo33g2tpod7e".parse().unwrap();
		// removing a non-existent key should succeed (IDB delete is idempotent).
		storage.remove(&cid).await.expect("remove missing should be ok");
	}

	#[wasm_bindgen_test]
	async fn oversize_block_rejected() {
		let storage = IndexedDbBlockStorage::new("co::test-oversize").await.expect("open db");
		let data = vec![0u8; <IndexedDbBlockStorage as BlockStorageStoreParams>::StoreParams::MAX_BLOCK_SIZE + 1];
		let block = Block::new_data(co_primitives::KnownMultiCodec::Raw, data);
		let err = storage.set(block).await.unwrap_err();
		assert!(matches!(err, StorageError::InvalidArgument(..)));
	}
}
