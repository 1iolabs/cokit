// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use cid::Cid;
use co_messaging::{
	user_events::{PostUserStoryContent, UpdateProfileContent, ViewUserStoryContent},
	EventType, MatrixEvent,
};

#[test]
fn test_story_post() {
	let event_content = PostUserStoryContent::new(86400000, 10000, Cid::default());
	let event = MatrixEvent::new("some_event", 1577836800000, "some_room", event_content);
	assert_eq!(event.generate_event_type(), "m.user.story.post");
	let json = serde_json::to_string_pretty(&event).unwrap();
	println!("{json}");
	assert_eq!(event, serde_json::from_str(&json).unwrap());
}

#[test]
fn test_story_view() {
	let event_content = ViewUserStoryContent::new("some_event_id");
	let event = MatrixEvent::new("some_event", 1577836800000, "some_room", event_content);
	assert_eq!(event.generate_event_type(), "m.user.story.view");
	let json = serde_json::to_string_pretty(&event).unwrap();
	println!("{json}");
	assert_eq!(event, serde_json::from_str(&json).unwrap());
}

#[test]
fn test_profile_update() {
	let event_content = UpdateProfileContent::new("Max Mustermann", None, "am mustern");
	let event = MatrixEvent::new("some_event", 1577836800000, "some_room", event_content);
	assert_eq!(event.generate_event_type(), "m.user.profile.update");
	let json = serde_json::to_string_pretty(&event).unwrap();
	println!("{json}");
	assert_eq!(event, serde_json::from_str(&json).unwrap());
}
