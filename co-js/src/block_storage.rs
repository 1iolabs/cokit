use crate::actor::{JsActor, JsActorHandle};
use anyhow::anyhow;
use async_trait::async_trait;
use cid::Cid;
use co_actor::Response;
use co_primitives::{Block, BlockStorage, DefaultParams, StorageError};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::js_sys::{Function, Promise, Uint8Array};

#[wasm_bindgen(js_name = "BlockStorage")]
#[derive(Debug, Clone)]
pub struct JsBlockStorage {
	handle: JsActorHandle<JsBlockStorageMessage>,
}
#[wasm_bindgen(js_class = "BlockStorage")]
impl JsBlockStorage {
	#[wasm_bindgen(constructor)]
	pub fn new(get: &Function, set: &Function) -> Self {
		Self { handle: JsActor::spawn(JsBlockStorageActor { get: get.clone(), set: set.clone() }) }
	}
}
#[async_trait]
impl BlockStorage for JsBlockStorage {
	type StoreParams = DefaultParams;

	async fn get(&self, cid: &Cid) -> Result<Block<Self::StoreParams>, StorageError> {
		Ok(self
			.handle
			.request(|response| JsBlockStorageMessage::Get(*cid, response))
			.await
			.map_err(|err| StorageError::Internal(err.into()))??)
	}

	async fn set(&self, block: Block<Self::StoreParams>) -> Result<Cid, StorageError> {
		Ok(self
			.handle
			.request(|response| JsBlockStorageMessage::Set(block, response))
			.await
			.map_err(|err| StorageError::Internal(err.into()))??)
	}

	async fn remove(&self, _cid: &Cid) -> Result<(), StorageError> {
		Err(StorageError::Internal(anyhow!("Unsupported")))
	}
}

enum JsBlockStorageMessage {
	Get(Cid, Response<Result<Block<DefaultParams>, StorageError>>),
	Set(Block<DefaultParams>, Response<Result<Cid, StorageError>>),
}

#[derive(Debug)]
struct JsBlockStorageActor {
	/// Typescript: ```typescript
	/// (cid: any) => UInt8Array
	/// ```
	get: Function,

	/// Typescript: ```typescript
	/// (cid: any, data: UInt8Array) => void
	/// ```
	set: Function,
}
impl JsBlockStorageActor {
	async fn get(&self, cid: &Cid) -> Result<Block<DefaultParams>, StorageError> {
		let this = JsValue::null();
		let js_cid = serde_wasm_bindgen::to_value(cid)
			.map_err(|err| anyhow!("Convert `Cid` to `JsValue` failed: {}", err.to_string()))?;
		let call: JsValue = self.get.call1(&this, &js_cid).map_err(|err| anyhow!("Call error: {:?}", err))?;
		let promise: Promise = call
			.dyn_into::<Promise>()
			.map_err(|value| anyhow!("Result is not a `Promise`: {:?}", value))?;
		let future = JsFuture::from(promise);
		let result = future.await.map_err(|err| anyhow!("Get block failed: {:?}", err))?;
		let bytes = result
			.dyn_into::<Uint8Array>()
			.map_err(|err| anyhow!("Failed to convert result to Uint8Array: {:?}", err))?;
		Ok(Block::new_unchecked(*cid, bytes.to_vec()))
	}

	async fn set(&self, block: Block<DefaultParams>) -> Result<Cid, StorageError> {
		let this = JsValue::null();
		let (cid, data) = block.into_inner();
		let js_cid = serde_wasm_bindgen::to_value(&cid)
			.map_err(|err| anyhow!("Convert `Cid` to `JsValue` failed: {}", err.to_string()))?;
		let js_data = Uint8Array::from(data.as_ref());
		let call = self
			.set
			.call2(&this, &js_cid, &js_data)
			.map_err(|err| anyhow!("Call error: {:?}", err))?;
		let promise: Promise = call
			.dyn_into::<Promise>()
			.map_err(|value| anyhow!("Result is not a `Promise`: {:?}", value))?;
		let future = JsFuture::from(promise);
		let _result = future.await.map_err(|err| anyhow!("Set block failed: {:?}", err))?;
		Ok(cid)
	}
}
impl JsActor for JsBlockStorageActor {
	type Message = JsBlockStorageMessage;

	async fn handle(&self, message: Self::Message) {
		match message {
			JsBlockStorageMessage::Get(cid, response) => {
				response.respond(self.get(&cid).await);
			},
			JsBlockStorageMessage::Set(block, response) => {
				response.respond(self.set(block).await);
			},
		}
	}
}
