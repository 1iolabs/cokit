use crate::{
	js::{from_js_value, to_js_value},
	JsBlockStorage,
};
use cid::Cid;
use co_primitives::{CoMap, TagValue};
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

	pub fn default() -> Self {
		JsCoMap { root: None }
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
		Ok(match value {
			Some(value) => Some(to_js_value(&value)?),
			None => None,
		})
	}

	pub async fn contains(&self, storage: &JsBlockStorage, key: JsValue) -> Result<bool, JsValue> {
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
