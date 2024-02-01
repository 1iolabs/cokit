use co_api::ReducerAction;
use co_sdk::{RuntimeContext, RuntimePool};
use co_storage::{store_file, BlockSerializer, BlockStorage, MemoryBlockStorage};
use example_counter::{Counter, CounterAction};
use libipld::Cid;
use std::process::Command;

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
	let wasm = store_file(&storage, "../../target/wasm32-unknown-unknown/release/example_counter.wasm")
		.await
		.unwrap();

	// execute
	let next_state = RuntimePool::default()
		.execute(&storage, &wasm, RuntimeContext { state: None, event: action_cid })
		.await
		.unwrap();

	// test
	assert_eq!(Some(Cid::try_from("bafyr4ibjkgjouhwikzwvmoy2owd6l4azqwam3piehbbpkcikjqmxyiggpi").unwrap()), next_state);
	let block = storage.get(&next_state.unwrap()).await.unwrap();
	let state: Counter = BlockSerializer::default().deserialize(&block).unwrap();
	assert_eq!(state, Counter(10));
}
