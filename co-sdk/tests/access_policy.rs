// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

#![cfg(feature = "guard")]

use async_trait::async_trait;
use co_core_co::CoAction;
use co_core_membership::{MembershipOptions, MembershipsAction};
use co_network::connections::PeerRelateCoAction;
use co_primitives::{CoId, CoInviteMetadata};
use co_sdk::{
	request_co_state, tags, update_co, Action, BlockStorageExt, CoAccessPolicy, CoReducerState, CreateCo, Identity,
	KeyRequestAction, KnownTags, CO_CORE_NAME_CO, CO_CORE_NAME_MEMBERSHIP, CO_ID_LOCAL,
};
use futures::StreamExt;
use helper::instance::Instances;
#[allow(unused_imports)]
use helper::shared_co::SharedCo;
use std::time::Duration;
use tokio::time::timeout;

pub mod helper;

struct AllowAll;

#[async_trait]
impl CoAccessPolicy for AllowAll {
	async fn check_access(&self, _co: &CoId, _requester: &str) -> Result<bool, anyhow::Error> {
		Ok(true)
	}
}

/// Test that a removed participant can still request a key when a CoAccessPolicy is configured.
///
/// Steps:
/// - Create two peers (peer1 with access policy, peer2 standard)
/// - Create encrypted shared CO, peer2 joins
/// - Peer1 removes peer2 (sets to Inactive)
/// - Peer2 dispatches a KeyRequest
/// - Access policy on peer1 grants access → key request succeeds
#[tokio::test]
async fn test_allows_removed_participant() {
	let timeout_duration = Duration::from_secs(30);
	let mut instances = Instances::new("test_access_policy");

	// peer1: with access policy
	let peer1 = instances.create_builder(|b| b.with_access_policy(AllowAll)).await;
	let peer2 = instances.create().await;
	let shared_co = SharedCo::create_with_peers(peer1, peer2, "shared").await;

	// make sure initial sync is done
	shared_co.sync("shared", 0, 1).await;
	shared_co.sync("shared", 1, 0).await;
	tracing::info!("test-sync");
	assert_eq!(
		shared_co.reducer(1, "shared").await.0.reducer_state().await,
		shared_co.reducer(0, "shared").await.0.reducer_state().await
	);

	// peer1: remove peer2
	let (peer1_co, identity1) = shared_co.reducer(0, "shared").await;
	let peer2_did = shared_co.identity(1).identity().to_owned();
	peer1_co
		.push(
			&identity1,
			CO_CORE_NAME_CO,
			&CoAction::ParticipantRemove { participant: peer2_did, tags: Default::default() },
		)
		.await
		.unwrap();
	shared_co.sync("shared", 0, 1).await;
	shared_co.sync("shared", 1, 0).await;

	// peer2: dispatch key request
	let peer2_app = shared_co.application(1);
	let key_request_complete = peer2_app
		.actions()
		.filter_map(|action| async move {
			match action {
				Action::KeyRequestComplete(_request, result) => Some(result),
				_ => None,
			}
		})
		.take(1)
		.collect::<Vec<_>>();
	peer2_app
		.handle()
		.dispatch(co_sdk::ApplicationMessage::Dispatch(Action::KeyRequest(KeyRequestAction {
			co: CoId::from("shared"),
			parent_co: CoId::from(CO_ID_LOCAL),
			key: None,
			from: None,
			network: None,
		})))
		.unwrap();

	// wait for key request to complete
	let results = timeout(timeout_duration, key_request_complete)
		.await
		.expect("key request to complete in time");
	let result = results.into_iter().next().expect("one result");
	assert!(result.is_ok(), "key request should succeed via access policy, got: {:?}", result);
	tracing::info!("test-complete");
}

