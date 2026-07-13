// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use cid::Cid;
use co_messaging::{
	message_event::{self, Formattable, LocationContent, Mentions, TextContent},
	multimedia::{AudioInfo, FileInfo, ImageInfo, LocationInfo, ThumbnailInfo, VideoInfo},
	poll_event::{self, PollAnswer},
	relation::{ReactionContent, RedactionContent, RelatesTo, Relation},
	MatrixEvent, FORMATTED_BODY_FORMAT,
};

#[test]
fn test_text_content() {
	let mut event_content = message_event::TextContent::new("Some message");
	event_content.set_format("formatted_body", FORMATTED_BODY_FORMAT);
	let mut mentions = Mentions::default();
	mentions.user_ids = vec!["did:some:user".into()];
	event_content.mentions = Some(mentions);
	let event = MatrixEvent::new("some_event", 1577836800000, "@some.room", event_content);
	assert_eq!(event.event_type(), "m.room.message");
	assert_eq!(event.content.generate_message_type().unwrap(), "m.text");
	// todo test formatting
	let buf = serde_ipld_dagcbor::to_vec(&event).expect("vector");
	let restored_event = serde_ipld_dagcbor::from_slice::<MatrixEvent>(&buf).expect("decoded event");
	let json = serde_json::to_string_pretty(&event).unwrap();
	println!("JSON: {}", json);
	// let serded_event: MatrixEvent = serde_json::from_str(&json).unwrap();
	assert_eq!(event, restored_event);
}

#[test]
fn test_notice_content() {
	let mut event_content = message_event::NoticeContent::new("Some message");
	event_content.set_format("formatted_body", FORMATTED_BODY_FORMAT);
	let event = MatrixEvent::new("some_event", 1577836800000, "@some.room", event_content);
	assert_eq!(event.event_type(), "m.room.message");
	assert_eq!(event.content.generate_message_type().unwrap(), "m.notice");
	// todo test formatting
	let json = serde_json::to_string_pretty(&event).unwrap();
	// println!("JSON: {}", json);
	let serded_event: MatrixEvent = serde_json::from_str(&json).unwrap();
	assert_eq!(event, serded_event);
}

#[test]
fn test_image_content() {
	let thumbnail_info = ThumbnailInfo::new(10, 10, "image/jpeg", 500);
	let info = ImageInfo::new(10, 20, "image/jpeg", 5000, Cid::default(), thumbnail_info);
	let event_content = message_event::ImageContent::new("Some image", Cid::default(), info);
	let event = MatrixEvent::new("some_event", 1577836800000, "@some.room", event_content);
	assert_eq!(event.event_type(), "m.room.message");
	assert_eq!(event.content.generate_message_type().unwrap(), "m.image");
	let json = serde_json::to_string_pretty(&event).unwrap();
	// println!("JSON: {}", json);
	let serded_event: MatrixEvent = serde_json::from_str(&json).unwrap();
	assert_eq!(event, serded_event);
}

#[test]
fn test_audio_content() {
	let info = AudioInfo::new(50, "audio/wav", 5000);
	let event_content = message_event::AudioContent::new("Some message", Cid::default(), info);
	let event = MatrixEvent::new("some_event", 1577836800000, "@some.room", event_content);
	assert_eq!(event.event_type(), "m.room.message");
	assert_eq!(event.content.generate_message_type().unwrap(), "m.audio");
	let json = serde_json::to_string_pretty(&event).unwrap();
	// println!("JSON: {}", json);
	let serded_event: MatrixEvent = serde_json::from_str(&json).unwrap();
	assert_eq!(event, serded_event);
}

#[test]
fn test_video_content() {
	let thumbnail_info = ThumbnailInfo::new(10, 10, "image/jpeg", 500);
	let info = VideoInfo::new(1080, 1690, 50, "video/mp4", 5000, Cid::default(), thumbnail_info);
	let event_content = message_event::VideoContent::new("Some message", Cid::default(), info);
	let event = MatrixEvent::new("some_event", 1577836800000, "@some.room", event_content);
	assert_eq!(event.event_type(), "m.room.message");
	assert_eq!(event.content.generate_message_type().unwrap(), "m.video");
	let json = serde_json::to_string_pretty(&event).unwrap();
	// println!("JSON: {}", json);
	let serded_event: MatrixEvent = serde_json::from_str(&json).unwrap();
	assert_eq!(event, serded_event);
}

