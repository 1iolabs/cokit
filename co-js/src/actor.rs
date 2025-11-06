use co_actor::{LocalJoinError, LocalJoinHandle, LocalTaskSpawner};
use std::future::Future;
use wasm_bindgen_futures::spawn_local;

#[derive(Debug, Default, Clone, Copy)]
pub struct JsLocalTaskSpawner {}
impl LocalTaskSpawner for JsLocalTaskSpawner {
	fn spwan_local<F>(&self, fut: F) -> LocalJoinHandle<F::Output>
	where
		F: Future + 'static,
		F::Output: 'static,
	{
		let (tx, rx) = tokio::sync::oneshot::channel();
		spawn_local(async move {
			tx.send(fut.await).ok();
		});
		LocalJoinHandle::new(async move { rx.await.map_err(|_err| LocalJoinError::Cancelled) })
	}
}
