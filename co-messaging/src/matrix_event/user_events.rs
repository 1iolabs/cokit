use crate::{EventContent, EventType};
use co_primitives::CoCid;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
pub struct PostUserStoryContent {
	/// How long users can view the story after it was posted in ms
	pub lifetime: u64,
	/// How long the story will be shown once opened in ms
	pub display_time: u64,
	/// Content ID for a json file containing the story data
	pub content: CoCid,
}

impl EventType for PostUserStoryContent {
	fn generate_event_type(&self) -> String {
		"m.user.story.post".into()
	}
}

impl From<PostUserStoryContent> for EventContent {
	fn from(val: PostUserStoryContent) -> Self {
		EventContent::PostStory(val).into()
	}
}

impl PostUserStoryContent {
	pub fn new(lifetime: u64, display_time: u64, content: impl Into<CoCid>) -> Self {
		Self { lifetime, display_time, content: content.into() }
	}
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
pub struct ViewUserStoryContent {
	pub story: String, // ID of the event that containes the viewed story
}

impl EventType for ViewUserStoryContent {
	fn generate_event_type(&self) -> String {
		"m.user.story.view".into()
	}
}

impl From<ViewUserStoryContent> for EventContent {
	fn from(val: ViewUserStoryContent) -> Self {
		EventContent::ViewStory(val).into()
	}
}

impl ViewUserStoryContent {
	pub fn new(story: impl Into<String>) -> Self {
		Self { story: story.into() }
	}
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
pub struct UpdateProfileContent {
	pub display_name: String,  // The name that the user likes to use as a default
	pub avatar: Option<CoCid>, // Content ID pointing to the avatar of the user
	pub status_msg: String,    // The current status of the user
}

impl EventType for UpdateProfileContent {
	fn generate_event_type(&self) -> String {
		"m.user.profile.update".into()
	}
}

impl From<UpdateProfileContent> for EventContent {
	fn from(val: UpdateProfileContent) -> Self {
		EventContent::UpdateProfile(val).into()
	}
}

impl UpdateProfileContent {
	pub fn new(
		display_name: impl Into<String>,
		avatar: impl Into<Option<CoCid>>,
		status_msg: impl Into<String>,
	) -> Self {
		Self { display_name: display_name.into(), avatar: avatar.into(), status_msg: status_msg.into() }
	}
}
