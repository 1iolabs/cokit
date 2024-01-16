use co_primitives::{Link, ReducerAction};
use co_storage::{Algorithm, BlockSerializer, EncryptedStorage, MemoryStorage, Secret, Storage, SyncStorage};
use co_wasm_runtime::{co_v1::CoV1Api, create_runtime};
use example_message::{MessageAction, MessageState, Role};
use libipld::Cid;
use std::{collections::BTreeMap, iter::repeat, process::Command, str::FromStr};

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
	let memory = MemoryStorage::new();
	let algorithm = Algorithm::default();
	let key = Secret::new(repeat(42).take(algorithm.key_size()).collect());
	let encrypted = EncryptedStorage::new(memory, key, algorithm);
	let mut storage = SyncStorage::new(encrypted);

	// action
	let action = ReducerAction { payload: MessageAction::Message, from: "did:local:test".to_owned(), time: 0 };
	let action_block = BlockSerializer::default().serialize(&action).unwrap();
	let action_cid = action_block.cid().clone();
	storage.set(action_block).unwrap();

	// api
	let api = CoV1Api::new(Box::new(storage.clone()), None, action_cid);

	// wasm
	let wasm_path = "../../target/wasm32-unknown-unknown/release/example_message.wasm";
	let wasm_bytes = std::fs::read(wasm_path).unwrap();
	let next_state = create_runtime(wasm_bytes).execute(api).unwrap();

	// test
	assert_eq!(next_state, Some(Cid::try_from("bafyr4icsv74udrd3j5ewaybjf2sfp2frygniduzpf6scuu6ahlpdefdvoi").unwrap()));

	// test state
	let block = storage.get(&next_state.unwrap()).unwrap();
	let state: MessageState = BlockSerializer::default().deserialize(&block).unwrap();
	let mut participants = BTreeMap::new();
	participants.insert(
		"did:local:test".to_string(),
		Link::<Role>::new(Cid::from_str("bafyr4igf663hpuvdpvque42uxmkbacg5ubd4cgageulmwmqo33g2tpod7e").unwrap()),
	);
	assert_eq!(state, MessageState { message_count: 1, participants, ..MessageState::default() });
}