/// Test that an unrelated peer (never invited) can join a CO via `request_co_state` when the
/// owner has an AllowAll access policy and the CO is unencrypted.
///
/// Steps:
/// - Create two peers (peer1 with AllowAll policy, peer2 standard), networking between them
/// - Peer1: create unencrypted shared CO
/// - Peer2: request_co_state → (state, heads)
/// - Peer2: create Active membership in local CO with state/heads from step above
/// - Peer2: open CO via co_reducer() and sync via update_co
/// - Verify: peer2 CO state matches peer1 CO state
#[tokio::test]
async fn test_unrelated_peer_joins() {
	let timeout_duration = Duration::from_secs(30);
	let mut instances = Instances::new("test_access_policy_unrelated");

	// peer1: with access policy
	let mut peer1 = instances.create_builder(|b| b.with_access_policy(AllowAll)).await;
	let mut peer2 = instances.create().await;

	// network
	let (network1, _network2) = Instances::networking(&mut peer1, &mut peer2, true, true).await;

	// create identities
	let identity1 = peer1.create_identity().await;
	let identity2 = peer2.create_identity().await;

	// peer1: create unencrypted shared CO
	let shared_co = peer1
		.application
		.create_co(identity1.clone(), CreateCo::new("shared", None).with_algorithm(None))
		.await
		.unwrap();
	tracing::info!("peer1: created unencrypted shared CO");

	// peer2: request state from peer1
	let (state, heads) = timeout(
		timeout_duration,
		request_co_state(
			peer2.application.handle(),
			&CoId::from("shared"),
			&identity2,
			network1.local_peer_id(),
			peer2.application.context().date(),
			Duration::from_secs(10),
		),
	)
	.await
	.expect("request_co_state to complete in time")
	.expect("request_co_state to succeed");
	tracing::info!(?state, ?heads, "peer2: received state from peer1");

	// peer2: create Active membership in local CO
	let local_co = peer2.application.local_co_reducer().await.unwrap();
	let reducer_state = CoReducerState::new_weak(Some(state), heads);
	let co_state = reducer_state
		.to_external_co_state(&local_co.storage())
		.await
		.expect("to_external_co_state")
		.expect("co_state");
	local_co
		.push(
			&identity2,
			CO_CORE_NAME_MEMBERSHIP,
			&MembershipsAction::Join {
				id: CoId::from("shared"),
				did: identity2.identity().to_owned(),
				options: MembershipOptions::default().with_added_state(co_state),
			},
		)
		.await
		.unwrap();
	tracing::info!("peer2: created Active membership");

	// peer2: associate peer1 with the shared CO so block fetching can find it
	let connections = peer2.application.context().network_connections().await.expect("connections");
	connections
		.dispatch(PeerRelateCoAction {
			co: CoId::from("shared"),
			peer_id: network1.local_peer_id(),
			did: Some(identity1.identity().to_owned()),
			time: std::time::Instant::now(),
		})
		.ok();

	// peer2: open CO and sync
	let peer2_co = peer2
		.application
		.co_reducer(CoId::from("shared"))
		.await
		.expect("co_reducer")
		.expect("co exists after membership");
	update_co(peer2.application.handle(), &peer2_co, &identity2, network1.local_peer_id(), Duration::from_secs(10))
		.await
		.expect("update_co");
	tracing::info!("peer2: synced CO");

	// verify: states match
	assert_eq!(peer2_co.reducer_state().await, shared_co.reducer_state().await);
	tracing::info!("test-complete");
}

/// Test that an unrelated peer can auto-fetch CO state via the `pending_resolve` epic when
/// creating a membership with `CoInviteMetadata` tags and Pending state.
///
/// Steps:
/// - Create two peers (peer1 with AllowAll policy, peer2 standard), networking between them
/// - Peer1: create unencrypted shared CO
/// - Peer2: create CoInviteMetadata with peer1's PeerId, store in local CO tags
/// - Peer2: create Pending membership with empty state + metadata tags
/// - Wait for Pending → Active transition (epic resolves state from network)
/// - Peer2: open CO via co_reducer() and verify state matches
#[tokio::test]
async fn test_unrelated_peer_auto_state() {
	let timeout_duration = Duration::from_secs(30);
	let mut instances = Instances::new("test_access_policy_auto_state");

	// peer1: with access policy
	let mut peer1 = instances.create_builder(|b| b.with_access_policy(AllowAll)).await;
	let mut peer2 = instances.create().await;

	// network
	let (network1, _network2) = Instances::networking(&mut peer1, &mut peer2, true, true).await;

	// create identities
	let identity1 = peer1.create_identity().await;
	let identity2 = peer2.create_identity().await;

	// peer1: create unencrypted shared CO
	let shared_co = peer1
		.application
		.create_co(identity1.clone(), CreateCo::new("shared", None).with_algorithm(None))
		.await
		.unwrap();
	tracing::info!("peer1: created unencrypted shared CO");

	// peer2: create CoInviteMetadata and store in local CO
	let local_co = peer2.application.local_co_reducer().await.unwrap();
	let metadata = CoInviteMetadata {
		id: "auto-state-request".to_string(),
		from: identity1.identity().to_owned(),
		peer: Some(network1.local_peer_id().to_bytes()),
		network: Default::default(),
	};
	let metadata_cid = local_co.storage().set_serialized(&metadata).await.unwrap();
	let membership_tags = tags!(
		{KnownTags::CoInviteMetadata}: metadata_cid,
	);

	// peer2: subscribe for Pending → Active transition before pushing membership
	let active_transition = peer2
		.application
		.actions()
		.filter_map(|action| async move {
			match &action {
				Action::CoreAction { co, action, .. } if co.as_str() == CO_ID_LOCAL => {
					let membership_action: MembershipsAction = action.get_payload().ok()?;
					match membership_action {
						MembershipsAction::Join { id, .. } if id.as_str() == "shared" => Some(()),
						_ => None,
					}
				},
				_ => None,
			}
		})
		.take(1)
		.collect::<Vec<_>>();

	// peer2: create Pending membership with empty state + metadata tags
	local_co
		.push(
			&identity2,
			CO_CORE_NAME_MEMBERSHIP,
			&MembershipsAction::JoinPending {
				id: CoId::from("shared"),
				did: identity2.identity().to_owned(),
				options: MembershipOptions::default().with_tags(membership_tags),
			},
		)
		.await
		.unwrap();
	tracing::info!("peer2: created Pending membership with empty state");

	// wait for Pending → Active transition
	timeout(timeout_duration, active_transition)
		.await
		.expect("Pending → Active transition to complete in time");
	tracing::info!("peer2: membership transitioned to Active");

	// peer2: open CO via co_reducer
	let peer2_co = timeout(timeout_duration, peer2.application.co_reducer(CoId::from("shared")))
		.await
		.expect("co_reducer to complete in time")
		.expect("co_reducer ok")
		.expect("co exists after membership");
	tracing::info!("peer2: opened CO via auto state resolution");

	// verify: states match
	assert_eq!(peer2_co.reducer_state().await, shared_co.reducer_state().await);
	tracing::info!("test-complete");
}

