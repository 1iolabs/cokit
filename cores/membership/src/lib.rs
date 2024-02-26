use co_api::{reduce, CoId, Context, Did, Reducer, ReducerAction, Tags};
use libipld::Cid;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Membership COre.
/// Stores membership information of an CO (counterpart to co participants).
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct Memberships {
	pub memberships: Vec<Membership>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Membership {
	/// The CO Unique Identifier.
	pub id: CoId,

	/// The did used for the membership.
	pub did: Did,

	/// The CO root state (usually co-core-co).
	/// Note: This is not an Option so we can not be member of an emtpy CO (which has no id anyway).
	pub state: Cid,

	/// The CO heads.
	pub heads: BTreeSet<Cid>,

	/// The encryption mapping if the CO is encrypted.
	pub encryption_mapping: Option<Cid>,

	/// Some encryption key URI if the CO is encrypted.
	pub key: Option<String>,

	/// Membership state.
	pub membership_state: MembershipState,

	/// Membership tags.
	pub tags: Tags,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub enum MembershipState {
	Invited,
	Active,
	Closed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MembershipsAction {
	Join(Membership),
	Update { id: CoId, state: Cid, heads: BTreeSet<Cid>, encryption_mapping: Option<Cid> },
	ChangeMembershipState { id: CoId, did: Did, membership_state: MembershipState },
	ChangeKey { id: CoId, did: Did, key: String },
	TagsInsert { id: CoId, did: Did, tags: Tags },
	TagsRemove { id: CoId, did: Did, tags: Tags },
	Remove { id: CoId, did: Option<Did> },
}

impl Reducer for Memberships {
	type Action = MembershipsAction;

	fn reduce(self, event: &ReducerAction<Self::Action>, _: &mut dyn Context) -> Self {
		let mut result = self;
		match &event.payload {
			MembershipsAction::Update { id, state, heads, encryption_mapping } => {
				// if find(&mut result, &membership.id, &membership.did).is_none() {
				// 	membership.state = state.clone();
				// 	membership.heads = heads.clone();
				// 	membership.encryption_mapping = encryption_mapping.clone();
				// }
				for membership in result.memberships.iter_mut() {
					if &membership.id == id {
						membership.state = state.clone();
						membership.heads = heads.clone();
						membership.encryption_mapping = encryption_mapping.clone();
					}
				}
			},
			MembershipsAction::Join(membership) =>
				if find(&mut result, &membership.id, &membership.did).is_none() {
					result.memberships.push(membership.clone());
				},
			MembershipsAction::ChangeMembershipState { id, did, membership_state } =>
				if let Some(membership) = find(&mut result, id, did) {
					membership.membership_state = *membership_state;
				},
			MembershipsAction::ChangeKey { id, did, key } =>
				if let Some(membership) = find(&mut result, id, did) {
					membership.key = Some(key.to_owned());
				},
			MembershipsAction::TagsInsert { id, did, tags } =>
				if let Some(membership) = find(&mut result, id, did) {
					membership.tags.append(&mut tags.clone());
				},
			MembershipsAction::TagsRemove { id, did, tags } =>
				if let Some(membership) = find(&mut result, id, did) {
					membership.tags.clear(Some(tags));
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

#[no_mangle]
pub extern "C" fn state() {
	reduce::<Memberships>()
}
