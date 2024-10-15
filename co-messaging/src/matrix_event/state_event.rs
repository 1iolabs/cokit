use super::multimedia::ImageInfo;
use crate::{EventContent, EventType};
use co_primitives::CoCid;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use typeshare::typeshare;

/**
 * All events that in some way alter the state of a room
 */
#[typeshare]
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
#[serde(tag = "type", content = "content")]
pub enum StateType {
	#[serde(rename = "room_name")]
	RoomName(RoomNameContent),
	#[serde(rename = "room_topic")]
	RoomTopic(RoomTopicContent),
	#[serde(rename = "room_avatar")]
	RoomAvatar(RoomAvatarContent),
	#[serde(rename = "room_pinned_events")]
	PinnedEvents(PinnedEventsContent),
}

impl From<StateType> for EventContent {
	fn from(val: StateType) -> Self {
		EventContent::State(val)
	}
}

impl EventType for StateType {
	fn generate_event_type(&self) -> String {
		match &self {
			StateType::RoomName(content) => content.generate_event_type(),
			StateType::RoomTopic(content) => content.generate_event_type(),
			StateType::RoomAvatar(content) => content.generate_event_type(),
			StateType::PinnedEvents(content) => content.generate_event_type(),
		}
	}
}

#[typeshare]
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
pub struct RoomNameContent {
	pub name: String,
}

impl RoomNameContent {
	pub fn new(name: impl Into<String>) -> Self {
		Self { name: name.into() }
	}
}

impl EventType for RoomNameContent {
	fn generate_event_type(&self) -> String {
		"m.room.name".into()
	}
}

impl From<RoomNameContent> for EventContent {
	fn from(val: RoomNameContent) -> Self {
		StateType::RoomName(val).into()
	}
}

#[typeshare]
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
pub struct RoomTopicContent {
	pub topic: String,
}

impl RoomTopicContent {
	pub fn new(topic: impl Into<String>) -> Self {
		Self { topic: topic.into() }
	}
}

impl EventType for RoomTopicContent {
	fn generate_event_type(&self) -> String {
		"m.room.topic".into()
	}
}

impl From<RoomTopicContent> for EventContent {
	fn from(val: RoomTopicContent) -> Self {
		StateType::RoomTopic(val).into()
	}
}

#[typeshare]
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
pub struct RoomAvatarContent {
	pub file: Option<CoCid>,
	pub info: ImageInfo,
}

impl RoomAvatarContent {
	pub fn new(file: Option<impl Into<CoCid>>, info: ImageInfo) -> Self {
		Self { file: file.map(Into::into), info }
	}
}

impl EventType for RoomAvatarContent {
	fn generate_event_type(&self) -> String {
		"m.room.avatar".into()
	}
}

impl From<RoomAvatarContent> for EventContent {
	fn from(val: RoomAvatarContent) -> Self {
		StateType::RoomAvatar(val).into()
	}
}

#[typeshare]
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
pub struct PinnedEventsContent {
	pub pinned: Vec<String>,
}

impl PinnedEventsContent {
	pub fn new(pinned: Vec<String>) -> Self {
		Self { pinned }
	}
}

impl EventType for PinnedEventsContent {
	fn generate_event_type(&self) -> String {
		"m.room.pinned_events".into()
	}
}

impl From<PinnedEventsContent> for EventContent {
	fn from(val: PinnedEventsContent) -> Self {
		StateType::PinnedEvents(val).into()
	}
}