/// Test that an unrelated peer can auto-fetch an encrypted CO state via the `pending_resolve`
/// epic when creating a membership with `CoInviteMetadata` tags and Pending state.
///
/// Steps:
/// - Create two peers (peer1 with AllowAll policy, peer2 standard), networking between them
/// - Peer1: create encrypted shared CO
/// - Peer2: create CoInviteMetadata with peer1's PeerId, store in local CO tags
/// - Peer2: create Pending membership with empty state + metadata tags
/// - Wait for Pending → Active transition (epic resolves state + key from network)
/// - Peer2: open CO via co_reducer() and verify state matches
#[tokio::test]
async fn test_unrelated_auto_state_encrypted() {
	let timeout_duration = Duration::from_secs(30);
	let mut instances = Instances::new("test_access_policy_auto_state");

	// peer1: with access policy
	let mut peer1 = instances.create_builder(|b| b.with_access_policy(AllowAll)).await;
	let mut peer2 = instances.create().await;

	// network
	let (network1, _network2) = Instances::networking(&mut peer1, &mut peer2, true, true).await;

	// create identities
	let identity1 = peer1.create_identity().await;
	let identity2 = peer2.create_identity().await;

	// peer1: create encrypted shared CO
	let shared_co = peer1
		.application
		.create_co(identity1.clone(), CreateCo::new("shared", None))
		.await
		.unwrap();
	tracing::info!("peer1: created encrypted shared CO");

	// peer2: create CoInviteMetadata and store in local CO
	let local_co = peer2.application.local_co_reducer().await.unwrap();
	let metadata = CoInviteMetadata {
		id: "auto-state-request".to_string(),
		from: identity1.identity().to_owned(),
		peer: Some(network1.local_peer_id().to_bytes()),
		network: Default::default(),
	};
	let metadata_cid = local_co.storage().set_serialized(&metadata).await.unwrap();
	let membership_tags = tags!(
		{KnownTags::CoInviteMetadata}: metadata_cid,
	);

	// peer2: subscribe for Pending → Active transition before pushing membership
	let active_transition = peer2
		.application
		.actions()
		.filter_map(|action| async move {
			match &action {
				Action::CoreAction { co, action, .. } if co.as_str() == CO_ID_LOCAL => {
					let membership_action: MembershipsAction = action.get_payload().ok()?;
					match membership_action {
						MembershipsAction::Join { id, .. } if id.as_str() == "shared" => Some(()),
						_ => None,
					}
				},
				_ => None,
			}
		})
		.take(1)
		.collect::<Vec<_>>();

	// peer2: create Pending membership with empty state + metadata tags
	local_co
		.push(
			&identity2,
			CO_CORE_NAME_MEMBERSHIP,
			&MembershipsAction::JoinPending {
				id: CoId::from("shared"),
				did: identity2.identity().to_owned(),
				options: MembershipOptions::default().with_tags(membership_tags),
			},
		)
		.await
		.unwrap();
	tracing::info!("peer2: created Pending membership with empty state");

	// wait for Pending → Active transition
	timeout(timeout_duration, active_transition)
		.await
		.expect("Pending → Active transition to complete in time");
	tracing::info!("peer2: membership transitioned to Active");

	// peer2: open CO via co_reducer
	let peer2_co = timeout(timeout_duration, peer2.application.co_reducer(CoId::from("shared")))
		.await
		.expect("co_reducer to complete in time")
		.expect("co_reducer ok")
		.expect("co exists after membership");
	tracing::info!("peer2: opened CO via auto state resolution");

	// verify: states match
	assert_eq!(peer2_co.reducer_state().await, shared_co.reducer_state().await);
	tracing::info!("test-complete");
}
