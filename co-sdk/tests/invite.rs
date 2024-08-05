use co_core_co::CoAction;
use co_core_membership::{MembershipState, MembershipsAction};
use co_sdk::{
	Action, CoId, CreateCo, Did, Identity, Observable, CO_CORE_NAME_CO, CO_CORE_NAME_MEMBERSHIP, CO_ID_LOCAL,
};
use futures::{join, StreamExt};
use helper::instance::Instance;
use std::{collections::BTreeSet, future::ready};

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
	let mut peer1 = Instance::new(1).await;
	peer1.application.create_network(false).await.unwrap();
	let mut peer2 = Instance::new(2).await;
	peer2.application.create_network(false).await.unwrap();

	// networks
	let _network1 = peer1.application.network().unwrap();
	let network2 = peer2.application.network().unwrap();

	// // connect
	// network2
	// 	.dail(network1.peer_id(), network1.listeners().await.unwrap())
	// 	.await
	// 	.unwrap();
	// network1
	// 	.dail(network2.peer_id(), network2.listeners().await.unwrap())
	// 	.await
	// 	.unwrap();

	// create identity
	let identity1 = peer1.create_identity().await;
	let identity2 = peer2.create_identity().await;

	// peer1: create shared co
	let shared_co =
		tracing::trace_span!("peer1: created shared co", application = peer1.application.settings().identifier)
			.in_scope(|| async {
				peer1
					.application
					.create_co(
						identity1.clone(),
						CreateCo { id: "shared".into(), algorithm: None, name: "shared".to_owned() },
					)
					.await
					.unwrap()
			})
			.await;

	// peer1: invite peer2
	let peer1_invite =
		tracing::trace_span!("peer1: added other peer identity", application = peer1.application.settings().identifier)
			.in_scope(|| async {
				shared_co
					.push(
						&identity1,
						CO_CORE_NAME_CO,
						&CoAction::ParticipantInvite {
							participant: identity2.identity().to_owned(),
							tags: Default::default(),
						},
					)
					.await
					.unwrap();
			});

	// peer1: invite-sent/error
	let peer1_invite_sent = peer1
		.application
		.actions()
		.filter_map(|action| {
			ready(match action {
				Action::InviteSent { co, participant, peer } => Some(Ok((co, participant, peer))),
				Action::Error { err } => Some(Err(err)),
				_ => None,
			})
		})
		.take(1)
		.collect::<Vec<_>>();
	let peer1_invite_sent = async move {
		peer1_invite_sent
			.await
			.into_iter()
			.next()
			.expect("not empty")
			.expect("invite sent")
	};

	// peer2: membership-invite
	let peer2_membership_invite = wait_membership_state(peer2.application.actions(), [MembershipState::Invite]);
	let peer2_membership_invite = async move { peer2_membership_invite.await.expect("not empty") };

	// check
	let ((_, membership_co, membership_participant), (_, invited_participant, invited_peer), _) =
		join!(peer2_membership_invite, peer1_invite_sent, peer1_invite);
	assert_eq!(invited_participant, identity2.identity());
	assert_eq!(invited_peer, network2.peer_id());
	assert_eq!(membership_co, CoId::from("shared"));
	assert_eq!(membership_participant, identity2.identity());

	// peer2: join
	//  set membership to join and wait for membership set to active when join is complete
	let local_co = peer2.application.local_co_reducer().await.unwrap();
	let payload = MembershipsAction::ChangeMembershipState {
		id: "shared".into(),
		did: identity2.identity().to_owned(),
		membership_state: MembershipState::Join,
	};
	let (push, membership_state) = join!(
		local_co.push(&identity2, CO_CORE_NAME_MEMBERSHIP, &payload),
		wait_membership_state(peer2.application.actions(), [MembershipState::Active, MembershipState::Invite]),
	);
	push.unwrap();
	assert_eq!(
		membership_state.unwrap(),
		(MembershipState::Active, CoId::from("shared"), identity2.identity().to_owned())
	);

	// peer2: read state
	let peer2_shared_co = peer2.application.co_reducer(CoId::from("shared")).await.unwrap().unwrap();
	assert_eq!(peer2_shared_co.reducer_state().await, shared_co.reducer_state().await);
}

async fn wait_membership_state(
	actions: Observable<Action>,
	state: impl IntoIterator<Item = MembershipState>,
) -> Option<(MembershipState, CoId, Did)> {
	let state: BTreeSet<MembershipState> = state.into_iter().collect();
	actions
		.filter_map(move |action| {
			let state = state.clone();
			async move {
				match action {
					Action::CoreAction { co, context: _, action, cid: _ }
						if co.as_str() == CO_ID_LOCAL && action.core == CO_CORE_NAME_MEMBERSHIP =>
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
