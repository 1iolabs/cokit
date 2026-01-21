use serde_wasm_bindgen::Serializer;
use std::any::type_name;
use wasm_bindgen::prelude::*;

pub fn from_js_value<T: serde::de::DeserializeOwned>(value: JsValue) -> Result<T, JsValue> {
	Ok(serde_wasm_bindgen::from_value(value)
		.map_err(|err| format!("convert from `JsValue` to `{}` failed: {}", type_name::<T>(), err))?)
}

pub fn to_js_value<T: serde::Serialize>(value: &T) -> Result<JsValue, JsValue> {
	let serializer = Serializer::new().serialize_maps_as_objects(true);
	Ok(value
		.serialize(&serializer)
		.map_err(|err| format!("convert from `{}` to `JsValue` failed: {}", type_name::<T>(), err))?)
}
