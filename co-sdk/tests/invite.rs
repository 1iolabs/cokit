use co_core_co::CoAction;
use co_core_membership::{MembershipState, MembershipsAction};
use co_sdk::{update_co, Action, CoId, CreateCo, Did, Identity, CO_CORE_NAME_CO, CO_CORE_NAME_MEMBERSHIP, CO_ID_LOCAL};
use co_storage::Algorithm;
use futures::{join, Stream, StreamExt};
use helper::instance::Instances;
use std::{collections::BTreeSet, future::ready, time::Duration};
use tokio::time::timeout;
use tracing::{info_span, Instrument};

pub mod helper;

/// Invite/Join
///
/// Steps:
/// - P1: Create (unencrypted) shared CO
/// - P1: Invite P2
/// - P2: Join by accepting manual invite request
/// - P2: Read state
#[tokio::test]
async fn test_invite() {
	let timeout_duration = Duration::from_secs(60);

	let mut instances = Instances::new("test_invite");
	let mut peer1 = instances.create().await;
	let mut peer2 = instances.create().await;

	// network
	let (network1, network2) = Instances::networking(&mut peer1, &mut peer2).await;

	// create identity
	let identity1 = peer1.create_identity().await;
	let identity2 = peer2.create_identity().await;

	// peer1: create shared co
	let shared_co = async {
		peer1
			.application
			.create_co(identity1.clone(), CreateCo { id: "shared".into(), algorithm: None, name: "shared".to_owned() })
			.await
			.unwrap()
	}
	.instrument(info_span!("peer1: created shared co", application = peer1.application.settings().identifier))
	.await;

	// peer1: invite peer2
	let peer1_invite = async {
		shared_co
			.push(
				&identity1,
				CO_CORE_NAME_CO,
				&CoAction::ParticipantInvite { participant: identity2.identity().to_owned(), tags: Default::default() },
			)
			.await
			.unwrap();
	}
	.instrument(info_span!("peer1: added other peer identity", application = peer1.application.settings().identifier));

	// peer1: invite-sent/error
	let peer1_invite_sent = peer1
		.application
		.actions()
		.filter_map(|action| {
			ready(match action {
				Action::InviteSent { co, to, peer } => Some(Ok((co, to, peer))),
				Action::Error { err } => Some(Err(err)),
				_ => None,
			})
		})
		.take(1)
		.collect::<Vec<_>>();
	let peer1_invite_sent = async move {
		timeout(timeout_duration, peer1_invite_sent)
			.await
			.expect("peer1 to send invite in time")
			.into_iter()
			.next()
			.expect("not empty")
			.expect("invite sent")
	};

	// peer2: membership-invite
	let peer2_membership_invite = wait_membership_state(peer2.application.actions(), [MembershipState::Invite]);
	let peer2_membership_invite = async move {
		timeout(timeout_duration, peer2_membership_invite)
			.await
			.expect("peer2 to recv invite in time")
			.expect("not empty")
	};

	// check
	let ((_, membership_co, membership_participant), (_, invited_participant, invited_peer), _) =
		join!(peer2_membership_invite, peer1_invite_sent, peer1_invite);
	assert_eq!(invited_participant, identity2.identity());
	assert_eq!(invited_peer, network2.local_peer_id());
	assert_eq!(membership_co, CoId::from("shared"));
	assert_eq!(membership_participant, identity2.identity());

	// peer2: join
	//  set membership to join and wait for membership set to active when join is complete
	async {
		let local_co = peer2.application.local_co_reducer().await.unwrap();
		let payload = MembershipsAction::ChangeMembershipState {
			id: "shared".into(),
			did: identity2.identity().to_owned(),
			membership_state: MembershipState::Join,
		};
		let (push, membership_state) = join!(local_co.push(&identity2, CO_CORE_NAME_MEMBERSHIP, &payload), async {
			timeout(
				timeout_duration,
				wait_membership_state(peer2.application.actions(), [MembershipState::Active, MembershipState::Invite]),
			)
			.await
			.expect("peer2 to join in time")
		});
		push.unwrap();
		assert_eq!(
			membership_state.unwrap(),
			(MembershipState::Active, CoId::from("shared"), identity2.identity().to_owned())
		);
	}
	.instrument(info_span!("peer2: join", application = peer2.application.settings().identifier))
	.await;

	// peer2: force sync (needed because of the paricipant state update)
	let peer2_shared_co = peer2.application.co_reducer(CoId::from("shared")).await.unwrap().unwrap();
	async {
		update_co(peer2.application.handle(), &peer2_shared_co, &identity2, network1.local_peer_id(), timeout_duration)
			.await
			.unwrap();
	}
	.instrument(info_span!("peer2: force sync", application = peer2.application.settings().identifier))
	.await;

	// peer2: read state
	assert_eq!(peer2_shared_co.reducer_state().await, shared_co.reducer_state().await);
	let (_, co) = shared_co.co().await.unwrap();
	let (_, peer2_co) = peer2_shared_co.co().await.unwrap();
	assert_eq!(peer2_co, co);
}

