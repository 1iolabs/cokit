// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::actor::JsLocalTaskSpawner;
use anyhow::anyhow;
use async_trait::async_trait;
use cid::Cid;
use co_actor::{ActorError, ActorHandle, LocalActor, Response};
use co_primitives::{Block, BlockStorage, DefaultParams, StorageError, StoreParams};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::js_sys::{Function, Promise, Uint8Array};

#[wasm_bindgen]
extern "C" {
	#[wasm_bindgen(typescript_type = "(cid: Uint8Array) => Promise<Uint8Array | undefined>")]
	pub type JsBlockStorageGet;

	#[wasm_bindgen(typescript_type = "(cid: Uint8Array, data: Uint8Array) => void")]
	pub type JsBlockStorageSet;
}

#[wasm_bindgen(js_name = "BlockStorage")]
#[derive(Debug, Clone)]
pub struct JsBlockStorage {
	handle: ActorHandle<JsBlockStorageMessage>,
}
#[wasm_bindgen(js_class = "BlockStorage")]
impl JsBlockStorage {
	#[wasm_bindgen(constructor)]
	pub fn new(get: JsBlockStorageGet, set: JsBlockStorageSet) -> Result<Self, JsValue> {
		Ok(Self {
			handle: LocalActor::spawn_with(
				JsLocalTaskSpawner::default(),
				Default::default(),
				JsBlockStorageActor { get: get.dyn_into()?, set: set.dyn_into()? },
				(),
			)
			.map_err(|err| format!("block storage failed: {:?}", err))?
			.handle(),
		})
	}
}
#[async_trait]
impl BlockStorage for JsBlockStorage {
	async fn get(&self, cid: &Cid) -> Result<Block, StorageError> {
		Ok(self
			.handle
			.request(|response| JsBlockStorageMessage::Get(*cid, response))
			.await
			.map_err(|err| StorageError::Internal(err.into()))??)
	}

	async fn set(&self, block: Block) -> Result<Cid, StorageError> {
		Ok(self
			.handle
			.request(|response| JsBlockStorageMessage::Set(block, response))
			.await
			.map_err(|err| StorageError::Internal(err.into()))??)
	}

	async fn remove(&self, _cid: &Cid) -> Result<(), StorageError> {
		Err(StorageError::Internal(anyhow!("Unsupported")))
	}

	fn max_block_size(&self) -> usize {
		DefaultParams::MAX_BLOCK_SIZE
	}
}

enum JsBlockStorageMessage {
	Get(Cid, Response<Result<Block, StorageError>>),
	Set(Block, Response<Result<Cid, StorageError>>),
}

#[derive(Debug)]
struct JsBlockStorageActor {
	/// Typescript: ```typescript
	/// (cid: Uint8Array) => Promise<Uint8Array | undefined>
	/// ```
	get: Function,

	/// Typescript: ```typescript
	/// (cid: Uint8Array, data: Uint8Array) => void
	/// ```
	set: Function,
}
impl JsBlockStorageActor {
	async fn get(&self, cid: &Cid) -> Result<Block, StorageError> {
		let this = JsValue::null();
		let js_cid =
			serde_wasm_bindgen::to_value(cid).map_err(|err| anyhow!("Convert `Cid` to `JsValue` failed: {}", err))?;
		let call: JsValue = self.get.call1(&this, &js_cid).map_err(|err| anyhow!("Call error: {:?}", err))?;
		let promise: Promise = call
			.dyn_into::<Promise>()
			.map_err(|value| anyhow!("Result is not a `Promise`: {:?}", value))?;
		let future = JsFuture::from(promise);
		let result = future.await.map_err(|err| anyhow!("Get block failed: {:?}", err))?;
		if result.is_null_or_undefined() {
			return Err(StorageError::NotFound(*cid, anyhow!("Getter returned undefined")));
		}
		let bytes = result
			.clone()
			.dyn_into::<Uint8Array>()
			.map_err(|err| anyhow!("Failed to convert result to Uint8Array: {:?}", err))?;
		Ok(Block::new(*cid, bytes.to_vec()).map_err(|err| anyhow!("Data and Cid are not compatible: {:?}", err))?)
	}

	async fn set(&self, block: Block) -> Result<Cid, StorageError> {
		let this = JsValue::null();
		let (cid, data) = block.into_inner();
		let js_cid =
			serde_wasm_bindgen::to_value(&cid).map_err(|err| anyhow!("Convert `Cid` to `JsValue` failed: {}", err))?;
		let js_data = Uint8Array::from(data.as_ref());
		let call = self
			.set
			.call2(&this, &js_cid, &js_data)
			.map_err(|err| anyhow!("Call error: {:?}", err))?;
		let promise: Promise = call
			.dyn_into::<Promise>()
			.map_err(|value| anyhow!("Result is not a `Promise`: {:?}", value))?;
		let future = JsFuture::from(promise);
		let result = future.await.map_err(|err| anyhow!("Set block failed: {:?}", err))?;
		let cid_bytes = result
			.dyn_into::<Uint8Array>()
			.map_err(|err| anyhow!("Convert storage set result JsValue to Cid failed: {:?}", err.as_string()))?
			.to_vec();
		let storage_set_cid = Cid::try_from(cid_bytes).map_err(|err| anyhow!("Get Cid from bytes failed: {}", err))?;
		Ok(storage_set_cid)
	}
}
impl LocalActor for JsBlockStorageActor {
	type Message = JsBlockStorageMessage;
	type State = ();
	type Initialize = ();

	async fn handle(
		&self,
		_handle: &ActorHandle<Self::Message>,
		message: Self::Message,
		_state: &mut Self::State,
	) -> Result<(), ActorError> {
		match message {
			JsBlockStorageMessage::Get(cid, response) => {
				response.respond(self.get(&cid).await);
			},
			JsBlockStorageMessage::Set(block, response) => {
				response.respond(self.set(block).await);
			},
		}
		Ok(())
	}

	async fn initialize(
		&self,
		_handle: &ActorHandle<Self::Message>,
		_tags: &co_primitives::Tags,
		_initialize: Self::Initialize,
	) -> Result<Self::State, co_actor::ActorError> {
		Ok(())
	}
}
