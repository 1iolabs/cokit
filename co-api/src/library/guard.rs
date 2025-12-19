use crate::{
	async_api::Context,
	library::{wasm_context::WasmContext, wasm_storage::WasmStorage},
	Guard,
};
use co_primitives::{from_cbor, BlockStorage, BlockStorageExt, DiagnosticMessage, GuardVerifyPayload};
use futures::{executor::LocalPool, task::LocalSpawnExt};

pub fn guard<R>() -> bool
where
	R: Guard<WasmStorage>,
{
	let context = WasmContext::new();
	guard_with_context::<WasmStorage, WasmContext, R>(context)
}

pub fn guard_with_context<S, C, R>(mut context: C) -> bool
where
	S: BlockStorage + Clone + 'static,
	C: Context<S> + 'static,
	R: Guard<S>,
{
	let mut pool = LocalPool::new();
	let handle = pool
		.spawner()
		.spawn_local_with_handle(async move {
			match guard_execute_with_context::<S, C, R>(&context).await {
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

pub async fn guard_execute_with_context<S, C, R>(context: &C) -> Result<bool, anyhow::Error>
where
	S: BlockStorage + Clone + 'static,
	C: Context<S> + 'static,
	R: Guard<S>,
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
