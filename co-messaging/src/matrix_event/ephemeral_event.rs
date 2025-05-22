use crate::{EventContent, EventType};
use co_primitives::CoCid;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/**
 * Event used to indicate which users in the room are currently typing.
 * Gets sent to all active users. For direct messages this information will only be shared with the other
 * participant. Information should be updated regularly and have a timout after which no users should count as
 * typing when no new event was sent.
 */
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
pub struct TypingContent {
	/// List of users currently typing in the room
	pub user_ids: Vec<String>,
}

impl EventType for TypingContent {
	fn generate_event_type(&self) -> String {
		"m.typing".into()
	}
}

impl From<TypingContent> for EventContent {
	fn from(val: TypingContent) -> Self {
		EventContent::Typing(val).into()
	}
}

impl TypingContent {
	pub fn new(user_ids: Vec<String>) -> Self {
		Self { user_ids }
	}
}

/// Basic enum for possible presence states of a specific user
///
/// Online: Default state when user is connected to an event stream
///
/// Offline: The user is not connected to an event stream or is actively suppressing this information
///
/// DnD: As 'Online' but the user doesn't want to be disturbed
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
pub enum PresenceType {
	#[serde(rename = "online")]
	Online,
	#[serde(rename = "offline")]
	Offline,
	#[serde(rename = "dnd")]
	Dnd,
}

/**
 * Event content that is used to inform other users of the presence status.
 * In contrast to typing events, the sender is important here and always corresponds to the user the information is
 * about.
 */
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
pub struct PresenceContent {
	pub presence: PresenceType,
	/// Timestampt in milliseconds when the user last performed an action
	pub last_active: u32,
	/// Whether the user is currently active
	pub currently_active: bool,
	/// Avatar the user is currently using
	pub avatar: CoCid,
	/// Display name of the user
	pub display_name: String,
	/// An optional arbitrary description to accompany the presence
	pub status_msg: String,
}

impl EventType for PresenceContent {
	fn generate_event_type(&self) -> String {
		"m.presence".into()
	}
}

impl From<PresenceContent> for EventContent {
	fn from(val: PresenceContent) -> Self {
		EventContent::Presence(val).into()
	}
}

impl PresenceContent {
	pub fn new(
		presence: PresenceType,
		last_active: u32,
		currently_active: bool,
		avatar: impl Into<CoCid>,
		display_name: impl Into<String>,
		status_msg: impl Into<String>,
	) -> Self {
		Self {
			presence,
			last_active,
			currently_active,
			avatar: avatar.into(),
			display_name: display_name.into(),
			status_msg: status_msg.into(),
		}
	}
}
