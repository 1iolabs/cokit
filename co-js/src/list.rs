// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::{from_js_value, to_js_value, JsBlockStorage};
use cid::Cid;
use co_primitives::{CoList, CoListTransaction, TagValue};
use wasm_bindgen::prelude::*;
use web_sys::js_sys::Uint8Array;

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

	pub async fn open(&self, storage: &JsBlockStorage) -> Result<JsCoListTransaction, JsValue> {
		let transaction = self
			.list()
			.open(storage)
			.await
			.map_err(|err| format!("Open failed: {:?}", err))?;
		Ok(JsCoListTransaction(transaction))
	}

	pub async fn commit(&mut self, transaction: JsCoListTransaction) -> Result<(), JsValue> {
		let mut list = self.list();
		list.commit(transaction.0)
			.await
			.map_err(|err| format!("Commit failed: {:?}", err))?;
		self.root = Into::<Option<Cid>>::into(&list);
		Ok(())
	}

	pub async fn push(&mut self, storage: &JsBlockStorage, value: JsValue) -> Result<(), JsValue> {
		let mut transaction = self.open(storage).await?;
		transaction.push(value).await?;
		self.commit(transaction).await?;
		Ok(())
	}

	pub async fn pop(&mut self, storage: &JsBlockStorage) -> Result<Option<JsValue>, JsValue> {
		let mut transaction = self.open(storage).await?;
		let result = transaction.pop().await?;
		self.commit(transaction).await?;
		Ok(result)
	}

	pub async fn pop_front(&mut self, storage: &JsBlockStorage) -> Result<Option<JsValue>, JsValue> {
		let mut transaction = self.open(storage).await?;
		let result = transaction.pop_front().await?;
		self.commit(transaction).await?;
		Ok(result)
	}

	pub fn stream(&self, storage: &JsBlockStorage) -> web_sys::ReadableStream {
		let list = self.list();
		let storage = storage.clone();
		let stream = async_stream::try_stream! {
			let list = list.open(&storage).await
				.map_err(|err| format!("Open failed: {:?}", err))?;
			let stream = list.stream();
			for await item in stream {
				let (_, value) = item
					.map_err(|err| format!("Read failed: {:?}", err))?;
				let js_value = to_js_value(&value)?;
				yield js_value;
			}
		};
		wasm_streams::ReadableStream::from_stream(stream).into_raw()
	}

	pub fn reverse_stream(&self, storage: &JsBlockStorage) -> web_sys::ReadableStream {
		let list = self.list();
		let storage = storage.clone();
		let stream = async_stream::try_stream! {
			let list = list.open(&storage).await
				.map_err(|err| format!("Open failed: {:?}", err))?;
			let stream = list.reverse_stream();
			for await item in stream {
				let (_, value) = item
					.map_err(|err| format!("Read failed: {:?}", err))?;
				let js_value = to_js_value(&value)?;
				yield js_value;
			}
		};
		wasm_streams::ReadableStream::from_stream(stream).into_raw()
	}

	pub fn cid(&self) -> Result<Option<Uint8Array>, JsValue> {
		self.root.as_ref().map(|cid| to_js_value(cid).map(Uint8Array::from)).transpose()
	}
}
impl JsCoList {
	fn list(&self) -> CoList<TagValue> {
		CoList::from(self.root)
	}
}
impl From<Option<Cid>> for JsCoList {
	fn from(value: Option<Cid>) -> Self {
		Self { root: value }
	}
}
impl From<CoList<TagValue>> for JsCoList {
	fn from(value: CoList<TagValue>) -> Self {
		Self { root: Into::<Option<Cid>>::into(&value) }
	}
}

#[wasm_bindgen(js_name = "CoListTransaction")]
pub struct JsCoListTransaction(CoListTransaction<JsBlockStorage, TagValue>);

#[wasm_bindgen(js_class = "CoListTransaction")]
impl JsCoListTransaction {
	pub async fn store(&mut self) -> Result<JsCoList, JsValue> {
		let list = self.0.store().await.map_err(|err| format!("Store failed: {:?}", err))?;
		Ok(list.into())
	}

	pub async fn push(&mut self, value: JsValue) -> Result<(), JsValue> {
		let value: TagValue = from_js_value(value)?;
		self.0.push(value).await.map_err(|err| format!("Push failed: {:?}", err))?;
		Ok(())
	}

	pub async fn pop(&mut self) -> Result<Option<JsValue>, JsValue> {
		if let Some((_, value)) = self.0.pop().await.map_err(|err| format!("Pop failed: {:?}", err))? {
			return Some(to_js_value(&value)).transpose();
		}
		Ok(None)
	}

	pub async fn pop_front(&mut self) -> Result<Option<JsValue>, JsValue> {
		if let Some((_, value)) = self.0.pop_front().await.map_err(|err| format!("Pop front failed: {:?}", err))? {
			return Some(to_js_value(&value)).transpose();
		}
		Ok(None)
	}

	pub fn stream(&self) -> web_sys::ReadableStream {
		let transaction = self.0.clone();
		let stream = async_stream::try_stream! {
			let stream = transaction.stream();
			for await item in stream {
				let (_, value) = item
					.map_err(|err| format!("Read failed: {:?}", err))?;
				let js_value = to_js_value(&value)?;
				yield js_value;
			}
		};
		wasm_streams::ReadableStream::from_stream(stream).into_raw()
	}

	pub fn reverse_stream(&self) -> web_sys::ReadableStream {
		let transaction = self.0.clone();
		let stream = async_stream::try_stream! {
			let stream = transaction.reverse_stream();
			for await item in stream {
				let (_, value) = item
					.map_err(|err| format!("Read failed: {:?}", err))?;
				let js_value = to_js_value(&value)?;
				yield js_value;
			}
		};
		wasm_streams::ReadableStream::from_stream(stream).into_raw()
	}
}
