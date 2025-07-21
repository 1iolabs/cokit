use cid::Cid;
use co_api::{BlockSerializer, ReducerAction};
use co_sdk::{RuntimeContext, RuntimePool};
use co_storage::{unixfs_add_file, BlockStorage, MemoryBlockStorage};
use example_counter::{Counter, CounterAction};
use std::process::Command;

#[tokio::test]
async fn async_integration_test() {
	tracing_subscriber::fmt::fmt()
		.with_span_events(tracing_subscriber::fmt::format::FmtSpan::CLOSE)
		.with_target(false)
		.with_level(false)
		.init();

	// build
	assert!(Command::new("cargo")
		.args([
			"build",
			"--features",
			"core",
			"--target=wasm32-unknown-unknown",
			"--target-dir",
			"../../target-wasm",
			"--release",
		])
		.status()
		.unwrap()
		.success());

	// storage
	let storage = MemoryBlockStorage::default();

	// action
	let action = ReducerAction {
		core: "".to_owned(),
		payload: CounterAction::Increment(10),
		from: "did:local:test".to_owned(),
		time: 0,
	};
	let action_block = BlockSerializer::default().serialize(&action).unwrap();
	let action_cid = *action_block.cid();
	storage.set(action_block).await.unwrap();

	// wasm
	let wasm = unixfs_add_file(&storage, "../../target-wasm/wasm32-unknown-unknown/release/example_counter.wasm")
		.await
		.unwrap();

	// execute
	let next_state = RuntimePool::default()
		.execute_state(&storage, &wasm.into(), RuntimeContext::new(None, action_cid))
		.await
		.unwrap()
		.state;

	// test
	assert_eq!(Some(Cid::try_from("bafyr4ibjkgjouhwikzwvmoy2owd6l4azqwam3piehbbpkcikjqmxyiggpi").unwrap()), next_state);
	let block = storage.get(&next_state.unwrap()).await.unwrap();
	let state: Counter = BlockSerializer::default().deserialize(&block).unwrap();
	assert_eq!(state, Counter(10));
}
