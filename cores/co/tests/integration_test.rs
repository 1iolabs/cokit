use co_api::{BlockSerializer, ReducerAction, Tags};
use co_core_co::{Co, CoAction};
use co_runtime::{RuntimeContext, RuntimePool};
use co_storage::{store_file, BlockStorage, MemoryBlockStorage};
use libipld::Cid;
use std::process::Command;

#[tokio::test]
async fn integration_test() {
	// tracing_subscriber::fmt::fmt()
	// 	.with_span_events(tracing_subscriber::fmt::format::FmtSpan::CLOSE)
	// 	.with_target(false)
	// 	.with_level(false)
	// 	.init();

	// build
	Command::new("cargo")
		.args(["build", "--target=wasm32-unknown-unknown", "--release"])
		.output()
		.unwrap();

	// storage
	let storage = MemoryBlockStorage::new();

	// action
	let mut tags = Tags::new();
	tags.insert(("hello".to_owned(), "world".to_owned().into()));
	let action = ReducerAction {
		core: "".to_owned(),
		payload: CoAction::TagsInsert { tags: tags.clone() },
		from: "did:local:test".to_owned(),
		time: 0,
	};
	let action_block = BlockSerializer::default().serialize(&action).unwrap();
	let action_cid = action_block.cid().clone();
	storage.set(action_block).await.unwrap();

	// wasm
	let wasm = store_file(&storage, "../../target/wasm32-unknown-unknown/release/co_core_co.wasm")
		.await
		.unwrap();

	// execute
	let next_state = RuntimePool::default()
		.execute(&storage, &wasm.into(), RuntimeContext { state: None, event: action_cid })
		.await
		.unwrap();

	// test
	assert_eq!(Some(Cid::try_from("bafyr4ib6qju66lt5hlrewi7rhqtk2ndnmvdrsls4moozdh7qii75y447ui").unwrap()), next_state);
	let block = storage.get(&next_state.unwrap()).await.unwrap();
	let state: Co = BlockSerializer::default().deserialize(&block).unwrap();
	// Co { id: [], tags: Tags { hello: String("world") }, name: "", heads: {}, participants: {}, cores: {}, keys: None,
	// peers: {} }
	// println!("{:?}", state);
	assert_eq!(tags, state.tags);
}
