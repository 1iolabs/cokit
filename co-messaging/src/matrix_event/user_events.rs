use crate::{EventContent, EventType};
use libipld::Cid;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(tag = "type", content = "content")]
pub enum UserType {
	#[serde(rename = "m.user.story.post")]
	PostStory(PostUserStoryContent),
	#[serde(rename = "m.user.story.view")]
	ViewStory(ViewUserStoryContent),
	#[serde(rename = "m.user.profile.update")]
	UpdateProfile(UpdateProfileContent),
}

impl EventType for UserType {
	fn generate_event_type(&self) -> String {
		match self {
			UserType::PostStory(content) => content.generate_event_type(),
			UserType::ViewStory(content) => content.generate_event_type(),
			UserType::UpdateProfile(content) => content.generate_event_type(),
		}
	}
}

impl Into<EventContent> for UserType {
	fn into(self) -> EventContent {
		EventContent::User(self)
	}
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct PostUserStoryContent {
	pub lifetime: u64,     // How long users can view the story after it was posted in ms
	pub display_time: u64, // How long the story will be shown once opened in ms
	pub content: Cid,      // Content ID for a json file containing the story data
}

impl EventType for PostUserStoryContent {
	fn generate_event_type(&self) -> String {
		"m.user.story.post".into()
	}
}

impl Into<EventContent> for PostUserStoryContent {
	fn into(self) -> EventContent {
		UserType::PostStory(self).into()
	}
}

impl PostUserStoryContent {
	pub fn new(lifetime: u64, display_time: u64, content: Cid) -> Self {
		Self { lifetime, display_time, content }
	}
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct ViewUserStoryContent {
	pub story: String, // ID of the event that containes the viewed story
}

impl EventType for ViewUserStoryContent {
	fn generate_event_type(&self) -> String {
		"m.user.story.view".into()
	}
}

impl Into<EventContent> for ViewUserStoryContent {
	fn into(self) -> EventContent {
		UserType::ViewStory(self).into()
	}
}

impl ViewUserStoryContent {
	pub fn new(story: impl Into<String>) -> Self {
		Self { story: story.into() }
	}
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct UpdateProfileContent {
	pub display_name: String, // The name that the user likes to use as a default
	pub avatar: Cid,          // Content ID pointing to the avatar of the user
	pub status_msg: String,   // The current status of the user
}

impl EventType for UpdateProfileContent {
	fn generate_event_type(&self) -> String {
		"m.user.profile.update".into()
	}
}

impl Into<EventContent> for UpdateProfileContent {
	fn into(self) -> EventContent {
		UserType::UpdateProfile(self).into()
	}
}

impl UpdateProfileContent {
	pub fn new(display_name: impl Into<String>, avatar: Cid, status_msg: impl Into<String>) -> Self {
		Self { display_name: display_name.into(), avatar, status_msg: status_msg.into() }
	}
}
