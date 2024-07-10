use co_primitives::serde_string_enum;
use co_primitives::Tag;
use co_primitives::Tags;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum KnownTags {
	#[serde(rename = "co-invite")]
	CoInvite,
}
impl KnownTags {}
serde_string_enum!(KnownTags);

#[derive(Debug, Default, Serialize, Deserialize)]
pub enum CoInvite {
	/// Manual: Add "pending" membership.
	#[serde(rename = "manual")]
	#[default]
	Manual,

	/// Disable: Reject all Invite requests.
	#[serde(rename = "disable")]
	Disable,

	// All: Auto accept all Invite requests.
	#[serde(rename = "all")]
	All,

	/// DID Verification: Only accept join when DID can be verified for certain properties.
	#[serde(rename = "did")]
	Did,
}
serde_string_enum!(CoInvite);
impl CoInvite {
	pub fn from_tags(tags: &Tags) -> Option<Self> {
		tags.string(&KnownTags::CoInvite.to_string())
			.and_then(|value| Self::try_from(value).ok())
	}

	pub fn tag(&self) -> Tag {
		(KnownTags::CoInvite.to_string(), self.to_string().into())
	}
}
impl Into<Tag> for &CoInvite {
	fn into(self) -> Tag {
		self.tag()
	}
}
impl Into<Tags> for &CoInvite {
	fn into(self) -> Tags {
		[self.tag()].into_iter().collect()
	}
}
