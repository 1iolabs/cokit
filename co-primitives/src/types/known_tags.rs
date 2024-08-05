use super::tags::TagValue;
use crate::{serde_string_enum, Tag, Tags};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize)]
pub enum KnownTags {
	/// [`CoInvite`]
	#[serde(rename = "co-invite")]
	CoInvite,

	/// [`CoJoin`]
	#[serde(rename = "co-join")]
	CoJoin,

	/// [`crate::CoInviteMetadata`]
	/// [`crate::Link`]
	#[serde(rename = "co-invite-metadata")]
	CoInviteMetadata,

	/// [`CoNetwork`]
	#[serde(rename = "co-network")]
	CoNetwork,

	/// [`CoTimeout`]
	#[serde(rename = "co-timeout")]
	CoTimeout,
}
impl KnownTags {}
serde_string_enum!(KnownTags);

pub trait KnownTag {
	fn key() -> KnownTags;
	fn value(&self) -> TagValue;
	fn from_value(value: &TagValue) -> Option<Self>
	where
		Self: Sized;

	fn from_tags<'a>(tags: &'a Tags) -> Option<Self>
	where
		Self: Sized,
	{
		tags.value(&Self::key().to_string()).and_then(Self::from_value)
	}

	fn tag(&self) -> Tag {
		(Self::key().to_string(), self.value())
	}
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub enum CoInvite {
	/// Manual: Add "pending" membership.
	#[serde(rename = "manual")]
	#[default]
	Manual,

	/// Disable: Reject all Invite requests.
	#[serde(rename = "disable")]
	Disable,

	/// Accept: Auto accept all Invite requests.
	#[serde(rename = "accept")]
	Accept,

	/// DID Verification: Only accept invite when DID can be verified for certain properties.
	#[serde(rename = "did")]
	Did,
}
serde_string_enum!(CoInvite);
impl KnownTag for CoInvite {
	fn key() -> KnownTags {
		KnownTags::CoInvite
	}

	fn value(&self) -> TagValue {
		self.to_string().into()
	}

	fn from_value(value: &TagValue) -> Option<Self>
	where
		Self: Sized,
	{
		value.string().and_then(|str| Self::try_from(str).ok())
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

#[derive(Debug, Default, Serialize, Deserialize)]
pub enum CoJoin {
	/// Invite: Only accept joins when participant has been invited.
	#[serde(rename = "invite")]
	#[default]
	Invite,

	/// Accept: Auto Accept all Join requests.
	#[serde(rename = "accept")]
	Accept,

	/// DID Verification: Only accept join when DID can be verified for certain properties.
	#[serde(rename = "did")]
	Did,

	/// Manual: Add "pending" participant.
	#[serde(rename = "manual")]
	Manual,
}
serde_string_enum!(CoJoin);
impl KnownTag for CoJoin {
	fn key() -> KnownTags {
		KnownTags::CoJoin
	}

	fn value(&self) -> TagValue {
		self.to_string().into()
	}

	fn from_value(value: &TagValue) -> Option<Self>
	where
		Self: Sized,
	{
		value.string().and_then(|str| Self::try_from(str).ok())
	}
}

/// CO Participant network feature(s) tag.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
pub enum CoNetwork {
	/// Use all available networking capabilities.
	#[serde(rename = "all")]
	#[default]
	All,

	/// Use this
	#[serde(rename = "invite")]
	Invite,
}
impl CoNetwork {
	pub fn has_feature(&self, feature: CoNetwork) -> bool {
		match self {
			CoNetwork::All => true,
			other => other == &feature,
		}
	}
}
impl KnownTag for CoNetwork {
	fn key() -> KnownTags {
		KnownTags::CoNetwork
	}

	fn value(&self) -> TagValue {
		self.to_string().into()
	}

	fn from_value(value: &TagValue) -> Option<Self>
	where
		Self: Sized,
	{
		value.string().and_then(|str| Self::try_from(str).ok())
	}
}
serde_string_enum!(CoNetwork);

/// CO timeout setting.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum CoTimeout {
	/// Use default timeout settings. Usually from the parent CO or global timeout.
	#[default]
	Default,

	/// Duration.
	Duration(Duration),

	/// Scoped setting.
	Scope(String, Duration),
}
impl CoTimeout {
	pub fn default_duration() -> Duration {
		Duration::from_secs(30)
	}

	pub fn get_timeout<'a>(
		tags: impl IntoIterator<Item = &'a Tag>,
		scope: Option<&str>,
		default: Option<Duration>,
	) -> Duration {
		let tag = KnownTags::CoTimeout.to_string();
		let mut result = default.unwrap_or(Self::default_duration());
		for timeout in tags
			.into_iter()
			.filter(|(key, _)| key == &tag)
			.filter_map(|(_, value)| libipld::serde::from_ipld::<CoTimeout>(value.clone().into()).ok())
		{
			match timeout {
				CoTimeout::Default => {},
				CoTimeout::Duration(timeout) => {
					result = timeout;
				},
				CoTimeout::Scope(v, timeout) => {
					if Some(v.as_str()) == scope {
						result = timeout;
						break;
					}
				},
			}
		}
		result
	}
}
