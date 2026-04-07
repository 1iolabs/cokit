// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::{dynamic_value::DynamicValue, from_js_value, to_js_value, JsBlockStorage};
use cid::Cid;
use co_primitives::{CoSet, CoSetTransaction};
use wasm_bindgen::prelude::*;
use web_sys::js_sys::Uint8Array;

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

	pub async fn open(&self, storage: &JsBlockStorage) -> Result<JsCoSetTransaction, JsValue> {
		let transaction = self
			.set()
			.open(storage)
			.await
			.map_err(|err| format!("Open failed: {:?}", err))?;
		Ok(JsCoSetTransaction(transaction))
	}
	pub async fn commit(&mut self, transaction: JsCoSetTransaction) -> Result<(), JsValue> {
		let mut set = self.set();
		set.commit(transaction.0)
			.await
			.map_err(|err| format!("Commit failed: {:?}", err))?;
		self.root = Into::<Option<Cid>>::into(&set);
		Ok(())
	}
	pub async fn contains(&self, storage: &JsBlockStorage, value: JsValue) -> Result<bool, JsValue> {
		self.open(storage).await?.contains(value).await
	}

	pub async fn insert(&mut self, storage: &JsBlockStorage, value: JsValue) -> Result<(), JsValue> {
		let mut transaction = self.open(storage).await?;
		transaction.insert(value).await?;
		self.commit(transaction).await?;
		Ok(())
	}

	pub async fn remove(&mut self, storage: &JsBlockStorage, value: JsValue) -> Result<bool, JsValue> {
		let mut transaction = self.open(storage).await?;
		let removed = transaction.remove(value).await?;
		self.commit(transaction).await?;
		Ok(removed)
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

	pub fn cid(&self) -> Result<Option<Uint8Array>, JsValue> {
		self.root.as_ref().map(|cid| to_js_value(cid).map(Uint8Array::from)).transpose()
	}
}
impl JsCoSet {
	fn set(&self) -> CoSet<DynamicValue> {
		CoSet::from(self.root)
	}
}
impl From<Option<Cid>> for JsCoSet {
	fn from(value: Option<Cid>) -> Self {
		Self { root: value }
	}
}
impl From<CoSet<DynamicValue>> for JsCoSet {
	fn from(value: CoSet<DynamicValue>) -> Self {
		Into::<Option<Cid>>::into(&value).into()
	}
}

#[wasm_bindgen(js_name = "CoSetTransaction")]
pub struct JsCoSetTransaction(CoSetTransaction<JsBlockStorage, DynamicValue>);

#[wasm_bindgen(js_class = "CoSetTransaction")]
impl JsCoSetTransaction {
	pub async fn store(&mut self) -> Result<JsCoSet, JsValue> {
		let co_set = self.0.store().await.map_err(|err| format!("Store failed: {:?}", err))?;
		Ok(co_set.into())
	}
	pub async fn contains(&self, key: JsValue) -> Result<bool, JsValue> {
		let key: DynamicValue = from_js_value(key)?;
		Ok(self
			.0
			.contains(&key)
			.await
			.map_err(|err| format!("Contains failed: {:?}", err))?)
	}
	pub async fn insert(&mut self, key: JsValue) -> Result<(), JsValue> {
		let key: DynamicValue = from_js_value(key)?;
		Ok(self.0.insert(key).await.map_err(|err| format!("Insert failed: {:?}", err))?)
	}
	pub async fn remove(&mut self, key: JsValue) -> Result<bool, JsValue> {
		let key: DynamicValue = from_js_value(key)?;
		Ok(self.0.remove(key).await.map_err(|err| format!("Remove failed: {:?}", err))?)
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
