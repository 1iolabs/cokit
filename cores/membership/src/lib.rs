use cid::Cid;
use co_api::{CoId, Context, Did, Reducer, ReducerAction, Tags, WeakCid};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::collections::BTreeSet;

/// Membership COre.
/// Stores membership information of an CO (counterpart to co participants).
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct Memberships {
	pub memberships: Vec<Membership>,
}

/// Membership entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CoState {
	/// The CO root state (usually co-core-co).
	/// Note: This is not an Option so we can not be member of an emtpy CO (which has no id anyway).
	pub state: WeakCid,

	/// The CO heads.
	pub heads: BTreeSet<WeakCid>,

	// TODO mark as external as this field shouldn't be further resolved when pinning
	// TODO https://gitlab.1io.com/1io/co-sdk/-/issues/47
	/// The encryption mapping if the CO is encrypted.
	pub encryption_mapping: Option<Cid>,
}

#[derive(Debug, Clone, Copy, Serialize_repr, Deserialize_repr, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
#[repr(u8)]
pub enum MembershipState {
	/// Active membership.
	Active = 0,

	/// Inactive membership.
	Inactive = 1,

	/// Pending invite by some participant of the CO.
	///
	/// Use Cases:
	/// - This is waiting for our acception/rejection.
	/// - Accept invite by change membership state to [`MembershipState::Join`].
	/// - Reject invite by removing the membership using [`MembershipsAction::Remove`].
	///
	/// Related membership Tags:
	///  `co-invite: CoInviteMetadata`
	Invite = 2,

	/// Pending join by us.
	///
	/// Use Cases:
	/// - This is a pending join triggered by an invite waiting for completion.
	/// - This is waiting for CO participant acception/rejection (remote).
	///
	/// Related membership Tags:
	///  `co-invite: CoInviteMetadata`
	///  `join-date: Date`
	Join = 3,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MembershipsAction {
	Join(Membership),
	Update {
		id: CoId,
		state: Cid,
		heads: BTreeSet<Cid>,
		encryption_mapping: Option<Cid>,
		/// Remove all [`CoState`] which heads are fully covered.
		remove: BTreeSet<Cid>,
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

impl Reducer for Memberships {
	type Action = MembershipsAction;

	fn reduce(self, event: &ReducerAction<Self::Action>, _: &mut dyn Context) -> Self {
		let mut result = self;
		match &event.payload {
			MembershipsAction::Update { id, state, heads, encryption_mapping, remove } => {
				// if find(&mut result, &membership.id, &membership.did).is_none() {
				// 	membership.state = state.clone();
				// 	membership.heads = heads.clone();
				// 	membership.encryption_mapping = encryption_mapping.clone();
				// }
				let remove = remove.into_iter().map(WeakCid::from).collect::<BTreeSet<WeakCid>>();
				for membership in result.memberships.iter_mut() {
					if &membership.id == id {
						membership.state.retain(|item| !remove.is_superset(&item.heads));
						membership.state.insert(CoState {
							state: state.into(),
							heads: heads.iter().map(WeakCid::from).collect(),
							encryption_mapping: *encryption_mapping,
						});
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
		result
	}
}

fn find<'a>(memberships: &'a mut Memberships, co: &CoId, did: &str) -> Option<&'a mut Membership> {
	memberships
		.memberships
		.iter_mut()
		.find(|item| &item.id == co && &item.did == did)
}

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
#[no_mangle]
pub extern "C" fn state() {
	co_api::reduce::<Memberships>()
}
