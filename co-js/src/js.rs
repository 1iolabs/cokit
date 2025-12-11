use futures::{Stream, TryStreamExt};
use serde::Serialize;
use serde_wasm_bindgen::Serializer;
use std::{any::type_name, cell::RefCell, pin::Pin, rc::Rc};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;
use web_sys::js_sys::Promise;

pub fn from_js_value<T: serde::de::DeserializeOwned>(value: JsValue) -> Result<T, JsValue> {
	Ok(serde_wasm_bindgen::from_value(value)
		.map_err(|err| format!("convert from `JsValue` to `{}` failed: {}", type_name::<T>(), err.to_string()))?)
}

pub fn to_js_value<T: serde::Serialize>(value: &T) -> Result<JsValue, JsValue> {
	let serializer = Serializer::new().serialize_maps_as_objects(true);
	Ok(value
		.serialize(&serializer)
		.map_err(|err| format!("convert from `{}` to `JsValue` failed: {}", type_name::<T>(), err.to_string()))?)
}

#[wasm_bindgen]
pub struct AsyncIteratorStream {
	stream: Rc<RefCell<Pin<Box<dyn Stream<Item = Result<JsValue, JsValue>> + 'static>>>>,
}
impl AsyncIteratorStream {
	pub fn new(stream: impl Stream<Item = Result<JsValue, JsValue>> + 'static) -> Self {
		Self { stream: Rc::new(RefCell::new(Box::pin(stream))) }
	}
}
#[wasm_bindgen]
impl AsyncIteratorStream {
	#[wasm_bindgen(unchecked_return_type = "Promise<{done: boolean, value?: any}>")]
	pub async fn next(&self) -> Promise {
		let stream = self.stream.clone();
		future_to_promise(async move {
			let mut stream = stream.borrow_mut();
			let fut = stream.try_next();
			match fut.await? {
				Some(value) => to_js_value(&AsyncIteratorValue { done: false, value }),
				None => to_js_value(&AsyncIteratorValue { done: true, value: JsValue::undefined() }),
			}
		})
	}
}

#[derive(Debug, Serialize)]
pub struct AsyncIteratorValue {
	pub done: bool,
	#[serde(with = "serde_wasm_bindgen::preserve")]
	pub value: JsValue,
}
