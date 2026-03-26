// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::helper::shared_co::SharedCo;
use co_core_co::CoAction;
use co_sdk::{create_default_guards, tags, CreateCo, CO_CORE_CO, CO_CORE_NAME_CO};
use helper::instance::Instances;

pub mod helper;

/// Test that pushed actions that are rejected by a guard not modify the state and heads.
#[tokio::test]
async fn test_guard_push() {
	let peer1 = Instances::new("test").create().await;

	// create identity
	let identity = peer1.create_identity().await;

	// create shared co
	let shared_co = peer1
		.application
		.create_co(identity.clone(), CreateCo::new("shared", None).without_co_guard())
		.await
		.unwrap();

	// push
	shared_co
		.push(&identity, CO_CORE_NAME_CO, &CoAction::TagsInsert { tags: tags!("hello": "world") })
		.await
		.unwrap();

	// create a second identity which is not a participant
	let identity_not_a_participant = peer1.create_identity().await;

	// push (without guard)
	let result = shared_co
		.push(&identity_not_a_participant, CO_CORE_NAME_CO, &CoAction::TagsInsert { tags: tags!("test": "123") })
		.await;
	tracing::info!(?result, "push-with-non-participant-without-guard");
	assert!(result.is_ok());

	// add guard
	let before_push_with_non_participant = shared_co
		.push(
			&identity,
			CO_CORE_NAME_CO,
			&CoAction::GuardCreate {
				guard: "participant".to_owned(),
				binary: create_default_guards().binary(CO_CORE_CO).unwrap(),
				tags: Default::default(),
			},
		)
		.await
		.unwrap();

	// push (with guard)
	let result = shared_co
		.push(&identity_not_a_participant, CO_CORE_NAME_CO, &CoAction::TagsInsert { tags: tags!("not": "allowed") })
		.await;
	tracing::info!(?result, "push-with-non-participant");
	assert!(result.is_err());

	// check state and heads don't has changed
	//  we always strip errored trailing heads
	//  this prevents the log being spammed with known invalid actions
	assert_eq!(shared_co.reducer_state().await, before_push_with_non_participant);
}

/// Test that joined actions that are rejected by a guard not modify the state and heads.
/// Make sure that a malicious peer is ignored by the other peers.
#[tokio::test]
async fn test_guard_join() {
	let mut instances = Instances::new("test_guard_join");
	let shared = SharedCo::create_with_peers(
		instances.create().await,
		instances
			.create_builder(|builder| builder.with_setting("feature", "co-guard-ignore"))
			.await,
		"shared",
	)
	.await;

	let (peer0_co, peer0_identity) = shared.reducer(0, "shared").await;
	let (peer1_co, _peer1_identity) = shared.reducer(1, "shared").await;

	// push
	peer0_co
		.push(&peer0_identity, CO_CORE_NAME_CO, &CoAction::TagsInsert { tags: tags!("hello": "world") })
		.await
		.unwrap();

	// create a second identity which is not a participant
	let identity_not_a_participant = shared.instance(0).create_identity().await;

	// push (without guard)
	let result = peer0_co
		.push(&peer0_identity, CO_CORE_NAME_CO, &CoAction::TagsInsert { tags: tags!("test": "123") })
		.await;
	tracing::info!(?result, "push-with-non-participant-without-guard");
	assert!(result.is_ok());

	// add guard
	let before_push_with_non_participant = peer0_co
		.push(
			&peer0_identity,
			CO_CORE_NAME_CO,
			&CoAction::GuardCreate {
				guard: "participant".to_owned(),
				binary: create_default_guards().binary(CO_CORE_CO).unwrap(),
				tags: Default::default(),
			},
		)
		.await
		.unwrap();

	// sync
	shared.sync("shared", 0, 1).await;

	// push (with guard)
	let result = peer1_co
		.push(&identity_not_a_participant, CO_CORE_NAME_CO, &CoAction::TagsInsert { tags: tags!("not": "allowed") })
		.await;
	tracing::info!(?result, "push-with-non-participant");
	assert!(result.is_ok()); // as we forced to ignore the guard using "co-guard-ignore"

	// sync
	shared.sync("shared", 1, 0).await;

	// check state and heads don't has changed
	//  we always strip errored trailing heads
	//  this prevents the log being spammed with known invalid actions
	assert_eq!(peer0_co.reducer_state().await, before_push_with_non_participant);
}
