// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

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
	shared_co
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
}
