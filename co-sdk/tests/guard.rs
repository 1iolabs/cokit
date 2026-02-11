use co_core_co::CoAction;
use co_sdk::{tags, CreateCo, Guards, CO_CORE_CO, CO_CORE_NAME_CO};
use helper::instance::Instances;

pub mod helper;

#[tokio::test]
async fn test_guard() {
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
				binary: Guards::default().binary(CO_CORE_CO).unwrap(),
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
