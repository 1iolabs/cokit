use crate::{js::to_js_value, JsBlockStorage};
use co_primitives::unixfs_add;
use wasm_bindgen::prelude::*;

/// Add stream as unixfs file to storage.
/// The last CID in the result is the root.
#[wasm_bindgen(js_name = "unixfsAdd", unchecked_return_type = "Promise<UInt8Array[]>")]
pub async fn js_unixfs_add(storage: &JsBlockStorage, stream: web_sys::ReadableStream) -> Result<JsValue, JsValue> {
	let mut async_stream = wasm_streams::ReadableStream::from_raw(stream).into_async_read();
	let cids = unixfs_add(storage, &mut async_stream)
		.await
		.map_err(|err| format!("unixfs add failed: \n{:?}", err))?;
	to_js_value(&cids)
}
