use co_identity::{IdentityResolver, LocalIdentityResolver};
use co_log::Log;
use co_storage::MemoryBlockStorage;
use std::collections::BTreeSet;

/// Test unseen but already integrated joins.
///
/// See: https://gitlab.1io.com/1io/co-sdk/-/issues/57
#[tokio::test]
async fn test_previous_heads() {
	let identities = LocalIdentityResolver::new();
	let identity = identities.private_identity("did:local:test").unwrap();
	let storage = MemoryBlockStorage::new();

	// create
	let mut log = Log::new("test".as_bytes().to_vec(), identities.clone().boxed(), storage.clone(), Default::default());
	log.push_event(&identity, &0).await.unwrap();
	log.push_event(&identity, &1).await.unwrap();
	log.push_event(&identity, &2).await.unwrap();
	let heads2 = log.heads().clone();
	log.push_event(&identity, &3).await.unwrap();
	log.push_event(&identity, &4).await.unwrap();
	let heads4 = log.heads().clone();

	// join
	let mut log = Log::new("test".as_bytes().to_vec(), identities.clone().boxed(), storage.clone(), Default::default());
	assert_eq!(log.join_heads(heads4.iter()).await.unwrap(), true);
	assert_eq!(log.heads(), &heads4);
	assert_eq!(log.join_heads(heads2.iter()).await.unwrap(), false); // should have no effect as its already integrated
	assert_eq!(log.heads(), &heads4);
}

#[tokio::test]
async fn test_previous_heads_not_load_whole_log_item_hit() {
	let identities = LocalIdentityResolver::new();
	let identity = identities.private_identity("did:local:test").unwrap();
	let storage = MemoryBlockStorage::new();

	// create
	let mut log = Log::new("test".as_bytes().to_vec(), identities.clone().boxed(), storage.clone(), Default::default());
	let (entry0, _) = log.push_event(&identity, &0).await.unwrap();
	let (entry1, _) = log.push_event(&identity, &1).await.unwrap();
	let (entry2, _) = log.push_event(&identity, &2).await.unwrap();
	let heads2 = log.heads().clone();
	log.push_event(&identity, &3).await.unwrap();
	log.push_event(&identity, &4).await.unwrap();
	let heads4 = log.heads().clone();

	// validate to not the whole log has been loaded
	let mut log = Log::new("test".as_bytes().to_vec(), identities.clone().boxed(), storage.clone(), heads4.clone());
	assert_eq!(log.join_heads(heads2.iter()).await.unwrap(), false);
	assert_eq!(log.heads(), &heads4);
	assert_eq!(log.contains(entry2.cid()), true);
	assert_eq!(log.contains(entry1.cid()), false);
	assert_eq!(log.contains(entry0.cid()), false);
}

#[tokio::test]
async fn test_previous_heads_not_load_whole_log_clock_hit() {
	let identities = LocalIdentityResolver::new();
	let identity = identities.private_identity("did:local:test").unwrap();
	let identity2 = identities.private_identity("did:local:test").unwrap();
	let storage = MemoryBlockStorage::new();

	// create
	let mut log = Log::new("test".as_bytes().to_vec(), identities.clone().boxed(), storage.clone(), Default::default());
	let (entry0, _) = log.push_event(&identity, &0).await.unwrap();
	let (entry1, _) = log.push_event(&identity, &1).await.unwrap();
	let (entry2, _) = log.push_event(&identity, &2).await.unwrap();
	let heads2 = log.heads().clone();
	log.push_event(&identity, &3).await.unwrap();
	log.push_event(&identity, &4).await.unwrap();
	let heads4 = log.heads().clone();

	// create item after `2`
	let mut log2 = Log::new("test".as_bytes().to_vec(), identities.clone().boxed(), storage.clone(), heads2.clone());
	let (entry30, _) = log2.push_event(&identity2, &30).await.unwrap();
	let log2_heads = log2.heads().clone();

	// validate to not the whole log has been loaded and stop after clock has seen
	let mut log = Log::new("test".as_bytes().to_vec(), identities.clone().boxed(), storage.clone(), heads4.clone());
	assert_eq!(log.join_heads(log2_heads.iter()).await.unwrap(), true);
	let mut expected_heads = heads4.clone();
	expected_heads.insert(*entry30.cid());
	assert_eq!(log.heads(), &expected_heads);
	assert_eq!(log.contains(entry2.cid()), true);
	assert_eq!(log.contains(entry1.cid()), true);
	assert_eq!(log.contains(entry0.cid()), false);
}
