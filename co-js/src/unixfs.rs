use crate::{
	js::{from_js_value, to_js_value},
	JsBlockStorage,
};
use co_primitives::unixfs_add;
use futures::{StreamExt, TryStreamExt};
use wasm_bindgen::prelude::*;
use web_sys::console;

/// Add stream as unixfs file to storage.
/// The last CID in the result is the root.
#[wasm_bindgen(js_name = "unixfsAdd", unchecked_return_type = "Promise<UInt8Array[]>")]
pub async fn js_unixfs_add(storage: &JsBlockStorage, stream: web_sys::ReadableStream) -> Result<JsValue, JsValue> {
	let mut async_stream = wasm_streams::ReadableStream::from_raw(stream)
		.try_into_stream()
		.map_err(|err| format!("Error converting stream: {:?}", err))?
		.map_err(|err| futures::io::Error::new(std::io::ErrorKind::Other, format!("{:?}", err)))
		.map(|v| {
			Ok(from_js_value::<Vec<u8>>(v?)
				.map_err(|err| futures::io::Error::new(std::io::ErrorKind::Other, format!("{:?}", err)))?)
		})
		.into_async_read();
	console::log_1(&"After Stream maybe?".into());
	let cids = unixfs_add(storage, &mut async_stream)
		.await
		.map_err(|err| format!("unixfs add failed: \n{:?}", err))?;
	console::log_2(&"After Stream maybe?".into(), &to_js_value(&cids).expect("msg"));
	to_js_value(&cids)
}
