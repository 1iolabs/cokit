use anyhow::anyhow;
use co_actor::{ActorError, Response, ResponseReceiver};
use std::ops::Not;
use wasm_bindgen_futures::spawn_local;

pub trait JsActor: 'static {
	type Message: 'static;

	async fn handle(&self, message: Self::Message);

	fn spawn(actor: Self) -> JsActorHandle<Self::Message>
	where
		Self: Sized,
	{
		let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Self::Message>();
		spawn_local(async move {
			while let Some(message) = rx.recv().await {
				actor.handle(message).await;
			}
		});
		JsActorHandle { tx }
	}
}

pub struct JsActorHandle<M> {
	tx: tokio::sync::mpsc::UnboundedSender<M>,
}
impl<M> JsActorHandle<M> {
	/// Request with response.
	pub async fn request<T>(&self, message: impl FnOnce(Response<T>) -> M) -> Result<T, ActorError> {
		let (responder, response) = ResponseReceiver::new();
		self.tx
			.send(message(responder))
			.map_err(|_| ActorError::InvalidState(anyhow!("Actor not running."), Default::default()))?;
		response.await
	}
}
impl<M> std::fmt::Debug for JsActorHandle<M> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("JsActorHandle")
			.field("open", &self.tx.is_closed().not())
			.finish()
	}
}
impl<M> Clone for JsActorHandle<M> {
	fn clone(&self) -> Self {
		Self { tx: self.tx.clone() }
	}
}
