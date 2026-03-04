// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use async_trait::async_trait;
use co_core_co::CoAction;
use co_primitives::CoId;
use co_sdk::{Action, CoAccessPolicy, Identity, KeyRequestAction, CO_CORE_NAME_CO, CO_ID_LOCAL};
use futures::StreamExt;
use helper::{instance::Instances, shared_co::SharedCo};
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
async fn test_access_policy_allows_removed_participant() {
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
