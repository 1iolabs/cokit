// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use co_identity::{IdentityResolver, LocalIdentityResolver};
use co_log::{IdentityEntryVerifier, Log};
use co_storage::MemoryBlockStorage;

/// Test unseen but already integrated joins.
///
/// See: https://gitlab.1io.com/1io/co-sdk/-/issues/57
#[tokio::test]
async fn test_previous_heads() {
	let identities = LocalIdentityResolver::new();
	let entry_verifier = IdentityEntryVerifier::new(identities.clone().boxed());
	let identity = identities.private_identity("did:local:test").unwrap();
	let storage = MemoryBlockStorage::default();

	// create
	let mut log = Log::new("test".as_bytes().to_vec(), entry_verifier.clone(), Default::default());
	log.push_event(&storage, &identity, &0).await.unwrap();
	log.push_event(&storage, &identity, &1).await.unwrap();
	log.push_event(&storage, &identity, &2).await.unwrap();
	let heads2 = log.heads().clone();
	log.push_event(&storage, &identity, &3).await.unwrap();
	log.push_event(&storage, &identity, &4).await.unwrap();
	let heads4 = log.heads().clone();

	// join
	let mut log = Log::new("test".as_bytes().to_vec(), entry_verifier.clone(), Default::default());
	assert!(log.join_heads(&storage, heads4.iter()).await.unwrap());
	assert_eq!(log.heads(), &heads4);
	assert!(!log.join_heads(&storage, heads2.iter()).await.unwrap()); // should have no effect as its already integrated
	assert_eq!(log.heads(), &heads4);
}

#[tokio::test]
async fn test_previous_heads_not_load_whole_log_item_hit() {
	let identities = LocalIdentityResolver::new();
	let entry_verifier = IdentityEntryVerifier::new(identities.clone().boxed());
	let identity = identities.private_identity("did:local:test").unwrap();
	let storage = MemoryBlockStorage::default();

	// create
	let mut log = Log::new("test".as_bytes().to_vec(), entry_verifier.clone(), Default::default());
	let (entry0, _) = log.push_event(&storage, &identity, &0).await.unwrap();
	let (entry1, _) = log.push_event(&storage, &identity, &1).await.unwrap();
	let (entry2, _) = log.push_event(&storage, &identity, &2).await.unwrap();
	let heads2 = log.heads().clone();
	log.push_event(&storage, &identity, &3).await.unwrap();
	log.push_event(&storage, &identity, &4).await.unwrap();
	let heads4 = log.heads().clone();

	// validate to not the whole log has been loaded
	let mut log = Log::new("test".as_bytes().to_vec(), entry_verifier.clone(), heads4.clone());
	assert!(!log.join_heads(&storage, heads2.iter()).await.unwrap());
	assert_eq!(log.heads(), &heads4);
	assert!(log.contains(entry2.cid()));
	assert!(!log.contains(entry1.cid()));
	assert!(!log.contains(entry0.cid()));
}

#[tokio::test]
async fn test_previous_heads_not_load_whole_log_clock_hit() {
	let identities = LocalIdentityResolver::new();
	let entry_verifier = IdentityEntryVerifier::new(identities.clone().boxed());
	let identity = identities.private_identity("did:local:test").unwrap();
	let identity2 = identities.private_identity("did:local:test").unwrap();
	let storage = MemoryBlockStorage::default();

	// create
	let mut log = Log::new("test".as_bytes().to_vec(), entry_verifier.clone(), Default::default());
	let (entry0, _) = log.push_event(&storage, &identity, &0).await.unwrap();
	let (entry1, _) = log.push_event(&storage, &identity, &1).await.unwrap();
	let (entry2, _) = log.push_event(&storage, &identity, &2).await.unwrap();
	let heads2 = log.heads().clone();
	log.push_event(&storage, &identity, &3).await.unwrap();
	log.push_event(&storage, &identity, &4).await.unwrap();
	let heads4 = log.heads().clone();

	// create item after `2`
	let mut log2 = Log::new("test".as_bytes().to_vec(), entry_verifier.clone(), heads2.clone());
	let (entry30, _) = log2.push_event(&storage, &identity2, &30).await.unwrap();
	let log2_heads = log2.heads().clone();

	// validate to not the whole log has been loaded and stop after clock has seen
	let mut log = Log::new("test".as_bytes().to_vec(), entry_verifier.clone(), heads4.clone());
	assert!(log.join_heads(&storage, log2_heads.iter()).await.unwrap());
	let mut expected_heads = heads4.clone();
	expected_heads.insert(*entry30.cid());
	assert_eq!(log.heads(), &expected_heads);
	assert!(log.contains(entry2.cid()));
	assert!(log.contains(entry1.cid()));
	assert!(!log.contains(entry0.cid()));
}
