// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use co_core_membership::{MembershipOptions, MembershipsAction};
use co_sdk::{
	state::{query_core, QueryExt},
	ApplicationBuilder, Identity, CO_CORE_NAME_MEMBERSHIP,
};

/// Verify that the reducer actor stays operational after a failed membership action.
///
/// Reproduces the scenario where `InviteAccept` is pushed on a membership that is
/// already `Active`. The action is expected to fail, but subsequent pushes to the
/// same reducer must still succeed.
#[tokio::test]
async fn test_reducer_push_error() {
	let application = ApplicationBuilder::new_memory("test_reducer_push_error")
		.without_keychain()
		.build()
		.await
		.expect("application");

	let local_identity = application.local_identity();
	let local_co = application.local_co_reducer().await.unwrap();

	// the local CO automatically has an Active membership for CO_ID_LOCAL with
	// the device identity, so create a second membership that we control:
	// first create it via Join (which sets it to Active)
	let test_co_id = "test-co";
	let did = local_identity.identity().to_owned();
	local_co
		.push(
			&local_identity,
			CO_CORE_NAME_MEMBERSHIP,
			&MembershipsAction::Join { id: test_co_id.into(), did: did.clone(), options: MembershipOptions::default() },
		)
		.await
		.expect("join should succeed");

	// verify the membership is Active
	let (storage, memberships) = query_core(CO_CORE_NAME_MEMBERSHIP).execute_reducer(&local_co).await.unwrap();
	let membership = memberships
		.memberships
		.get(&storage, &test_co_id.into())
		.await
		.unwrap()
		.expect("membership exists");
	assert_eq!(
		membership.did.get(&did).copied(),
		Some(co_core_membership::MembershipState::Active),
		"membership should be Active after Join"
	);

	let state_before_failure = local_co.reducer_state().await;

	// push InviteAccept on the already-Active membership — this should fail
	let failed_result = local_co
		.push(
			&local_identity,
			CO_CORE_NAME_MEMBERSHIP,
			&MembershipsAction::InviteAccept {
				id: test_co_id.into(),
				did: did.clone(),
				options: MembershipOptions::default(),
			},
		)
		.await;
	assert!(failed_result.is_err(), "InviteAccept on Active membership should fail");

	// the reducer actor must still be running
	assert!(local_co.is_running(), "reducer should still be running after failed action");

	// push another valid action to prove the reducer is still operational
	let second_co_id = "second-test-co";
	let result = local_co
		.push(
			&local_identity,
			CO_CORE_NAME_MEMBERSHIP,
			&MembershipsAction::Join {
				id: second_co_id.into(),
				did: did.clone(),
				options: MembershipOptions::default(),
			},
		)
		.await;
	assert!(result.is_ok(), "subsequent push should succeed: {:?}", result.err());

	// verify the second membership was actually created
	let (storage, memberships) = query_core(CO_CORE_NAME_MEMBERSHIP).execute_reducer(&local_co).await.unwrap();
	let membership = memberships
		.memberships
		.get(&storage, &second_co_id.into())
		.await
		.unwrap()
		.expect("second membership exists");
	assert_eq!(
		membership.did.get(&did).copied(),
		Some(co_core_membership::MembershipState::Active),
		"second membership should be Active"
	);

	// verify the original membership was not corrupted
	let membership = memberships
		.memberships
		.get(&storage, &test_co_id.into())
		.await
		.unwrap()
		.expect("original membership still exists");
	assert_eq!(
		membership.did.get(&did).copied(),
		Some(co_core_membership::MembershipState::Active),
		"original membership should still be Active"
	);

	// verify state advanced (the failed action should not have left ghost state)
	let state_after = local_co.reducer_state().await;
	assert_ne!(
		state_before_failure, state_after,
		"reducer state should have advanced after the successful second push"
	);
}
