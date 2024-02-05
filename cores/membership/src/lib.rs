use co_api::{reduce, Context, Did, Reducer, ReducerAction, Tags, TagsPattern};
use libipld::Cid;
use serde::{Deserialize, Serialize};

/// Membership COre.
/// Stores membership information of an CO (counterpart to co participants).
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct Memberships {
	pub memberships: Vec<Membership>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Membership {
	/// The CO state (usually co-core-co).
	pub co: Cid,

	/// The did used for the membership.
	pub did: Did,

	/// The encryption key URI.
	pub key: String,

	/// Membership state.
	pub state: MembershipState,

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
	ChangeState { co: Cid, did: Did, state: MembershipState },
	ChangeKey { co: Cid, did: Did, key: String },
	TagsInsert { co: Cid, did: Did, tags: Tags },
	TagsRemove { co: Cid, did: Did, tags: Tags },
	Remove { co: Cid, did: Did },
}

impl Reducer for Memberships {
	type Action = MembershipsAction;

	fn reduce(self, event: &ReducerAction<Self::Action>, _: &mut dyn Context) -> Self {
		let mut result = self;
		match &event.payload {
			MembershipsAction::Join(membership) =>
				if find(&mut result, &membership.co, &membership.did).is_none() {
					result.memberships.push(membership.clone());
				},
			MembershipsAction::ChangeState { co, did, state } =>
				if let Some(membership) = find(&mut result, co, did) {
					membership.state = *state;
				},
			MembershipsAction::ChangeKey { co, did, key } =>
				if let Some(membership) = find(&mut result, co, did) {
					membership.key = key.to_owned();
				},
			MembershipsAction::TagsInsert { co, did, tags } =>
				if let Some(membership) = find(&mut result, co, did) {
					membership.tags.append(&mut tags.clone());
				},
			MembershipsAction::TagsRemove { co, did, tags } =>
				if let Some(membership) = find(&mut result, co, did) {
					membership.tags.clear(Some(tags));
				},
			MembershipsAction::Remove { co, did } => {
				if let Some((index, _)) = result
					.memberships
					.iter()
					.enumerate()
					.find(|(_, item)| &item.co == co && &item.did == did)
				{
					result.memberships.remove(index);
				}
			},
		}
		result
	}
}

fn find<'a>(memberships: &'a mut Memberships, co: &Cid, did: &str) -> Option<&'a mut Membership> {
	memberships
		.memberships
		.iter_mut()
		.find(|item| &item.co == co && &item.did == did)
}

#[no_mangle]
pub extern "C" fn state() {
	reduce::<Memberships>()
}