#[test]
fn test_file_content() {
	let thumbnail_info = ThumbnailInfo::new(10, 10, "image/jpeg", 500);
	let info = FileInfo::new("application/msword", 5000, Cid::default(), thumbnail_info);
	let event_content = message_event::FileContent::new("Some message", Cid::default(), "", info);
	let event = MatrixEvent::new("some_event", 1577836800000, "@some.room", event_content);
	assert_eq!(event.event_type(), "m.room.message");
	assert_eq!(event.content.generate_message_type().unwrap(), "m.file");
	let json = serde_json::to_string_pretty(&event).unwrap();
	// println!("JSON: {}", json);
	let serded_event: MatrixEvent = serde_json::from_str(&json).unwrap();
	assert_eq!(event, serded_event);
}

#[test]
fn test_location_content() {
	let thumbnail_info = ThumbnailInfo::new(20, 20, "image/jpeg", 500);
	let info = LocationInfo::new(Cid::default(), thumbnail_info);
	let event_content = LocationContent::new("Eiffeltower", "wherever the eiffeltower is", info);
	let event = MatrixEvent::new("some_event", 1577836800000, "@some.room", event_content);
	assert_eq!(event.event_type(), "m.room.message");
	assert_eq!(event.content.generate_message_type().unwrap(), "m.location");
	let json = serde_json::to_string_pretty(&event).unwrap();
	// println!("JSON: {}", json);
	let serded_event: MatrixEvent = serde_json::from_str(&json).unwrap();
	assert_eq!(event, serded_event);
}

#[test]
fn test_poll_start() {
	let mut event_content = poll_event::PollStartContent::new("What?", vec![], poll_event::PollKind::Anonymous);
	event_content.add_answer(PollAnswer::new("test", "Test"));
	event_content.set_max_selection(2);
	let event = MatrixEvent::new("some_poll_start", 1577836800000, "@some.room", event_content);
	assert_eq!(event.event_type(), "m.room.message");
	assert_eq!(event.content.generate_relation_type(), None);
	let json = serde_json::to_string_pretty(&event).unwrap();
	println!("JSON: {}", json);
	let serded_event: MatrixEvent = serde_json::from_str(&json).unwrap();
	assert_eq!(event, serded_event);
}

#[test]
fn test_poll_response() {
	let mut event_content = poll_event::PollResponseContent::new("response", vec![], "some_poll_start");
	event_content.add_answer("test".into());
	let event = MatrixEvent::new("some_poll_response", 1577836800000, "@some.room", event_content);
	assert_eq!(event.event_type(), "m.room.message");
	assert_eq!(event.content.generate_relation_type(), Some("m.poll".into()));
	let json = serde_json::to_string_pretty(&event).unwrap();
	println!("JSON: {}", json);
	let serded_event: MatrixEvent = serde_json::from_str(&json).unwrap();
	assert_eq!(event, serded_event);
}

#[test]
fn test_poll_end() {
	let event_content = poll_event::PollEndContent::new("end", "some_poll_start");
	let event = MatrixEvent::new("some_poll_response", 1577836800000, "@some.room", event_content);
	assert_eq!(event.event_type(), "m.room.message");
	assert_eq!(event.content.generate_relation_type(), Some("m.poll".into()));
	let json = serde_json::to_string_pretty(&event).unwrap();
	println!("JSON: {}", json);
	let serded_event: MatrixEvent = serde_json::from_str(&json).unwrap();
	assert_eq!(event, serded_event);
}

#[test]
fn test_reply() {
	let mut event_content = TextContent::new("some body");
	event_content.relates_to = RelatesTo::in_reply_to("some_event").into();
	let event = MatrixEvent::new("some_event", 1577836800000, "@some.room", event_content);
	assert_eq!(event.get_in_reply_to(), Some("some_event".into()));
	let json = serde_json::to_string_pretty(&event).unwrap();
	println!("JSON: {}", json);
	let serded_event: MatrixEvent = serde_json::from_str(&json).unwrap();
	assert_eq!(event, serded_event);
}

#[test]
fn test_annotation() {
	let event_content = ReactionContent::new(RelatesTo::annotation("some_other_event", "thumbs_up"));
	let event = MatrixEvent::new("some_event", 1577836800000, "@some.room", event_content);
	assert_eq!(event.event_type(), "m.reaction");
	assert_eq!(event.generate_relation_type(), Some("m.annotation".into()));
	let json = serde_json::to_string_pretty(&event).unwrap();
	println!("JSON: {}", json);
	let serded_event: MatrixEvent = serde_json::from_str(&json).unwrap();
	assert_eq!(event, serded_event);
}

#[test]
fn test_redaction() {
	let event_content = RedactionContent::new("some_older_event", None);
	let event = MatrixEvent::new("some_event", 1577836800000, "@some.room", event_content);
	assert_eq!(event.event_type(), "m.room.redaction");
	let json = serde_json::to_string_pretty(&event).unwrap();
	println!("JSON: {}", json);
	let serded_event: MatrixEvent = serde_json::from_str(&json).unwrap();
	assert_eq!(event, serded_event);
}
