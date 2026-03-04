// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use cid::Cid;
use co_api::{
	async_api::Reducer, co, BlockStorageExt, CoId, CoReference, CoreBlockStorage, Did, Link, OptionLink, ReducerAction,
	StorageError, Tags, WeakCid,
};
use futures::TryStreamExt;
use std::collections::BTreeSet;

/// Membership COre.
/// Stores membership information of an CO (counterpart to co participants).
#[co(state)]
pub struct Memberships {
	pub memberships: Vec<Membership>,
}

/// Membership entry.
#[co]
pub struct Membership {
	/// The CO Unique Identifier.
	pub id: CoId,

	/// The did used for the membership.
	pub did: Did,

	/// CO States. This can be multiple states if we have heads that are not joined yet.
	pub state: BTreeSet<CoState>,

	/// Some encryption key URI if the CO is encrypted.
	pub key: Option<String>,

	/// Membership state.
	pub membership_state: MembershipState,

	/// Membership tags.
	pub tags: Tags,
}

/// A CO State entry.
/// Contains heads the computed state for the heads and an option encryption mapping.
#[co]
pub struct CoState {
	/// The CO root state (usually co-core-co) and heads.
	/// Note: This is not an Option so we can not be member of an emtpy CO (which has no id anyway).
	/// Note: We want to use `CoReference::Weak` instead of `WeakCid` here because we need to have mappings generated
	/// for it.
	pub state: Link<CoReference<(Cid, BTreeSet<Cid>)>>,

	// TODO mark as external as this field shouldn't be further resolved when pinning
	// TODO https://gitlab.1io.com/1io/co-sdk/-/issues/47
	/// The encryption mapping if the CO is encrypted.
	#[serde(skip_serializing_if = "Option::is_none", default)]
	pub encryption_mapping: Option<Cid>,
}

/// Membership state.
///
/// # Guarantees
/// - Sortable from active (low) to inactive (high).
#[co(repr)]
#[non_exhaustive]
#[repr(u8)]
pub enum MembershipState {
	/// Active membership.
	Active = 10,

	/// Pending join by us.
	///
	/// Use Cases:
	/// - This is a pending join triggered by an invite waiting for completion.
	/// - This is waiting for CO participant acceptation/rejection (remote).
	///
	/// Related membership Tags:
	///  `co-invite: CoInviteMetadata`
	///  `join-date: Date`
	Join = 20,

	/// Pending invite by some participant of the CO.
	///
	/// Use Cases:
	/// - This is waiting for our acceptation/rejection.
	/// - Accept invite by change membership state to [`MembershipState::Join`].
	/// - Reject invite by removing the membership using [`MembershipsAction::Remove`].
	///
	/// Related membership Tags:
	///  `co-invite: CoInviteMetadata`
	Invite = 30,

	/// Inactive membership.
	Inactive = 40,
}

#[co]
pub enum MembershipsAction {
	/// Join a Co. The membership state indicates if it was an invite from someone.
	Join(Membership),
	Update {
		id: CoId,
		state: CoState,
		/// Remove all [`CoState`] which heads are fully covered.
		remove: BTreeSet<WeakCid>,
	},
	ChangeMembershipState {
		id: CoId,
		did: Did,
		membership_state: MembershipState,
	},
	/// Change the active encryption key reference which is used the read the current heads/state.
	ChangeKey {
		id: CoId,
		did: Did,
		key: String,
	},
	TagsInsert {
		id: CoId,
		did: Did,
		tags: Tags,
	},
	TagsRemove {
		id: CoId,
		did: Did,
		tags: Tags,
	},
	Remove {
		id: CoId,
		did: Option<Did>,
	},
}

impl Reducer<MembershipsAction> for Memberships {
	async fn reduce(
		state_ref: OptionLink<Self>,
		action_ref: Link<ReducerAction<MembershipsAction>>,
		storage: &CoreBlockStorage,
	) -> Result<Link<Self>, anyhow::Error> {
		let action = storage.get_value(&action_ref).await?;
		let mut result = storage.get_value_or_default(&state_ref).await?;
		match &action.payload {
			MembershipsAction::Update { id, state, remove } => {
				// if find(&mut result, &membership.id, &membership.did).is_none() {
				// 	membership.state = state.clone();
				// 	membership.heads = heads.clone();
				// 	membership.encryption_mapping = encryption_mapping.clone();
				// }
				let remove = remove.iter().map(WeakCid::cid).collect::<BTreeSet<Cid>>();
				for membership in result.memberships.iter_mut() {
					if &membership.id == id {
						membership.state = filter_state(storage.clone(), &membership.state, &remove).await?;

						// add
						membership.state.insert(state.clone());
					}
				}
			},
			MembershipsAction::Join(membership) => {
				if find(&mut result, &membership.id, &membership.did).is_none() {
					result.memberships.push(membership.clone());
				}
			},
			MembershipsAction::ChangeMembershipState { id, did, membership_state } => {
				if let Some(membership) = find(&mut result, id, did) {
					membership.membership_state = *membership_state;
				}
			},
			MembershipsAction::ChangeKey { id, did, key } => {
				if let Some(membership) = find(&mut result, id, did) {
					membership.key = Some(key.to_owned());
				}
			},
			MembershipsAction::TagsInsert { id, did, tags } => {
				if let Some(membership) = find(&mut result, id, did) {
					membership.tags.append(&mut tags.clone());
				}
			},
			MembershipsAction::TagsRemove { id, did, tags } => {
				if let Some(membership) = find(&mut result, id, did) {
					membership.tags.clear(Some(tags));
				}
			},
			MembershipsAction::Remove { id, did } => {
				if let Some((index, _)) = result.memberships.iter().enumerate().find(|(_, item)| {
					&item.id == id && (did.is_none() || did.as_ref().is_some_and(|did| &item.did == did))
				}) {
					result.memberships.remove(index);
				}
			},
		}
		Ok(storage.set_value(&result).await?)
	}
}

async fn filter_state(
	storage: CoreBlockStorage,
	state: &BTreeSet<CoState>,
	remove: &BTreeSet<Cid>,
) -> Result<BTreeSet<CoState>, StorageError> {
	async_stream::try_stream! {
		for item in state {
			let (_state, heads) = storage.get_value(&item.state).await?.into_value();
			if !remove.is_superset(&heads) {
				yield item.clone();
			}
		}
	}
	.try_collect()
	.await
}

fn find<'a>(memberships: &'a mut Memberships, co: &CoId, did: &str) -> Option<&'a mut Membership> {
	memberships
		.memberships
		.iter_mut()
		.find(|item| &item.id == co && item.did == did)
}
