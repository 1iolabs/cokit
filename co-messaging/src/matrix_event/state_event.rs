use super::multimedia::ImageInfo;
use crate::{EventContent, EventType};
use cid::Cid;
use co_macros::co_data;
use co_primitives::CoCid;
use schemars::JsonSchema;

#[co_data]
#[derive(JsonSchema)]
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

#[co_data]
#[derive(JsonSchema)]
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

#[co_data]
#[derive(JsonSchema)]
pub struct RoomAvatarContent {
	#[schemars(with = "Option<CoCid>")]
	pub file: Option<Cid>,
	pub info: ImageInfo,
}

impl RoomAvatarContent {
	pub fn new(file: Option<Cid>, info: ImageInfo) -> Self {
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

#[co_data]
#[derive(JsonSchema)]
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
