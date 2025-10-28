use super::instance::{Instance, Instances};
use co_core_co::CoAction;
use co_core_membership::{MembershipState, MembershipsAction};
use co_sdk::{
	update_co, Action, CoId, CoReducer, CoReducerFactory, CreateCo, Did, DidKeyIdentity, Identity, PrivateIdentity,
	PrivateIdentityBox, CO_CORE_NAME_CO, CO_CORE_NAME_MEMBERSHIP, CO_ID_LOCAL,
};
use futures::{join, Stream, StreamExt};
use std::{collections::BTreeSet, time::Duration};
use tokio::time::timeout;
use tracing::{info_span, Instrument};

pub struct SharedCo {
	pub peers: Vec<(Instance, DidKeyIdentity)>,
}
impl SharedCo {
	/// Get reducer and identity for a peer.
	pub async fn reducer(&self, peer: usize, id: &str) -> (CoReducer, PrivateIdentityBox) {
		let (instance, identity) = self.peers.get(peer).unwrap();
		let context = instance.application.co();
		let co_id = CoId::from(id);
		(context.try_co_reducer(&co_id).await.unwrap(), PrivateIdentity::boxed(identity.clone()))
	}

	/// Create two peers with an connected shared co.
	pub async fn create(instances: &mut Instances, id: &str) -> Self {
		let timeout_duration = Duration::from_secs(10);

		let mut peer1 = instances.create().await;
		let mut peer2 = instances.create().await;

		// network
		let (network1, _network2) = Instances::networking(&mut peer1, &mut peer2, true, true).await;

		// create identity
		let identity1 = peer1.create_identity().await;
		let identity2 = peer2.create_identity().await;

		// peer1: create shared co
		let shared_co = async {
			peer1
				.application
				.create_co(identity1.clone(), CreateCo::new(id, None))
				.await
				.unwrap()
		}
		.instrument(info_span!("peer1: created shared co", application = peer1.application.settings().identifier))
		.await;

		// peer1: invite peer2
		async {
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
		}
		.instrument(info_span!(
			"peer1: added other peer identity",
			application = peer1.application.settings().identifier
		))
		.await;

		// peer2: membership-invite
		let peer2_membership_invite = wait_membership_state(peer2.application.actions(), [MembershipState::Invite]);
		let peer2_membership_invite = timeout(timeout_duration, peer2_membership_invite)
			.await
			.expect("peer2 to recv invite in time")
			.expect("not empty");
		assert_eq!(peer2_membership_invite, (MembershipState::Invite, CoId::from(id), identity2.identity().to_owned()));

		// peer2: join
		//  set membership to join and wait for membership set to active when join is complete
		async {
			let local_co = peer2.application.local_co_reducer().await.unwrap();
			let payload = MembershipsAction::ChangeMembershipState {
				id: id.into(),
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
				(MembershipState::Active, CoId::from(id), identity2.identity().to_owned())
			);
		}
		.instrument(info_span!("peer2: join", application = peer2.application.settings().identifier))
		.await;

		// peer2: force sync (needed because of the paricipant state update)
		let peer2_shared_co = peer2.application.co_reducer(CoId::from(id)).await.unwrap().unwrap();
		async {
			update_co(
				peer2.application.handle(),
				&peer2_shared_co,
				&identity2,
				network1.local_peer_id(),
				Duration::from_secs(10),
			)
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

		// result
		Self { peers: vec![(peer1, identity1), (peer2, identity2)] }
	}
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
