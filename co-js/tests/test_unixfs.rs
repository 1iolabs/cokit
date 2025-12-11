use cid::Cid;
use co_js::{from_js_value, js_unixfs_add, JsBlockStorage, JsBlockStorageGet, JsBlockStorageSet};
use futures::stream;
use std::{
	collections::BTreeMap,
	sync::{Arc, RwLock},
};
use wasm_bindgen::prelude::Closure;
use wasm_bindgen_futures::future_to_promise;
use wasm_bindgen_test::*;
use web_sys::js_sys::{Promise, Uint8Array};

#[wasm_bindgen_test]
async fn test_unixfs() {
	let blocks: Arc<RwLock<BTreeMap<Cid, Uint8Array>>> = Default::default();
	let get_closure = Closure::wrap(Box::new({
		let blocks = blocks.clone();
		move |cid: Uint8Array| {
			let cid_r: Cid = from_js_value(cid.into()).unwrap();
			let data = blocks.read().unwrap().get(&cid_r).unwrap().clone();
			future_to_promise(async move { Ok(data.into()) })
		}
	}) as Box<dyn FnMut(Uint8Array) -> Promise>);
	let set_closure: Closure<dyn FnMut(Uint8Array, Uint8Array) -> Promise> = Closure::wrap(Box::new({
		let blocks = blocks.clone();
		move |cid, data| {
			let cid_r: Cid = from_js_value(cid.clone().into()).unwrap();
			blocks.write().unwrap().insert(cid_r, data);
			future_to_promise(async move { Ok(cid.into()) })
		}
	}));
	let get: JsBlockStorageGet = JsBlockStorageGet::from(get_closure.into_js_value());
	let set: JsBlockStorageSet = JsBlockStorageSet::from(set_closure.into_js_value());
	let storage = JsBlockStorage::new(get, set).expect("storage");

	let rust_stream = stream::empty();
	let web_stream = wasm_streams::ReadableStream::from_stream(rust_stream).into_raw();
	let cids = js_unixfs_add(&storage, web_stream).await.unwrap();
	let rust_cids: Vec<Cid> = from_js_value(cids).unwrap();
	assert_eq!(rust_cids.len(), 1);
	assert_eq!(rust_cids[0].to_string(), "QmbFMke1KXqnYyBBWxB74N4c5SBnJMVAiMNRcGu6x1AwQH".to_owned());
}
