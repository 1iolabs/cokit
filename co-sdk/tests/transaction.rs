// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use cid::Cid;
use co_core_co::CoAction;
use co_identity::LocalIdentity;
use co_runtime::Core;
use co_sdk::{
	build_core, crate_repository_path,
	state::{query_core, QueryExt},
	ApplicationBuilder, BuildCoreArtifact, CoReducer, CoreName, CreateCo, DidKeyIdentity, DidKeyProvider,
	PrivateIdentity, PrivateIdentityBox, CO_CORE_NAME_CO, CO_CORE_NAME_KEYSTORE,
};
use co_storage::MemoryBlockStorage;
use co_test::test_log_path;
use example_counter::{Counter, CounterAction};

async fn build_counter() -> (Cid, Core, BuildCoreArtifact) {
	let core_storage = MemoryBlockStorage::default();
	let repository_path = crate_repository_path(true).unwrap();
	let core_path = repository_path.join("examples/counter");
	let counter_artifact = build_core(repository_path, core_path).unwrap();
	let counter = counter_artifact.store_artifact(&core_storage).await.unwrap();
	let native_counter = Core::native::<Counter, CounterAction>();
	(counter, native_counter, counter_artifact)
}

async fn counter_count(co: &CoReducer) -> i64 {
	let (_storage, counter) = query_core(CoreName::<Counter>::new("counter"))
		.execute_reducer(co)
		.await
		.unwrap();
	counter.0
}

async fn setup_local_counter() -> (co_sdk::Application, CoReducer) {
	let (counter, counter_core, counter_artifact) = build_counter().await;
	let application = ApplicationBuilder::new_memory(format!("test_transaction-{}", uuid::Uuid::new_v4()))
		.with_bunyan_logging(Some(test_log_path()))
		.with_optional_tracing()
		.without_keychain()
		.with_disabled_feature("co-local-encryption")
		.with_core(counter, counter_core)
		.build()
		.await
		.expect("application");
	let local_co = application.local_co_reducer().await.unwrap();
	counter_artifact.store_artifact(&local_co.storage()).await.unwrap();
	local_co
		.push(
			&application.local_identity(),
			CO_CORE_NAME_CO,
			&CoAction::CoreCreate { core: "counter".to_owned(), binary: counter, tags: Default::default() },
		)
		.await
		.unwrap();

	(application, local_co)
}

/// Setup a shared CO with counter core. Pinning only applies to the local CO,
/// so a shared CO gives deterministic CIDs between sequential and transaction paths.
async fn setup_shared_counter(
	encrypted: bool,
	counter: Cid,
	counter_core: Core,
	counter_artifact: &BuildCoreArtifact,
) -> (co_sdk::Application, CoReducer, PrivateIdentityBox) {
	let application = ApplicationBuilder::new_memory(format!("deterministic-{}", uuid::Uuid::new_v4()))
		.without_keychain()
		.with_core(counter, counter_core)
		.build()
		.await
		.expect("application");
	let local_co = application.local_co_reducer().await.unwrap();

	// identity
	let identity = DidKeyIdentity::generate(Some(&[1; 32]));
	let provider = DidKeyProvider::new(local_co, CO_CORE_NAME_KEYSTORE);
	provider.store(&identity, None).await.unwrap();

	// create shared CO
	let co = application
		.create_co(identity.clone(), CreateCo::new("shared", None).with_public(!encrypted))
		.await
		.unwrap();
	counter_artifact.store_artifact(&co.storage()).await.unwrap();
	co.push(
		&identity,
		CO_CORE_NAME_CO,
		&CoAction::CoreCreate { core: "counter".to_owned(), binary: counter, tags: Default::default() },
	)
	.await
	.unwrap();

	(application, co, identity.boxed())
}

/// Verify that a transaction produces the same state as sequential pushes.
///
/// Uses a shared CO (no pinning interference) to compare sequential and transaction push.
/// Both paths push the same actions and must produce the same counter value and structure.
#[tokio::test]
async fn test_transaction_same_state_as_sequential_push() {
	let (counter, counter_core, counter_artifact) = build_counter().await;
	let (_app1, co1, identity1) = setup_shared_counter(false, counter, counter_core.clone(), &counter_artifact).await;
	let (_app2, co2, identity2) = setup_shared_counter(false, counter, counter_core, &counter_artifact).await;

	let actions: Vec<CounterAction> = (1..=20).map(CounterAction::Increment).collect();

	// sequential push on co1
	for action in &actions {
		co1.push(&identity1, "counter", action).await.unwrap();
	}

	// transaction push on co2
	let mut tx = co2.transaction().unwrap();
	for action in &actions {
		tx.push(&identity2, "counter", action).await.unwrap();
	}
	tx.commit().await.unwrap();

	// verify same counter value and structure
	assert_eq!(counter_count(&co1).await, 210, "sequential push counter");
	assert_eq!(counter_count(&co2).await, 210, "transaction push counter");

	let state1 = co1.reducer_state().await;
	let state2 = co2.reducer_state().await;
	assert!(state1.state().is_some(), "sequential state should exist");
	assert!(state2.state().is_some(), "transaction state should exist");
	assert_eq!(state1.heads().len(), 1, "sequential should have one head");
	assert_eq!(state2.heads().len(), 1, "transaction should have one head");
}