/// Invite/Join
///
/// Steps:
/// - P1: Create (encrypted) shared CO
/// - P1: Invite P2
/// - P2: Join by accepting manual invite request
/// - P2: Read state
#[tokio::test]
async fn test_invite_encrypted() {
	let timeout_duration = Duration::from_secs(60);

	// instance
	let mut instances = Instances::new("test_invite");
	let mut peer1 = instances.create().await;
	let mut peer2 = instances.create().await;

	// network
	let (network1, network2) = Instances::networking(&mut peer1, &mut peer2).await;

	// create identity
	let identity1 = peer1.create_identity().await;
	let identity2 = peer2.create_identity().await;

	// peer1: create shared co
	let shared_co = async {
		peer1
			.application
			.create_co(
				identity1.clone(),
				CreateCo { id: "shared".into(), algorithm: Some(Algorithm::default()), name: "shared".to_owned() },
			)
			.await
			.unwrap()
	}
	.instrument(info_span!("peer1: created shared co", application = peer1.application.settings().identifier))
	.await;

	// peer1: invite peer2
	let peer1_invite = async {
		shared_co
			.push(
				&identity1,
				CO_CORE_NAME_CO,
				&CoAction::ParticipantInvite { participant: identity2.identity().to_owned(), tags: Default::default() },
			)
			.await
			.unwrap();
	}
	.instrument(info_span!("peer1: added other peer identity", application = peer1.application.settings().identifier));

	// peer1: invite-sent/error
	let peer1_invite_sent = peer1
		.application
		.actions()
		.filter_map(|action| {
			ready(match action {
				Action::InviteSent { co, to, peer } => Some(Ok((co, to, peer))),
				Action::Error { err } => Some(Err(err)),
				_ => None,
			})
		})
		.take(1)
		.collect::<Vec<_>>();
	let peer1_invite_sent = async move {
		timeout(timeout_duration, peer1_invite_sent)
			.await
			.expect("peer1 to send invite in time")
			.into_iter()
			.next()
			.expect("not empty")
			.expect("invite sent")
	};

	// peer2: membership-invite
	let peer2_membership_invite = wait_membership_state(peer2.application.actions(), [MembershipState::Invite]);
	let peer2_membership_invite = async move {
		timeout(timeout_duration, peer2_membership_invite)
			.await
			.expect("peer2 to recv invite in time")
			.expect("not empty")
	};

	// check
	let ((_, membership_co, membership_participant), (_, invited_participant, invited_peer), _) =
		join!(peer2_membership_invite, peer1_invite_sent, peer1_invite);
	assert_eq!(invited_participant, identity2.identity());
	assert_eq!(invited_peer, network2.local_peer_id());
	assert_eq!(membership_co, CoId::from("shared"));
	assert_eq!(membership_participant, identity2.identity());

	// peer2: join
	//  set membership to join and wait for membership set to active when join is complete
	async {
		let local_co = peer2.application.local_co_reducer().await.unwrap();
		let payload = MembershipsAction::ChangeMembershipState {
			id: "shared".into(),
			did: identity2.identity().to_owned(),
			membership_state: MembershipState::Join,
		};
		let (push, membership_state) = join!(
			local_co.push(&identity2, CO_CORE_NAME_MEMBERSHIP, &payload),
			wait_membership_state(peer2.application.actions(), [MembershipState::Active]),
		);
		push.unwrap();
		assert_eq!(
			membership_state.unwrap(),
			(MembershipState::Active, CoId::from("shared"), identity2.identity().to_owned())
		);
	}
	.instrument(info_span!("peer2: join", application = peer2.application.settings().identifier))
	.await;

	// peer2: force sync (needed because of the paricipant state update)
	let peer2_shared_co = peer2.application.co_reducer(CoId::from("shared")).await.unwrap().unwrap();
	async {
		update_co(peer2.application.handle(), &peer2_shared_co, &identity2, network1.local_peer_id(), timeout_duration)
			.await
			.unwrap();
	}
	.instrument(info_span!("peer2: force sync", application = peer2.application.settings().identifier))
	.await;

	// peer2: read state
	assert_eq!(peer2_shared_co.reducer_state().await, shared_co.reducer_state().await);
	let (_, co) = shared_co.co().await.unwrap();
	let (_, peer2_co) = peer2_shared_co.co().await.unwrap();
	assert_eq!(peer2_co, co);
}

async fn wait_membership_state(
	actions: impl Stream<Item = Action>,
	state: impl IntoIterator<Item = MembershipState>,
) -> Option<(MembershipState, CoId, Did)> {
	let state: BTreeSet<MembershipState> = state.into_iter().collect();
	actions
		.filter_map(move |action| {
			let state = state.clone();
			async move {
				match action {
					Action::CoreAction { co, storage: _, context: _, action, cid: _ }
						if co.as_str() == CO_ID_LOCAL && CO_CORE_NAME_MEMBERSHIP == action.core =>
					{
						let mambership_action: MembershipsAction = action.get_payload().ok()?;
						match mambership_action {
							MembershipsAction::Join(membership) if state.contains(&membership.membership_state) => {
								Some((membership.membership_state, membership.id, membership.did))
							},
							MembershipsAction::ChangeMembershipState { id, did, membership_state }
								if state.contains(&membership_state) =>
							{
								Some((membership_state, id, did))
							},
							_ => None,
						}
					},
					_ => None,
				}
			}
		})
		.take(1)
		.collect::<Vec<_>>()
		.await
		.into_iter()
		.next()
}
