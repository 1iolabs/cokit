use cid::Cid;
use co_identity::{DidKeyIdentity, DidKeyIdentityResolver, IdentityResolverBox, PrivateIdentity};
use co_log::{Entry, Log};
use co_primitives::{BlockSerializer, Link};
use co_storage::{BlockStorage, MemoryBlockStorage};
use futures::TryStreamExt;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[tokio::test]
async fn smoke() {
	// tracing_subscriber::fmt::fmt()
	// 	.with_span_events(tracing_subscriber::fmt::format::FmtSpan::CLOSE)
	// 	.with_max_level(tracing::Level::TRACE)
	// 	.init();

	// create store
	let mut store = MemoryBlockStorage::default();
	let block0 = create_event(&mut store, "hello").await;

	// create log
	let identity = DidKeyIdentity::generate(None);
	let mut log = create_empty_log(&store).await;

	// populate log
	log.push(&identity, block0).await.unwrap();

	// check log
	let entries: Vec<_> = log.stream().try_collect().await.unwrap();
	assert_eq!(entries.len(), 1);

	// time
	assert_eq!(entries[0].entry().clock.time, 1);

	// next
	assert_eq!(BTreeSet::new(), entries[0].entry().next);
}

#[tokio::test]
async fn traverse_sinlge_user_log() {
	// create store
	let mut store = MemoryBlockStorage::default();
	let block0 = create_event(&mut store, "hello").await;
	let block1 = create_event(&mut store, "world").await;
	let block2 = create_event(&mut store, "whats").await;

	// create log
	let identity = DidKeyIdentity::generate(None);
	let mut log = create_empty_log(&store).await;

	// populate log
	log.push(&identity, block0).await.unwrap();
	log.push(&identity, block1).await.unwrap();
	log.push(&identity, block2).await.unwrap();

	// check log
	let entries: Vec<_> = log.stream().try_collect().await.unwrap();
	assert_eq!(entries.len(), 3);

	// time
	assert_eq!(entries[1].entry().clock.time, 2);
	assert_eq!(entries[0].entry().clock.time, 3);
	assert_eq!(entries[2].entry().clock.time, 1);

	// next
	assert_eq!(entries[0].entry().next, BTreeSet::from([*entries[1].cid()]));
	assert_eq!(entries[1].entry().next, BTreeSet::from([*entries[2].cid()]));
	assert_eq!(entries[2].entry().next, BTreeSet::from([]));
}

#[tokio::test]
async fn join_is_associative() {
	let store = MemoryBlockStorage::default();
	let identity1 = DidKeyIdentity::generate(None);
	let identity2 = DidKeyIdentity::generate(None);
	let identity3 = DidKeyIdentity::generate(None);

	// create logs
	let mut log1 = create_empty_log(&store).await;
	let mut log2 = create_empty_log(&store).await;
	let mut log3 = create_empty_log(&store).await;
	log_push(&mut log1, &identity1, "helloA1").await;
	log_push(&mut log1, &identity1, "helloA2").await;
	log_push(&mut log2, &identity2, "helloB1").await;
	log_push(&mut log2, &identity2, "helloB2").await;
	log_push(&mut log3, &identity3, "helloC1").await;
	log_push(&mut log3, &identity3, "helloC2").await;

	// log1 + (log2 + log3)
	log2.join(&log3).await.unwrap();
	log1.join(&log2).await.unwrap();
	let res1: Vec<_> = log1.stream().try_collect().await.unwrap();

	// create logs
	let mut log1 = create_empty_log(&store).await;
	let mut log2 = create_empty_log(&store).await;
	let mut log3 = create_empty_log(&store).await;
	log_push(&mut log1, &identity1, "helloA1").await;
	log_push(&mut log1, &identity1, "helloA2").await;
	log_push(&mut log2, &identity2, "helloB1").await;
	log_push(&mut log2, &identity2, "helloB2").await;
	log_push(&mut log3, &identity3, "helloC1").await;
	log_push(&mut log3, &identity3, "helloC2").await;

	// (log1 + log2) + log3)
	log1.join(&log2).await.unwrap();
	log3.join(&log1).await.unwrap();
	let res2: Vec<_> = log3.stream().try_collect().await.unwrap();

	// associativity: log1 + (log2 + log3) == (log1 + log2) + log3
	assert_eq!(res1.len(), 6);
	assert_eq!(res2.len(), 6);
	assert_eq!(res2, res1);
}

#[tokio::test]
async fn join_is_commutative() {
	let store = MemoryBlockStorage::default();
	let identity1 = DidKeyIdentity::generate(None);
	let identity2 = DidKeyIdentity::generate(None);

	// create logs
	let mut log1 = create_empty_log(&store).await;
	let mut log2 = create_empty_log(&store).await;
	log_push(&mut log1, &identity1, "helloA1").await;
	log_push(&mut log1, &identity1, "helloA2").await;
	log_push(&mut log2, &identity2, "helloB1").await;
	log_push(&mut log2, &identity2, "helloB2").await;

	// log2 + log1
	log2.join(&log1).await.unwrap();
	let res1: Vec<_> = log2.stream().try_collect().await.unwrap();

	// create logs
	let mut log1 = create_empty_log(&store).await;
	let mut log2 = create_empty_log(&store).await;
	log_push(&mut log1, &identity1, "helloA1").await;
	log_push(&mut log1, &identity1, "helloA2").await;
	log_push(&mut log2, &identity2, "helloB1").await;
	log_push(&mut log2, &identity2, "helloB2").await;

	// log1 + log2
	log1.join(&log2).await.unwrap();
	let res2: Vec<_> = log1.stream().try_collect().await.unwrap();

	// commutativity: log2 + log1 == log1 + log2
	assert_eq!(res1.len(), 4);
	assert_eq!(res2.len(), 4);
	assert_eq!(res2, res1);
}

async fn create_empty_log<S: BlockStorage + Clone + Send + Sync + 'static>(store: &S) -> Log<S> {
	Log::new(
		"test".as_bytes().to_vec(),
		IdentityResolverBox::new(DidKeyIdentityResolver::new()),
		store.clone(),
		Default::default(),
	)
}

async fn log_push<S, I>(log: &mut Log<S>, identity: &I, t: &str) -> (Cid, Link<Entry>)
where
	S: BlockStorage + Clone + Send + Sync + 'static,
	I: PrivateIdentity + Send + Sync,
{
	let block = create_event(log.storage(), t).await;
	let entry = log.push(identity, block).await.unwrap();
	(block, entry)
}

async fn create_event<S: BlockStorage + Send + Sync>(storage: &S, t: &str) -> Cid {
	let block = BlockSerializer::new().serialize(&Event { t: t.to_owned() }).unwrap();
	storage.set(block.clone()).await.unwrap()
}

#[derive(Debug, Serialize, Deserialize)]
struct Event {
	#[serde(rename = "type")]
	t: String,
}
