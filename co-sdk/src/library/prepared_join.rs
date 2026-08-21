// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::{CoReducer, CoReducerState};
use anyhow::{anyhow, Context};
use std::collections::BTreeSet;

/// Move independently prepared overlays into the target reducer and integrate their states once.
pub(crate) async fn join_prepared_states(
	target: &CoReducer,
	prepared: Vec<(CoReducer, CoReducerState)>,
) -> Result<CoReducerState, anyhow::Error> {
	let target_overlay = target
		.overlay_storage()
		.ok_or_else(|| anyhow!("prepared join target has no overlay"))?;
	let mut states = BTreeSet::new();
	for (source, state) in prepared {
		if target.id() != source.id() {
			return Err(anyhow!("prepared join source belongs to a different CO"));
		}
		let source_overlay = source
			.overlay_storage()
			.ok_or_else(|| anyhow!("prepared join source has no overlay"))?;
		target_overlay
			.join(source_overlay)
			.await
			.context("failed to merge prepared join overlay")?;
		states.insert(state);
	}
	target.join_states(states).await
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{
		application::memory::create_memory_reducer, library::create_reducer_action::create_reducer_action, Application,
		ApplicationBuilder, CoReducerFactory, CreateCo, MonotonicCoUuid, CO_CORE_NAME_CO,
	};
	use cid::Cid;
	use co_core_co::CoAction;
	use co_identity::{Identity, LocalIdentity};
	use co_log::EntryBlock;
	use co_primitives::{tags, BlockSerializer, Clock, Entry, MonotonicCoDate, TagsAction};
	use co_storage::{BlockStorage, BlockStorageContentMapping, ExtendedBlockStorage};

	async fn fixture(name: &str, encrypted: bool) -> (Application, LocalIdentity, CoReducer) {
		co_test::init_test_log();
		let mut builder = ApplicationBuilder::new_memory(name.to_owned())
			.without_keychain()
			.with_co_date(MonotonicCoDate::default())
			.with_co_uuid(MonotonicCoUuid::default());
		if !encrypted {
			builder = builder.with_disabled_feature("co-local-encryption");
		}
		let application = builder.build().await.expect("application");
		let identity = application.local_identity();
		let create = if encrypted { CreateCo::new(name, None) } else { CreateCo::new(name, None).with_public(true) };
		let co = application.create_co(identity.clone(), create).await.expect("create co");
		(application, identity, co)
	}

	/// Build a branch entirely in one factory-created reducer overlay.
	async fn prepared_branch(
		application: &Application,
		co: &CoReducer,
		identity: &LocalIdentity,
		tag: &str,
	) -> (CoReducer, CoReducerState) {
		let source = application
			.context()
			.try_co_reducer(co.id())
			.await
			.expect("prepared source reducer");
		let storage = source.storage();
		let runtime = application.context().inner.runtime();
		let core_resolver = application.context().inner.create_shared_core_resolver(co.id().clone());
		let mut reducer = create_memory_reducer(
			runtime.runtime(),
			co.date().clone(),
			co.id(),
			&storage,
			Some(core_resolver),
			co.reducer_state().await,
		)
		.await
		.expect("memory reducer");
		let action = create_reducer_action(
			&storage,
			identity,
			CO_CORE_NAME_CO,
			&CoAction::Tags { action: TagsAction::insert(tags!("prepared-join": tag)) },
			Default::default(),
			co.date(),
		)
		.await
		.expect("branch action");
		reducer
			.push_reference(&storage, runtime.runtime(), identity, action)
			.await
			.expect("branch push");
		let state = CoReducerState::new_reducer(&reducer);
		(source, state)
	}

	#[co_test::timeout(10000)]
	#[tokio::test]
	async fn prepared_encrypted_state_transfers_and_integrates_once() {
		let (application, identity, co) = fixture("prepared-encrypted-transfer", true).await;
		let baseline = co.reducer_state().await;
		let (source, state) = prepared_branch(&application, &co, &identity, "encrypted").await;
		assert!(source.storage().is_content_mapped().await, "fixture must exercise mapped storage");
		let target = application
			.context()
			.try_co_reducer(co.id())
			.await
			.expect("prepared target reducer");

		let joined = join_prepared_states(&target, vec![(source, state)])
			.await
			.expect("integrated state");
		assert_ne!(joined, baseline, "the prepared branch changes the reducer");
		assert_eq!(co.reducer_state().await, joined, "the actor publishes the transferred state");
		co.co().await.expect("published encrypted state remains readable");
	}

	#[co_test::timeout(10000)]
	#[tokio::test]
	async fn integration_failure_leaves_the_prepared_overlay_owned_by_target() {
		let (application, identity, co) = fixture("prepared-integration-cleanup", false).await;
		let baseline = co.reducer_state().await;
		let source = application
			.context()
			.try_co_reducer(co.id())
			.await
			.expect("prepared source reducer");
		let target = application
			.context()
			.try_co_reducer(co.id())
			.await
			.expect("prepared target reducer");

		// the blocks transfer, but the payload is not a ReducerAction and therefore fails inside the
		// existing reducer integration machinery.
		let payload = BlockSerializer::default()
			.serialize(&"not-a-reducer-action")
			.expect("payload block");
		let payload_cid = source.storage().set(payload).await.expect("store payload");
		let baseline_head = *baseline.1.iter().next().expect("baseline head");
		let baseline_entry =
			EntryBlock::from_block(source.storage().get(&baseline_head).await.expect("load baseline entry"))
				.expect("decode baseline entry");
		let entry = EntryBlock::from_entry(
			&identity,
			Entry {
				id: baseline_entry.entry().id.clone(),
				payload: payload_cid,
				next: baseline.1.clone(),
				refs: Default::default(),
				clock: Clock::new(identity.identity().as_bytes().to_vec(), baseline_entry.entry().clock.time + 1),
			},
		)
		.expect("signed entry");
		let head = source
			.storage()
			.set(entry.block().expect("entry block"))
			.await
			.expect("store entry");
		let state = CoReducerState::new(None, BTreeSet::from([head]));

		assert!(
			join_prepared_states(&target, vec![(source, state)]).await.is_err(),
			"invalid action fails integration"
		);
		assert_eq!(co.reducer_state().await, baseline, "failed integration restores actor state");
		assert!(target.storage().exists(&head).await.expect("target-owned head"));
		assert!(
			target.storage().exists(&payload_cid).await.expect("target-owned payload"),
			"the failed reducer keeps ownership of its merged overlay until it is dropped"
		);
		assert!(!target.context.storage(false).exists(&head).await.expect("base head"));
		drop(target);
		let replacement = application
			.context()
			.try_co_reducer(co.id())
			.await
			.expect("replacement target reducer");
		assert!(
			!replacement.storage().exists(&head).await.expect("replacement head"),
			"dropping the failed reducer releases its private overlay"
		);
	}

	#[co_test::timeout(10000)]
	#[tokio::test]
	async fn unexpected_source_remove_fails_the_batch_without_actor_changes() {
		let (application, identity, co) = fixture("prepared-source-remove", false).await;
		let baseline = co.reducer_state().await;
		let (source, state) = prepared_branch(&application, &co, &identity, "source-remove").await;
		let head = *state.1.iter().next().expect("branch head");
		source
			.storage()
			.remove(&Cid::default())
			.await
			.expect("store unrelated source remove");
		let target = application
			.context()
			.try_co_reducer(co.id())
			.await
			.expect("prepared target reducer");

		assert!(join_prepared_states(&target, vec![(source, state)]).await.is_err(), "source remove fails the batch");
		assert_eq!(co.reducer_state().await, baseline, "the actor is never changed");
		assert!(
			!target.storage().exists(&head).await.expect("target overlay stays clean"),
			"a rejected overlay merge leaves no reachable target root"
		);
	}
}
