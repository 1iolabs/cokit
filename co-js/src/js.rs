// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

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
