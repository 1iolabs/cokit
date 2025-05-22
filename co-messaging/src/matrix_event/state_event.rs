use super::multimedia::ImageInfo;
use crate::{EventContent, EventType};
use co_primitives::CoCid;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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
		EventContent::RoomName(val).into()
	}
}

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
		EventContent::RoomTopic(val).into()
	}
}

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
		EventContent::RoomAvatar(val).into()
	}
}

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
		EventContent::PinnedEvents(val).into()
	}
}
