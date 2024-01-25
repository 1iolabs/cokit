use co_primitives::ReducerAction;
use co_storage::{unixfs_add, BlockSerializer, BlockStorage, MemoryBlockStorage, SyncBlockStorage};
use co_wasm_runtime::{co_v1::CoV1Api, RuntimePool};
use example_counter::{Counter, CounterAction};
use libipld::Cid;
use std::process::Command;
use tokio::runtime::Handle;
use tokio_util::compat::TokioAsyncReadCompatExt;

#[tokio::test]
async fn async_integration_test() {
	tracing_subscriber::fmt::fmt()
		.with_span_events(tracing_subscriber::fmt::format::FmtSpan::CLOSE)
		.with_target(false)
		.with_level(false)
		.init();

	// build
	Command::new("cargo")
		.args(["build", "--target=wasm32-unknown-unknown", "--release"])
		.output()
		.unwrap();

	// storage
	let storage = MemoryBlockStorage::new();

	// action
	let action = ReducerAction {
		core: "".to_owned(),
		payload: CounterAction::Increment(10),
		from: "did:local:test".to_owned(),
		time: 0,
	};
	let action_block = BlockSerializer::default().serialize(&action).unwrap();
	let action_cid = action_block.cid().clone();
	storage.set(action_block).await.unwrap();

	// wasm
	let wasm_path = "../../target/wasm32-unknown-unknown/release/example_counter.wasm";
	let mut file = tokio::fs::File::open(wasm_path).await.unwrap().compat();
	let wasm = unixfs_add(&storage, &mut file).await.unwrap().last().unwrap().to_owned();

	// execute
	let api = CoV1Api::new(Box::new(SyncBlockStorage::new(storage.clone(), Handle::current())), None, action_cid);
	let next_state = RuntimePool::default().execute(&storage, &wasm, api).await.unwrap();

	// test
	assert_eq!(Some(Cid::try_from("bafyr4ibjkgjouhwikzwvmoy2owd6l4azqwam3piehbbpkcikjqmxyiggpi").unwrap()), next_state);
	let block = storage.get(&next_state.unwrap()).await.unwrap();
	let state: Counter = BlockSerializer::default().deserialize(&block).unwrap();
	assert_eq!(state, Counter(10));
}
