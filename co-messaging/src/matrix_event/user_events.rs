// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::{EventContent, EventType};
use cid::Cid;
use co_macros::co;
use co_primitives::CoCid;
use schemars::JsonSchema;

#[co]
#[derive(JsonSchema)]
pub struct PostUserStoryContent {
	/// How long users can view the story after it was posted in ms
	pub lifetime: u64,
	/// How long the story will be shown once opened in ms
	pub display_time: u64,
	/// Content ID for a json file containing the story data
	#[schemars(with = "CoCid")]
	pub content: Cid,
}

impl EventType for PostUserStoryContent {
	fn generate_event_type(&self) -> String {
		"m.user.story.post".into()
	}
}

impl From<PostUserStoryContent> for EventContent {
	fn from(val: PostUserStoryContent) -> Self {
		EventContent::PostStory(val)
	}
}

impl PostUserStoryContent {
	pub fn new(lifetime: u64, display_time: u64, content: Cid) -> Self {
		Self { lifetime, display_time, content }
	}
}

#[co]
#[derive(JsonSchema)]
pub struct ViewUserStoryContent {
	/// ID of the event that containes the viewed story
	pub story: String,
}

impl EventType for ViewUserStoryContent {
	fn generate_event_type(&self) -> String {
		"m.user.story.view".into()
	}
}

impl From<ViewUserStoryContent> for EventContent {
	fn from(val: ViewUserStoryContent) -> Self {
		EventContent::ViewStory(val)
	}
}

impl ViewUserStoryContent {
	pub fn new(story: impl Into<String>) -> Self {
		Self { story: story.into() }
	}
}

#[co]
#[derive(JsonSchema)]
pub struct UpdateProfileContent {
	/// The name that the user likes to use as a default
	pub display_name: String,
	/// Content ID pointing to the avatar of the user
	#[schemars(with = "Option<CoCid>")]
	pub avatar: Option<Cid>,
	/// The current status of the user
	pub status_msg: String,
}

impl EventType for UpdateProfileContent {
	fn generate_event_type(&self) -> String {
		"m.user.profile.update".into()
	}
}

impl From<UpdateProfileContent> for EventContent {
	fn from(val: UpdateProfileContent) -> Self {
		EventContent::UpdateProfile(val)
	}
}

impl UpdateProfileContent {
	pub fn new(display_name: impl Into<String>, avatar: Option<Cid>, status_msg: impl Into<String>) -> Self {
		Self { display_name: display_name.into(), avatar, status_msg: status_msg.into() }
	}
}
