// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use super::multimedia::ImageInfo;
use crate::{EventContent, EventType};
use cid::Cid;
use co_macros::co;
use co_primitives::CoCid;
use schemars::JsonSchema;

#[co]
#[derive(JsonSchema)]
#[non_exhaustive]
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
		EventContent::RoomName(val)
	}
}

#[co]
#[derive(JsonSchema)]
#[non_exhaustive]
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
		EventContent::RoomTopic(val)
	}
}

/// Room/group avatar: either a content-addressed image or an emoji.
/// The `Emoji` arm is a cokit extension — Matrix `m.room.avatar` is image-only.
#[co]
#[derive(JsonSchema)]
#[non_exhaustive]
pub enum Avatar {
	Image {
		#[schemars(with = "CoCid")]
		cid: Cid,
		info: ImageInfo,
	},
	Emoji(String),
}

#[co]
#[derive(JsonSchema)]
#[non_exhaustive]
pub struct RoomAvatarContent {
	pub avatar: Option<Avatar>,
}

impl RoomAvatarContent {
	pub fn new(avatar: Avatar) -> Self {
		Self { avatar: Some(avatar) }
	}

	pub fn remove() -> Self {
		Self { avatar: None }
	}
}

impl EventType for RoomAvatarContent {
	fn generate_event_type(&self) -> String {
		"m.room.avatar".into()
	}
}

impl From<RoomAvatarContent> for EventContent {
	fn from(val: RoomAvatarContent) -> Self {
		EventContent::RoomAvatar(val)
	}
}

/// A single pin/unpin toggle for one event. The room core LWW-merges these per
/// `event_id` (by timestamp) into `Room.pinned_messages` — concurrent pins on
/// different events do not clobber each other. (Diverges from the Matrix
/// whole-list `m.room.pinned_events` state event, which the room core no longer
/// applies wholesale.)
#[co]
#[derive(JsonSchema, Default)]
#[non_exhaustive]
pub struct PinnedEventsContent {
	pub event_id: String,
	pub pinned: bool,
}

impl PinnedEventsContent {
	pub fn new(event_id: impl Into<String>, pinned: bool) -> Self {
		Self { event_id: event_id.into(), pinned }
	}
}

impl EventType for PinnedEventsContent {
	fn generate_event_type(&self) -> String {
		"m.room.pinned_events".into()
	}
}

impl From<PinnedEventsContent> for EventContent {
	fn from(val: PinnedEventsContent) -> Self {
		EventContent::PinnedEvents(val)
	}
}
