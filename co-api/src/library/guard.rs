use crate::{async_api::Context, library::wasm_context::WasmContext, Guard};
use co_primitives::{from_cbor, BlockStorageExt, DiagnosticMessage, GuardVerifyPayload};
use futures::{executor::LocalPool, task::LocalSpawnExt};

pub fn guard<R>() -> bool
where
	R: Guard,
{
	guard_with_context::<_, R>(WasmContext::new())
}

pub fn guard_with_context<C, R>(mut context: C) -> bool
where
	C: Context + 'static,
	R: Guard,
{
	let mut pool = LocalPool::new();
	let handle = pool
		.spawner()
		.spawn_local_with_handle(async move {
			match guard_execute_with_context::<C, R>(&context).await {
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
		})
		.expect("future to execute");
	pool.run_until(handle)
}

pub async fn guard_execute_with_context<C, R>(context: &C) -> Result<bool, anyhow::Error>
where
	C: Context + 'static,
	R: Guard,
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
