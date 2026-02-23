use crate::{async_api::Context, library::wasm_context::WasmContext, Guard};
use co_primitives::{from_cbor, BlockStorageExt, DiagnosticMessage, GuardVerifyPayload};
use futures::{executor::LocalPool, future::LocalBoxFuture, task::LocalSpawnExt, FutureExt};
use std::sync::Arc;

pub fn guard<R>() -> bool
where
	R: Guard + 'static,
{
	GuardRef::new::<R>().execute_blocking(WasmContext::new())
}

async fn execute<C, R>(context: &C) -> Result<bool, anyhow::Error>
where
	C: Context + 'static,
	R: Guard + 'static,
{
	let payload = context.payload();
	let guard_payload: GuardVerifyPayload = from_cbor(&payload)?;

	// guard
	let next_state = R::verify(
		context.storage(),
		guard_payload.guard,
		guard_payload.state,
		guard_payload.heads,
		guard_payload.next_head,
	)
	.await?;

	// result
	Ok(next_state)
}

#[allow(clippy::type_complexity)]
pub struct GuardRef<C>(
	Arc<dyn for<'a> Fn(&'a C) -> LocalBoxFuture<'a, Result<bool, anyhow::Error>> + Sync + Send + 'static>,
)
where
	C: Context + 'static;
impl<C> GuardRef<C>
where
	C: Context + 'static,
{
	pub fn new<R>() -> Self
	where
		R: Guard + 'static,
	{
		Self(Arc::new(|context| async { execute::<C, R>(context).await }.boxed_local()))
	}

	pub fn execute_blocking(self, context: C) -> bool {
		let mut pool = LocalPool::new();
		let handle = pool
			.spawner()
			.spawn_local_with_handle(async move { self.execute_async(context).await })
			.expect("future to execute");
		pool.run_until(handle)
	}

	pub async fn execute_async(&self, mut context: C) -> bool {
		match self.execute(&context).await {
			Ok(result) => result,
			Err(err) => {
				let cid = context
					.storage()
					.set_serialized(&DiagnosticMessage::from(err))
					.await
					.expect("DiagnosticMessage to serialize");
				context.write_diagnostic(cid);
				false
			},
		}
	}

	async fn execute(&self, context: &C) -> Result<bool, anyhow::Error> {
		(self.0)(context).await
	}
}
impl<C> Clone for GuardRef<C>
where
	C: Context + 'static,
{
	fn clone(&self) -> Self {
		Self(self.0.clone())
	}
}
