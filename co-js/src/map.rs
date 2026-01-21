use crate::{
	js::{from_js_value, to_js_value},
	JsBlockStorage,
};
use cid::Cid;
use co_primitives::{CoMap, CoMapTransaction, TagValue};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = "CoMap")]
pub struct JsCoMap {
	root: Option<Cid>,
}
#[wasm_bindgen(js_class = "CoMap")]
impl JsCoMap {
	#[wasm_bindgen(constructor)]
	pub fn new(cid: JsValue) -> Result<Self, JsValue> {
		let root = if cid.is_null_or_undefined() { None } else { Some(from_js_value(cid)?) };
		Ok(JsCoMap { root })
	}

	#[allow(clippy::should_implement_trait)]
	pub fn default() -> Self {
		JsCoMap { root: None }
	}

	pub async fn open(&self, storage: &JsBlockStorage) -> Result<JsCoMapTransaction, JsValue> {
		let transaction = self
			.map()
			.open(storage)
			.await
			.map_err(|err| format!("open failed: {:?}", err))?;
		Ok(JsCoMapTransaction(transaction))
	}

	pub async fn commit(&mut self, transaction: JsCoMapTransaction) -> Result<(), JsValue> {
		let mut map = self.map();
		map.commit(transaction.0)
			.await
			.map_err(|err| format!("Commit transaction failed: {:?}", err))?;
		self.root = Into::<Option<Cid>>::into(&map);
		Ok(())
	}

	pub fn is_empty(&self) -> bool {
		self.map().is_empty()
	}

	pub async fn get(&self, storage: &JsBlockStorage, key: JsValue) -> Result<Option<JsValue>, JsValue> {
		let map = self
			.map()
			.open(storage)
			.await
			.map_err(|err| format!("open failed: {:?}", err))?;
		let key: TagValue = from_js_value(key)?;
		let value = map.get(&key).await.map_err(|err| format!("get failed: {:?}", err))?;
		value.as_ref().map(to_js_value).transpose()
	}

	pub async fn contains_key(&self, storage: &JsBlockStorage, key: JsValue) -> Result<bool, JsValue> {
		let map = self
			.map()
			.open(storage)
			.await
			.map_err(|err| format!("open failed: {:?}", err))?;
		let key: TagValue = from_js_value(key)?;
		Ok(map
			.contains_key(&key)
			.await
			.map_err(|err| format!("contains_key failed: {:?}", err))?)
	}

	pub async fn insert(&mut self, storage: &JsBlockStorage, key: JsValue, value: JsValue) -> Result<(), JsValue> {
		let mut map = self
			.map()
			.open(storage)
			.await
			.map_err(|err| format!("open failed: {:?}", err))?;
		let key: TagValue = from_js_value(key)?;
		let value: TagValue = from_js_value(value)?;
		map.insert(key, value)
			.await
			.map_err(|err| format!("insert failed: {:?}", err))?;
		let map = map.store().await.map_err(|err| format!("store failed: {:?}", err))?;
		self.root = Into::<Option<Cid>>::into(&map);
		Ok(())
	}

	pub fn stream(&self, storage: &JsBlockStorage) -> web_sys::ReadableStream {
		let map = self.map();
		let storage = storage.clone();
		let stream = async_stream::try_stream! {
			let tree = map.open(&storage).await
				.map_err(|err| format!("open failed: {:?}", err))?;
			let stream = tree.stream();
			for await item in stream {
				let value = item
					.map_err(|err| format!("read failed: {:?}", err))?;
					let js_value = to_js_value(&value)?;
				yield js_value;
			}
		};
		wasm_streams::ReadableStream::from_stream(stream).into_raw()
	}

	pub fn cid(&self) -> Result<JsValue, JsValue> {
		match &self.root {
			None => Ok(JsValue::null()),
			Some(cid) => to_js_value(cid),
		}
	}
}
impl JsCoMap {
	fn map(&self) -> CoMap<TagValue, TagValue> {
		CoMap::from(self.root)
	}
}
impl From<Option<Cid>> for JsCoMap {
	fn from(value: Option<Cid>) -> Self {
		Self { root: value }
	}
}

#[wasm_bindgen(js_name = "CoMapTransaction")]
pub struct JsCoMapTransaction(CoMapTransaction<JsBlockStorage, TagValue, TagValue>);

#[wasm_bindgen(js_class = "CoMapTransaction")]
impl JsCoMapTransaction {
	pub async fn store(&mut self) -> Result<JsCoMap, JsValue> {
		let co_map = self.0.store().await.map_err(|err| format!("Store failed: {:?}", err))?;
		Ok(Into::<Option<Cid>>::into(&co_map).into())
	}
	pub async fn get(&self, key: JsValue) -> Result<Option<JsValue>, JsValue> {
		let key: TagValue = from_js_value(key)?;
		let result = self.0.get(&key).await.map_err(|err| format!("Get failed: {:?}", err))?;
		result.as_ref().map(to_js_value).transpose()
	}
	pub async fn contains_key(&self, key: JsValue) -> Result<bool, JsValue> {
		let key: TagValue = from_js_value(key)?;
		self.0
			.contains_key(&key)
			.await
			.map_err(|err| format!("Contains key failed: {:?}", err).into())
	}
	pub async fn insert(&mut self, key: JsValue, value: JsValue) -> Result<(), JsValue> {
		let key: TagValue = from_js_value(key)?;
		let value: TagValue = from_js_value(value)?;
		self.0
			.insert(key, value)
			.await
			.map_err(|err| format!("insert failed: {:?}", err))?;
		Ok(())
	}
	pub fn stream(&self) -> web_sys::ReadableStream {
		let transaction = self.0.clone();
		let stream = async_stream::try_stream! {
			let stream = transaction.stream();
			for await item in stream {
				let value = item
					.map_err(|err| format!("read failed: {:?}", err))?;
					let js_value = to_js_value(&value)?;
				yield js_value;
			}
		};
		wasm_streams::ReadableStream::from_stream(stream).into_raw()
	}
}