/// Encrypted-shared-CO variant of `test_transaction_same_state_as_sequential_push`.
#[tokio::test]
async fn test_transaction_same_state_as_sequential_push_on_encrypted_co() {
	let (counter, counter_core, counter_artifact) = build_counter().await;
	let (_app1, co1, identity1) = setup_shared_counter(true, counter, counter_core.clone(), &counter_artifact).await;
	let (_app2, co2, identity2) = setup_shared_counter(true, counter, counter_core, &counter_artifact).await;

	let actions: Vec<CounterAction> = (1..=20).map(CounterAction::Increment).collect();

	// sequential push on co1
	for action in &actions {
		co1.push(&identity1, "counter", action).await.unwrap();
	}

	// transaction push on co2
	let mut tx = co2.transaction().unwrap();
	for action in &actions {
		tx.push(&identity2, "counter", action).await.unwrap();
	}
	tx.commit().await.unwrap();

	// verify same counter value and structure
	assert_eq!(counter_count(&co1).await, 210, "sequential push counter");
	assert_eq!(counter_count(&co2).await, 210, "transaction push counter");

	let state1 = co1.reducer_state().await;
	let state2 = co2.reducer_state().await;
	assert!(state1.state().is_some(), "sequential state should exist");
	assert!(state2.state().is_some(), "transaction state should exist");
	assert_eq!(state1.heads().len(), 1, "sequential should have one head");
	assert_eq!(state2.heads().len(), 1, "transaction should have one head");
}

/// Verify that an empty transaction returns the current state without side effects.
#[tokio::test]
async fn test_transaction_empty_commit() {
	let (application, local_co) = setup_local_counter().await;

	let state_before = local_co.reducer_state().await;
	let tx = local_co.transaction().unwrap();
	let state_after = tx.commit().await.unwrap();
	assert_eq!(state_before, state_after);

	drop(application);
}

/// Verify atomicity: when the transaction actor encounters an error,
/// the real reducer state is untouched.
#[tokio::test]
async fn test_transaction_atomic_on_error() {
	let (application, local_co) = setup_local_counter().await;
	let identity = application.local_identity();

	// push a valid action first so we have a known state
	local_co.push(&identity, "counter", &CounterAction::Increment(5)).await.unwrap();
	let state_before = local_co.reducer_state().await;
	assert_eq!(counter_count(&local_co).await, 5);

	// create a transaction that pushes a valid action then an invalid one
	let mut tx = local_co.transaction().unwrap();
	tx.push(&identity, "counter", &CounterAction::Increment(10)).await.unwrap();
	// push to a non-existent core to trigger an error in the transaction actor
	tx.push(&identity, "nonexistent_core", &CounterAction::Increment(1))
		.await
		.unwrap();
	let result = tx.commit().await;
	assert!(result.is_err(), "commit should fail when a push targets a missing core");

	// verify the real reducer is untouched
	let state_after = local_co.reducer_state().await;
	assert_eq!(state_before, state_after, "state should not change on failed transaction");
	assert_eq!(counter_count(&local_co).await, 5, "counter should remain at 5");

	drop(application);
}

/// Verify multiple sequential transactions accumulate state correctly.
#[tokio::test]
async fn test_transaction_multiple_commits() {
	let (application, local_co) = setup_local_counter().await;
	let identity = LocalIdentity::new("user");

	// first transaction
	let mut tx = local_co.transaction().unwrap();
	for i in 1..=5 {
		tx.push(&identity, "counter", &CounterAction::Increment(i)).await.unwrap();
	}
	tx.commit().await.unwrap();
	assert_eq!(counter_count(&local_co).await, 15);

	// second transaction
	let mut tx = local_co.transaction().unwrap();
	for i in 1..=5 {
		tx.push(&identity, "counter", &CounterAction::Multiply(i)).await.unwrap();
	}
	tx.commit().await.unwrap();

	// 15 * 1 * 2 * 3 * 4 * 5 = 1800
	assert_eq!(counter_count(&local_co).await, 1800);

	drop(application);
}
