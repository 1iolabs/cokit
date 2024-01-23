use super::multimedia::ImageInfo;
use crate::{EventContent, EventType};
use libipld::Cid;
use serde::{Deserialize, Serialize};

/**
 * All events that in some way alter the state of a room
 */
#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(tag = "type", content = "content")]
pub enum StateType {
	#[serde(rename = "m.room.name")]
	RoomName(RoomNameContent),
	#[serde(rename = "m.room.topic")]
	RoomTopic(RoomTopicContent),
	#[serde(rename = "m.room.avatar")]
	RoomAvatar(RoomAvatarContent),
	#[serde(rename = "m.room.pinned_events")]
	PinnedEvents(PinnedEventsContent),
}

impl Into<EventContent> for StateType {
	fn into(self) -> EventContent {
		EventContent::State(self)
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

#[derive(Serialize, Deserialize, Debug, PartialEq)]
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

impl Into<EventContent> for RoomNameContent {
	fn into(self) -> EventContent {
		StateType::RoomName(self).into()
	}
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
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

impl Into<EventContent> for RoomTopicContent {
	fn into(self) -> EventContent {
		StateType::RoomTopic(self).into()
	}
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct RoomAvatarContent {
	pub file: Cid,
	pub info: ImageInfo,
}

impl RoomAvatarContent {
	pub fn new(file: Cid, info: ImageInfo) -> Self {
		Self { file, info }
	}
}

impl EventType for RoomAvatarContent {
	fn generate_event_type(&self) -> String {
		"m.room.avatar".into()
	}
}

impl Into<EventContent> for RoomAvatarContent {
	fn into(self) -> EventContent {
		StateType::RoomAvatar(self).into()
	}
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
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

impl Into<EventContent> for PinnedEventsContent {
	fn into(self) -> EventContent {
		StateType::PinnedEvents(self).into()
	}
}
