use crate::helper::shared_co::wait_membership_state;
use co_core_co::CoAction;
use co_core_membership::{MembershipState, MembershipsAction};
use co_sdk::{CoOptions, CoReducerFactory, CreateCo, Identity, CO_CORE_NAME_CO, CO_CORE_NAME_MEMBERSHIP};
use helper::instance::Instances;
use std::time::Duration;
use tokio::time::timeout;

pub mod helper;

#[tokio::test]
async fn test_try_co_reducer_waits_after_invite_accept() {
	let mut instances = Instances::new("test_wait");
	let mut peer1 = instances.create().await;
	let mut peer2 = instances.create().await;

	// network
	let (_network1, _network2) = Instances::networking(&mut peer1, &mut peer2, true, true).await;

	// create identity
	let identity1 = peer1.create_identity().await;
	let identity2 = peer2.create_identity().await;

	// create co
	let new_co = peer2
		.application
		.create_co(identity2.clone(), CreateCo::generate("test".to_owned()))
		.await
		.unwrap();
	new_co
		.push(
			&identity2,
			CO_CORE_NAME_CO,
			&CoAction::ParticipantInvite { participant: identity1.identity().to_owned(), tags: Default::default() },
		)
		.await
		.unwrap();

	// wait for peer1 to receive the invite
	let peer1_invite = wait_membership_state(peer1.application.actions(), [MembershipState::Invite]);
	let _invite = timeout(Duration::from_secs(10), peer1_invite).await.unwrap().unwrap();

	// peer1 accepts the invite
	let local_co1 = peer1.application.local_co_reducer().await.unwrap();
	local_co1
		.push(
			&identity1,
			CO_CORE_NAME_MEMBERSHIP,
			&MembershipsAction::InviteAccept {
				id: new_co.id().clone(),
				did: identity1.identity().to_owned(),
				options: Default::default(),
			},
		)
		.await
		.unwrap();

	// this should wait for Join → Active, but fails with "No active membership"
	let options = CoOptions::default().with_wait(Some(Duration::from_secs(10)));
	let reducer = peer1
		.application
		.context()
		.try_co_reducer_with_options(new_co.id(), options)
		.await;
	assert!(reducer.is_ok(), "should wait for membership to become Active: {:?}", reducer.err());
}
