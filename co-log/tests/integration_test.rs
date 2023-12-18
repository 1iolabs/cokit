use co_log::{DidKeyIdentity, EntryStorage, Log};
use co_storage::{BlockSerializer, MemoryStorage, Storage};
use serde::{Deserialize, Serialize};

#[test]
fn it_should_travers_sinlge_user_logs() {
	tracing_subscriber::fmt::fmt()
		.with_span_events(tracing_subscriber::fmt::format::FmtSpan::CLOSE)
		.with_max_level(tracing::Level::TRACE)
		.init();

	// create store
	let mut store = MemoryStorage::new();
	let block0 = BlockSerializer::default().serialize(&Event { t: "hello".to_string() }).unwrap();
	let block1 = BlockSerializer::default().serialize(&Event { t: "world".to_string() }).unwrap();
	let block2 = BlockSerializer::default().serialize(&Event { t: "whats".to_string() }).unwrap();
	store.set(block0.clone()).unwrap();
	store.set(block1.clone()).unwrap();
	store.set(block2.clone()).unwrap();

	// create log
	let identity = Box::new(DidKeyIdentity::generate(None));
	let mut log = Log::new("test".as_bytes().to_vec(), identity, EntryStorage::new(Box::new(store)), Vec::new());

	// populate log
	log.push(block0.cid().clone()).unwrap();
	log.push(block1.cid().clone()).unwrap();
	log.push(block2.cid().clone()).unwrap();

	// check log
	let entries = log.iter().collect::<Result<Vec<_>, _>>().unwrap();
	assert_eq!(entries.len(), 3);

	// time
	assert_eq!(entries[1].entry().clock.time, 2);
	assert_eq!(entries[0].entry().clock.time, 3);
	assert_eq!(entries[2].entry().clock.time, 1);

	// next
	assert_eq!(entries[0].entry().next, vec![entries[1].cid().clone()]);
	assert_eq!(entries[1].entry().next, vec![entries[2].cid().clone()]);
	assert_eq!(entries[2].entry().next, vec![]);
}

#[derive(Debug, Serialize, Deserialize)]
struct Event {
	#[serde(rename = "type")]
	t: String,
}
