// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use anyhow::anyhow;
use co_core_co::{CoAction, ParticipantState};
use co_core_membership::{MembershipOptions, MembershipState, MembershipsAction};
use co_network::NetworkApi;
use co_sdk::{
	update_co, Action, CoId, CoReducer, CreateCo, Did, Identity, NetworkSettings, CO_CORE_NAME_CO,
	CO_CORE_NAME_MEMBERSHIP, CO_ID_LOCAL,
};
use futures::{join, Stream, StreamExt};
use helper::instance::Instances;
use std::{collections::BTreeSet, future::ready, time::Duration};
use tokio::time::timeout;
use tracing::{info_span, Instrument};

pub mod helper;

/// Invite two participants at once and verify both receive their invite.
///
/// Steps:
/// - P1: Create shared CO
/// - P1: Invite P2 and P3
/// - P2: Join by accepting invite
/// - P3: Join by accepting invite
/// - All: Verify all participants are active
#[tokio::test]
async fn test_invite_multiple_participants() {
	let timeout_duration = Duration::from_secs(60);

	let mut instances = Instances::new("test_invite_multiple_participants");
	let mut peer1 = instances.create().await;
	let mut peer2 = instances.create().await;
	let mut peer3 = instances.create().await;

	// network: create for all three peers
	peer1
		.application
		.create_network(NetworkSettings::default().with_localhost())
		.await
		.unwrap();
	peer2
		.application
		.create_network(NetworkSettings::default().with_localhost())
		.await
		.unwrap();
	peer3
		.application
		.create_network(NetworkSettings::default().with_localhost())
		.await
		.unwrap();

	let network1 = peer1.application.context().network().await.unwrap();
	let network2 = peer2.application.context().network().await.unwrap();
	let network3 = peer3.application.context().network().await.unwrap();

	// dial all peers
	dial(&network1, &network2).await;
	dial(&network1, &network3).await;
	dial(&network2, &network3).await;

	// create identities
	let identity1 = peer1.create_identity().await;
	let identity2 = peer2.create_identity().await;
	let identity3 = peer3.create_identity().await;

	// peer1: create shared co
	let shared_co = async {
		peer1
			.application
			.create_co(identity1.clone(), CreateCo::new("shared", None).with_public(true))
			.await
			.unwrap()
	}
	.instrument(info_span!("peer1: create shared co"))
	.await;

	// peer1: invite peer2 and peer3
	let peer1_invite = async {
		shared_co
			.push(
				&identity1,
				CO_CORE_NAME_CO,
				&CoAction::ParticipantInvite { participant: identity2.identity().to_owned(), tags: Default::default() },
			)
			.await
			.unwrap();
		shared_co
			.push(
				&identity1,
				CO_CORE_NAME_CO,
				&CoAction::ParticipantInvite { participant: identity3.identity().to_owned(), tags: Default::default() },
			)
			.await
			.unwrap();
	}
	.instrument(info_span!("peer1: invite peer2 and peer3"));

	// peer1: wait for 2 InviteSent actions
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
		.take(2)
		.collect::<Vec<_>>();
	let peer1_invite_sent = async move {
		timeout(timeout_duration, peer1_invite_sent)
			.await
			.expect("peer1 to send both invites in time")
	};

	// peer2: wait for membership invite
	let peer2_membership_invite = wait_membership_state(peer2.application.actions(), [MembershipState::Invite]);
	let peer2_membership_invite = async move {
		timeout(timeout_duration, peer2_membership_invite)
			.await
			.expect("peer2 to recv invite in time")
			.expect("peer2 invite not empty")
	};

	// peer3: wait for membership invite
	let peer3_membership_invite = wait_membership_state(peer3.application.actions(), [MembershipState::Invite]);
	let peer3_membership_invite = async move {
		timeout(timeout_duration, peer3_membership_invite)
			.await
			.expect("peer3 to recv invite in time")
			.expect("peer3 invite not empty")
	};

	// run invites and wait for all
	let (_, invite_sent_results, peer2_invite, peer3_invite) =
		join!(peer1_invite, peer1_invite_sent, peer2_membership_invite, peer3_membership_invite);

	// verify both invites were sent
	let invite_sent_results: Vec<_> = invite_sent_results.into_iter().collect::<Result<Vec<_>, _>>().unwrap();
	assert_eq!(invite_sent_results.len(), 2);

	// verify peer2 received invite for the right co and did
	assert_eq!(peer2_invite.1, CoId::from("shared"));
	assert_eq!(peer2_invite.2, identity2.identity());

	// verify peer3 received invite for the right co and did
	assert_eq!(peer3_invite.1, CoId::from("shared"));
	assert_eq!(peer3_invite.2, identity3.identity());

	// peer2: join
	async {
		let local_co = peer2.application.local_co_reducer().await.unwrap();
		let payload = MembershipsAction::InviteAccept {
			id: "shared".into(),
			did: identity2.identity().to_owned(),
			options: MembershipOptions::default(),
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
	.instrument(info_span!("peer2: join"))
	.await;

	// peer3: join
	async {
		let local_co = peer3.application.local_co_reducer().await.unwrap();
		let payload = MembershipsAction::InviteAccept {
			id: "shared".into(),
			did: identity3.identity().to_owned(),
			options: MembershipOptions::default(),
		};
		let (push, membership_state) = join!(local_co.push(&identity3, CO_CORE_NAME_MEMBERSHIP, &payload), async {
			timeout(
				timeout_duration,
				wait_membership_state(peer3.application.actions(), [MembershipState::Active, MembershipState::Invite]),
			)
			.await
			.expect("peer3 to join in time")
		});
		push.unwrap();
		assert_eq!(
			membership_state.unwrap(),
			(MembershipState::Active, CoId::from("shared"), identity3.identity().to_owned())
		);
	}
	.instrument(info_span!("peer3: join"))
	.await;

	// force sync all peers to propagate participant state
	let peer2_shared_co = peer2.application.co_reducer(CoId::from("shared")).await.unwrap().unwrap();
	let peer3_shared_co = peer3.application.co_reducer(CoId::from("shared")).await.unwrap().unwrap();

	async {
		update_co(
			peer2.application.handle(),
			&peer2_shared_co,
			&identity2,
			network1.local_peer_id(),
			Duration::from_secs(30),
		)
		.await
		.unwrap();
	}
	.instrument(info_span!("peer2: sync to peer1"))
	.await;

	async {
		update_co(
			peer3.application.handle(),
			&peer3_shared_co,
			&identity3,
			network1.local_peer_id(),
			Duration::from_secs(30),
		)
		.await
		.unwrap();
	}
	.instrument(info_span!("peer3: sync to peer1"))
	.await;

	// verify all participants are active on all instances
	let identity1_did = identity1.identity().to_owned();
	let identity2_did = identity2.identity().to_owned();
	let identity3_did = identity3.identity().to_owned();

	// peer1 (creator)
	assert_eq!(get_participant_state(&shared_co, &identity1_did).await.unwrap(), ParticipantState::Active);
	assert_eq!(get_participant_state(&shared_co, &identity2_did).await.unwrap(), ParticipantState::Active);
	assert_eq!(get_participant_state(&shared_co, &identity3_did).await.unwrap(), ParticipantState::Active);

	// peer2
	assert_eq!(get_participant_state(&peer2_shared_co, &identity1_did).await.unwrap(), ParticipantState::Active);
	assert_eq!(get_participant_state(&peer2_shared_co, &identity2_did).await.unwrap(), ParticipantState::Active);
	assert_eq!(get_participant_state(&peer2_shared_co, &identity3_did).await.unwrap(), ParticipantState::Active);

	// peer3
	assert_eq!(get_participant_state(&peer3_shared_co, &identity1_did).await.unwrap(), ParticipantState::Active);
	assert_eq!(get_participant_state(&peer3_shared_co, &identity2_did).await.unwrap(), ParticipantState::Active);
	assert_eq!(get_participant_state(&peer3_shared_co, &identity3_did).await.unwrap(), ParticipantState::Active);
}

async fn dial(from: &NetworkApi, to: &NetworkApi) {
	from.dial(Some(to.local_peer_id()), to.listeners(true, false).await.unwrap().into_iter().collect())
		.await
		.unwrap();
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
					Action::CoreAction { co, storage: _, context: _, action, cid: _, head: _ }
						if co.as_str() == CO_ID_LOCAL && CO_CORE_NAME_MEMBERSHIP == action.core =>
					{
						let membership_action: MembershipsAction = action.get_payload().ok()?;
						match membership_action {
							MembershipsAction::Join { id, did, .. } if state.contains(&MembershipState::Active) => {
								Some((MembershipState::Active, id, did))
							},
							MembershipsAction::Invited { id, did, .. } if state.contains(&MembershipState::Invite) => {
								Some((MembershipState::Invite, id, did))
							},
							MembershipsAction::JoinRequest { id, did, .. }
								if state.contains(&MembershipState::Join) =>
							{
								Some((MembershipState::Join, id, did))
							},
							MembershipsAction::JoinPending { id, did, .. }
								if state.contains(&MembershipState::Pending) =>
							{
								Some((MembershipState::Pending, id, did))
							},
							MembershipsAction::InviteAccept { id, did, .. }
								if state.contains(&MembershipState::Join) =>
							{
								Some((MembershipState::Join, id, did))
							},
							MembershipsAction::Deactivate { id, did } if state.contains(&MembershipState::Inactive) => {
								Some((MembershipState::Inactive, id, did))
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

async fn get_participant_state(co: &CoReducer, participant: &Did) -> Result<ParticipantState, anyhow::Error> {
	let (storage, co) = co.co().await?;
	let participant = co
		.participants
		.get(&storage, participant)
		.await?
		.ok_or(anyhow!("Not found: {}", participant))?;
	Ok(participant.state)
}
