use co_log::{DidKeyIdentity, DidKeyIdentityResolver, EntryStorage, Log};
use co_storage::{BlockSerializer, MemoryStorage, Storage, SyncStorage};
use libipld::Cid;
use serde::{Deserialize, Serialize};

#[test]
fn smoke() {
	// tracing_subscriber::fmt::fmt()
	// 	.with_span_events(tracing_subscriber::fmt::format::FmtSpan::CLOSE)
	// 	.with_max_level(tracing::Level::TRACE)
	// 	.init();

	// create store
	let (mut store, _) = SyncStorage::new(MemoryStorage::new());
	let block0 = create_event(&mut store, "hello");

	// create log
	let identity = Box::new(DidKeyIdentity::generate(None));
	let mut log = create_empty_log(&identity, &store);

	// populate log
	log.push(block0.clone()).unwrap();

	// check log
	let entries = log.iter().collect::<Result<Vec<_>, _>>().unwrap();
	assert_eq!(entries.len(), 1);

	// time
	assert_eq!(entries[0].entry().clock.time, 1);

	// next
	assert_eq!(entries[0].entry().next, vec![]);
}

#[test]
fn traverse_sinlge_user_log() {
	// create store
	let (mut store, _) = SyncStorage::new(MemoryStorage::new());
	let block0 = create_event(&mut store, "hello");
	let block1 = create_event(&mut store, "world");
	let block2 = create_event(&mut store, "whats");

	// create log
	let identity = Box::new(DidKeyIdentity::generate(None));
	let mut log = create_empty_log(&identity, &store);

	// populate log
	log.push(block0.clone()).unwrap();
	log.push(block1.clone()).unwrap();
	log.push(block2.clone()).unwrap();

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

#[test]
fn join_is_associative() {
	let (store, _) = SyncStorage::new(MemoryStorage::new());
	let identity1 = DidKeyIdentity::generate(None);
	let identity2 = DidKeyIdentity::generate(None);
	let identity3 = DidKeyIdentity::generate(None);

	// create logs
	let mut log1 = create_empty_log(&identity1, &store);
	let mut log2 = create_empty_log(&identity2, &store);
	let mut log3 = create_empty_log(&identity3, &store);
	log_push(&mut log1, "helloA1");
	log_push(&mut log1, "helloA2");
	log_push(&mut log2, "helloB1");
	log_push(&mut log2, "helloB2");
	log_push(&mut log3, "helloC1");
	log_push(&mut log3, "helloC2");

	// log1 + (log2 + log3)
	log2.join(&log3).unwrap();
	log1.join(&log2).unwrap();
	let res1 = log1.iter().collect::<Result<Vec<_>, _>>().unwrap();

	// create logs
	let mut log1 = create_empty_log(&identity1, &store);
	let mut log2 = create_empty_log(&identity2, &store);
	let mut log3 = create_empty_log(&identity3, &store);
	log_push(&mut log1, "helloA1");
	log_push(&mut log1, "helloA2");
	log_push(&mut log2, "helloB1");
	log_push(&mut log2, "helloB2");
	log_push(&mut log3, "helloC1");
	log_push(&mut log3, "helloC2");

	// (log1 + log2) + log3)
	log1.join(&log2).unwrap();
	log3.join(&log1).unwrap();
	let res2 = log3.iter().collect::<Result<Vec<_>, _>>().unwrap();

	// associativity: log1 + (log2 + log3) == (log1 + log2) + log3
	assert_eq!(res1.len(), 6);
	assert_eq!(res2.len(), 6);
	assert_eq!(res2, res1);
}

#[test]
fn join_is_commutative() {
	let (store, _) = SyncStorage::new(MemoryStorage::new());
	let identity1 = DidKeyIdentity::generate(None);
	let identity2 = DidKeyIdentity::generate(None);

	// create logs
	let mut log1 = create_empty_log(&identity1, &store);
	let mut log2 = create_empty_log(&identity2, &store);
	log_push(&mut log1, "helloA1");
	log_push(&mut log1, "helloA2");
	log_push(&mut log2, "helloB1");
	log_push(&mut log2, "helloB2");

	// log2 + log1
	log2.join(&log1).unwrap();
	let res1 = log2.iter().collect::<Result<Vec<_>, _>>().unwrap();

	// create logs
	let mut log1 = create_empty_log(&identity1, &store);
	let mut log2 = create_empty_log(&identity2, &store);
	log_push(&mut log1, "helloA1");
	log_push(&mut log1, "helloA2");
	log_push(&mut log2, "helloB1");
	log_push(&mut log2, "helloB2");

	// log1 + log2
	log1.join(&log2).unwrap();
	let res2 = log1.iter().collect::<Result<Vec<_>, _>>().unwrap();

	// commutativity: log2 + log1 == log1 + log2
	assert_eq!(res1.len(), 4);
	assert_eq!(res2.len(), 4);
	assert_eq!(res2, res1);
}

fn create_empty_log(identity: &DidKeyIdentity, store: &SyncStorage) -> Log {
	Log::new(
		"test".as_bytes().to_vec(),
		Box::new(identity.clone()),
		Box::new(DidKeyIdentityResolver::new()),
		EntryStorage::new(Box::new(store.clone())),
		Vec::new(),
	)
}

fn log_push(log: &mut Log, t: &str) -> (Cid, Cid) {
	let block = create_event(log.storage_mut(), t);
	let entry = log.push(block.clone()).unwrap();
	(block, entry)
}

fn create_event(storage: &mut dyn Storage, t: &str) -> Cid {
	let block = BlockSerializer::default().serialize(&Event { t: t.to_owned() }).unwrap();
	storage.set(block.clone()).unwrap();
	block.into_inner().0
}

#[derive(Debug, Serialize, Deserialize)]
struct Event {
	#[serde(rename = "type")]
	t: String,
}
