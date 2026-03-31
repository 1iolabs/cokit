// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use cid::Cid;
use co_api::{BlockSerializer, Link, ReducerAction};
use co_primitives::ReducerInput;
use co_runtime::{co_v1::CoV1Api, create_runtime, RuntimeContext};
use co_storage::{MemoryStorage, Storage, SyncStorage};
use example_message::{MessageAction, MessageState, Role};
use std::{collections::BTreeMap, process::Command, str::FromStr};

#[test]
fn integration_test() {
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
	let memory = MemoryStorage::new();
	let mut storage = SyncStorage::new(memory);

	// action
	let action = ReducerAction {
		core: "".to_owned(),
		payload: MessageAction::Message,
		from: "did:local:test".to_owned(),
		time: 0,
	};
	let action_block = BlockSerializer::default().serialize(&action).unwrap();
	let action_cid = *action_block.cid();
	storage.set(action_block).unwrap();

	// api
	let api = CoV1Api::new(
		Box::new(storage.clone()),
		RuntimeContext::new(&ReducerInput { state: None, action: action_cid }).unwrap(),
	);

	// wasm
	let wasm_path = "../../target-wasm/wasm32-unknown-unknown/release/example_message.wasm";
	let wasm_bytes = std::fs::read(wasm_path).unwrap();
	let next_state = create_runtime(false, wasm_bytes).execute_state(api).unwrap().state;

	// test
	assert_eq!(next_state, Some(Cid::try_from("bafyr4iaubci6nz2uvpvxj4tyduktwdbcnnff4rbbuq2mfy24a5l6sa3uii").unwrap()));

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
