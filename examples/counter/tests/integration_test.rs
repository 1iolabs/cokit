use co_api::{BlockSerializer, ReducerAction};
use co_runtime::{co_v1::CoV1Api, create_runtime, RuntimeContext};
use co_storage::{MemoryStorage, Storage, SyncStorage};
use example_counter::{Counter, CounterAction};
use libipld::Cid;
use std::process::Command;

#[test]
fn integration_test() {
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
	let mut storage = SyncStorage::new(MemoryStorage::new());

	// action
	let action = ReducerAction {
		core: "".to_owned(),
		payload: CounterAction::Increment(10),
		from: "did:local:test".to_owned(),
		time: 0,
	};
	let action_block = BlockSerializer::default().serialize(&action).unwrap();
	let action_cid = *action_block.cid();
	storage.set(action_block).unwrap();

	// api
	let api = CoV1Api::new(Box::new(storage.clone()), RuntimeContext { state: None, event: action_cid });

	// wasm
	let wasm_path = "../../target/wasm32-unknown-unknown/release/example_counter.wasm";
	let wasm_bytes = std::fs::read(wasm_path).unwrap();
	let next_state = create_runtime(wasm_bytes).execute(api).unwrap();

	// test
	assert_eq!(Some(Cid::try_from("bafyr4ibjkgjouhwikzwvmoy2owd6l4azqwam3piehbbpkcikjqmxyiggpi").unwrap()), next_state);
	let block = storage.get(&next_state.unwrap()).unwrap();
	let state: Counter = BlockSerializer::default().deserialize(&block).unwrap();
	assert_eq!(state, Counter(10));
}
