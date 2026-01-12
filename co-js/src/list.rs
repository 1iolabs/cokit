use crate::{from_js_value, to_js_value, JsBlockStorage};
use cid::Cid;
use co_primitives::{CoList, TagValue};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = "CoList")]
pub struct JsCoList {
	root: Option<Cid>,
}

#[wasm_bindgen(js_class = "CoList")]
impl JsCoList {
	#[wasm_bindgen(constructor)]
	pub fn new(cid: JsValue) -> Result<Self, JsValue> {
		let root = if cid.is_null_or_undefined() { None } else { Some(from_js_value(cid)?) };
		Ok(JsCoList { root })
	}

	#[allow(clippy::should_implement_trait)]
	pub fn default() -> Self {
		JsCoList { root: None }
	}

	pub async fn pop(&mut self, storage: &JsBlockStorage) -> Result<JsValue, JsValue> {
		let mut list = self
			.list()
			.open(storage)
			.await
			.map_err(|err| format!("open failed: {:?}", err))?;
		let result = list.pop().await.map_err(|err| format!("contains failed: {:?}", err))?;
		let list = list.store().await.map_err(|err| format!("store failed: {:?}", err))?;
		self.root = Into::<Option<Cid>>::into(&list);
		if let Some((_, value)) = result {
			return to_js_value(&value);
		}
		Ok(JsValue::null())
	}
}
impl JsCoList {
	fn list(&self) -> CoList<TagValue> {
		CoList::from(self.root)
	}
}
