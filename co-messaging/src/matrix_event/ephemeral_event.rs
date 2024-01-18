use libipld::Cid;
use serde::{Deserialize, Serialize};

use crate::{EventContent, EventType};

/**
 * Ephemeral events are once-off events that do not need to be saved.
 */
#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(tag = "type", content = "content")]
pub enum EphemeralType {
    #[serde(rename = "m.typing")]
    Typing(TypingContent),
    #[serde(rename = "m.presence")]
    Presence(PresenceContent),
}

impl EventType for EphemeralType {
    fn generate_event_type(&self) -> String {
        match self {
            EphemeralType::Typing(c) => c.generate_event_type(),
            EphemeralType::Presence(c) => c.generate_event_type(),
        }
    }
}

impl Into<EventContent> for EphemeralType {
    fn into(self) -> EventContent {
        EventContent::Ephemeral(self)
    }
}

/**
 * Event used to indicate which users in the room are currently typing.
 * Gets sent to all active users. For direct messages this information will only be shared with the other participant.
 * Information should be updated regularly and have a timout after which no users should count as typing when no new event was sent.
 */
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct TypingContent {
    pub user_ids: Vec<String>, // List of users currently typing in the room
}

impl EventType for TypingContent {
    fn generate_event_type(&self) -> String {
        "m.typing".into()
    }
}

impl Into<EventContent> for TypingContent {
    fn into(self) -> EventContent {
        EphemeralType::Typing(self).into()
    }
}

impl TypingContent {
    pub fn new(user_ids: Vec<String>) -> Self {
        Self { user_ids }
    }
}

/**
 * Basic enum for possible presence states of a specific user
 */
#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(tag = "presence")]
pub enum PresenceType {
    #[serde(rename = "online")]
    Online, // Default state when user is connected to an event stream
    #[serde(rename = "offline")]
    Offline, // The user is not connected to an event stream or is actively suppressing this information
    #[serde(rename = "dnd")]
    Dnd, // As 'Online' but the user doesn't want to be disturbed
}

/**
 * Event content that is used to inform other users of the presence status.
 * In contrast to typing events, the sender is important here and always corresponds to the user the information is about.
 */
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct PresenceContent {
    #[serde(flatten)]
    pub presence: PresenceType,
    pub last_active: i64, // Timestampt in milliseconds when the user last performed an action
    pub currently_active: bool, // Whether the user is currently active
    pub avatar: Cid,      // Avatar the user is currently using
    pub display_name: String, // Display name of the user
    pub status_msg: String, // An optional arbitrary description to accompany the presence
}

impl EventType for PresenceContent {
    fn generate_event_type(&self) -> String {
        "m.presence".into()
    }
}

impl Into<EventContent> for PresenceContent {
    fn into(self) -> EventContent {
        EphemeralType::Presence(self).into()
    }
}

impl PresenceContent {
    pub fn new(
        presence: PresenceType,
        last_active: i64,
        currently_active: bool,
        avatar: Cid,
        display_name: impl Into<String>,
        status_msg: impl Into<String>,
    ) -> Self {
        Self {
            presence,
            last_active,
            currently_active,
            avatar,
            display_name: display_name.into(),
            status_msg: status_msg.into(),
        }
    }
}
