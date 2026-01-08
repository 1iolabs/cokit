use crate::{from_js_value, to_js_value, JsBlockStorage};
use cid::Cid;
use co_primitives::{CoSet, TagValue};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = "CoSet")]
pub struct JsCoSet {
	root: Option<Cid>,
}

#[wasm_bindgen(js_class = "CoSet")]
impl JsCoSet {
	#[wasm_bindgen(constructor)]
	pub fn new(cid: JsValue) -> Result<Self, JsValue> {
		let root = if cid.is_null_or_undefined() { None } else { Some(from_js_value(cid)?) };
		Ok(JsCoSet { root })
	}

	#[allow(clippy::should_implement_trait)]
	pub fn default() -> Self {
		JsCoSet { root: None }
	}

	pub fn is_empty(&self) -> bool {
		self.set().is_empty()
	}

	pub async fn contains(&self, storage: &JsBlockStorage, value: JsValue) -> Result<bool, JsValue> {
		let set = self
			.set()
			.open(storage)
			.await
			.map_err(|err| format!("open failed: {:?}", err))?;
		let value: TagValue = from_js_value(value)?;
		Ok(set
			.contains(&value)
			.await
			.map_err(|err| format!("contains failed: {:?}", err))?)
	}

	pub async fn insert(&mut self, storage: &JsBlockStorage, value: JsValue) -> Result<(), JsValue> {
		let mut set = self
			.set()
			.open(storage)
			.await
			.map_err(|err| format!("open failed: {:?}", err))?;
		let value: TagValue = from_js_value(value)?;
		set.insert(value).await.map_err(|err| format!("insert failed: {:?}", err))?;
		let set = set.store().await.map_err(|err| format!("store failed: {:?}", err))?;
		self.root = Into::<Option<Cid>>::into(&set);
		Ok(())
	}

	pub async fn remove(&mut self, storage: &JsBlockStorage, value: JsValue) -> Result<JsValue, JsValue> {
		let mut set = self
			.set()
			.open(storage)
			.await
			.map_err(|err| format!("open failed: {:?}", err))?;
		let value: TagValue = from_js_value(value)?;
		let removed = set.remove(value).await.map_err(|err| format!("remove failed: {:?}", err))?;
		let set = set.store().await.map_err(|err| format!("store failed: {:?}", err))?;
		self.root = Into::<Option<Cid>>::into(&set);
		to_js_value(&removed)
	}

	pub fn stream(&self, storage: &JsBlockStorage) -> web_sys::ReadableStream {
		let set = self.set();
		let storage = storage.clone();
		let stream = async_stream::try_stream! {
			let tree = set.open(&storage).await
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
impl JsCoSet {
	fn set(&self) -> CoSet<TagValue> {
		CoSet::from(self.root)
	}
}
