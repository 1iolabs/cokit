// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use super::{
	multimedia::{AudioInfo, FileInfo, ImageInfo, LocationInfo},
	poll_event::{PollEndContent, PollResponseContent, PollStartContent},
};
use crate::{matrix_event::relation::RelatesTo, multimedia::VideoInfo, relation::Relation, EventContent};
use cid::Cid;
use co_macros::co;
use co_primitives::{CoCid, Did};
use schemars::JsonSchema;

/// Events that sent actual messages that can be seen by all participants in a room.
#[co]
#[derive(JsonSchema)]
#[serde(tag = "msgtype")]
pub enum MessageType {
	#[serde(rename = "text")]
	Text(TextContent),
	#[serde(rename = "notice")]
	Notice(NoticeContent),
	#[serde(rename = "image")]
	Image(ImageContent),
	#[serde(rename = "audio")]
	Audio(AudioContent),
	#[serde(rename = "video")]
	Video(VideoContent),
	#[serde(rename = "file")]
	File(FileContent),
	#[serde(rename = "location")]
	Location(LocationContent),

	/// All events that interact with or create a poll
	#[serde(rename = "poll_start")]
	Start(PollStartContent),
	#[serde(rename = "poll_response")]
	Response(PollResponseContent),
	#[serde(rename = "poll_end")]
	End(PollEndContent),
}

impl MessageType {
	pub fn generate_message_type(&self) -> String {
		match self {
			MessageType::Text(_) => String::from("m.text"),
			MessageType::Notice(_) => String::from("m.notice"),
			MessageType::Image(_) => String::from("m.image"),
			MessageType::Audio(_) => String::from("m.audio"),
			MessageType::Video(_) => String::from("m.video"),
			MessageType::File(_) => String::from("m.file"),
			MessageType::Location(_) => String::from("m.location"),
			MessageType::Start(_) => String::from("m.poll.start"),
			MessageType::Response(_) => String::from("m.poll.response"),
			MessageType::End(_) => String::from("m.poll.end"),
		}
	}
}

impl Relation for MessageType {
	fn generate_relation_type(&self) -> Option<String> {
		match self {
			MessageType::Text(content) => content.generate_relation_type(),
			MessageType::Notice(content) => content.generate_relation_type(),
			MessageType::Image(content) => content.generate_relation_type(),
			MessageType::Audio(content) => content.generate_relation_type(),
			MessageType::Video(content) => content.generate_relation_type(),
			MessageType::File(content) => content.generate_relation_type(),
			MessageType::Location(content) => content.generate_relation_type(),
			MessageType::Start(content) => content.generate_relation_type(),
			MessageType::Response(content) => content.generate_relation_type(),
			MessageType::End(content) => content.generate_relation_type(),
		}
	}
	fn get_in_reply_to(&self) -> Option<String> {
		match self {
			MessageType::Text(content) => content.get_in_reply_to(),
			MessageType::Notice(content) => content.get_in_reply_to(),
			MessageType::Image(content) => content.get_in_reply_to(),
			MessageType::Audio(content) => content.get_in_reply_to(),
			MessageType::Video(content) => content.get_in_reply_to(),
			MessageType::File(content) => content.get_in_reply_to(),
			MessageType::Location(content) => content.get_in_reply_to(),
			MessageType::Start(content) => content.get_in_reply_to(),
			MessageType::Response(content) => content.get_in_reply_to(),
			MessageType::End(content) => content.get_in_reply_to(),
		}
	}
}

impl From<MessageType> for EventContent {
	fn from(val: MessageType) -> Self {
		EventContent::Message(val)
	}
}

/// Simple trait for all events that have text that is formatted in a specific way. Usually these events need to
/// have fields for the format and the formatted text. Provides declaration for simple functions that provide
/// formatting functionality as well as getters and setters.
pub trait Formattable {
	fn format_body(&self) -> String;
	fn set_format(&mut self, formatted_body: impl Into<String>, format: impl Into<String>);
	fn remove_format(&mut self);
}

/// Used to describe which users got mentioned in the body of a message
#[co]
#[derive(JsonSchema, Default)]
#[non_exhaustive]
pub struct Mentions {
	pub user_ids: Vec<Did>,
}

impl Mentions {
	pub fn new(user_ids: Vec<Did>) -> Self {
		Self { user_ids }
	}
}

/// Formatted body and format are not pub to ensure with setters that formatted body is only set when a format is
/// also given.
#[co]
#[derive(JsonSchema)]
#[non_exhaustive]
pub struct TextContent {
	/// A formatted version of the body
	#[serde(skip_serializing_if = "Option::is_none")]
	formatted_body: Option<String>,
	/// The format used in formatted body
	#[serde(skip_serializing_if = "Option::is_none")]
	format: Option<String>,
	/// The body of the message
	pub body: String,
	/// Users that are mentioned in the body
	#[serde(skip_serializing_if = "Option::is_none")]
	pub mentions: Option<Mentions>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub is_silent: Option<bool>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub relates_to: Option<RelatesTo>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub new_content: Option<Box<EventContent>>,
	/// URL-preview card for a URL mentioned in `body`. Generated by the sender
	/// at send time (there is no server). Additive + optional.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub url_preview: Option<UrlPreview>,
}

impl TextContent {
	pub fn new(body: impl Into<String>) -> Self {
		Self {
			body: body.into(),
			formatted_body: Default::default(),
			format: Default::default(),
			mentions: Default::default(),
			is_silent: Default::default(),
			relates_to: Default::default(),
			new_content: Default::default(),
			url_preview: Default::default(),
		}
	}

	/// Attaches a URL preview, returning `self` for chaining.
	pub fn with_url_preview(mut self, preview: UrlPreview) -> Self {
		self.url_preview = Some(preview);
		self
	}
}

/// Preview image for a [`UrlPreview`], stored as a content-addressed CID in the
/// chat-CO UnixFS (same shape as an image message's `file` + `info`).
#[co]
#[derive(JsonSchema)]
#[non_exhaustive]
pub struct PreviewImage {
	#[schemars(with = "CoCid")]
	pub file: Cid,
	pub info: ImageInfo,
}

impl PreviewImage {
	pub fn new(file: Cid, info: ImageInfo) -> Self {
		Self { file, info }
	}
}

/// OpenGraph-style preview for a URL mentioned in a text message. `image =
/// None` renders a text-only card.
#[co]
#[derive(JsonSchema)]
#[non_exhaustive]
pub struct UrlPreview {
	pub url: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub title: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub description: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub site_name: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub image: Option<PreviewImage>,
}

impl UrlPreview {
	pub fn new(url: impl Into<String>) -> Self {
		Self { url: url.into(), title: None, description: None, site_name: None, image: None }
	}
}

impl From<TextContent> for EventContent {
	fn from(val: TextContent) -> Self {
		MessageType::Text(val).into()
	}
}

impl Formattable for TextContent {
	fn set_format(&mut self, formatted_body: impl Into<String>, format: impl Into<String>) {
		// TODO: validate format
		self.formatted_body = Some(formatted_body.into());
		self.format = Some(format.into());
	}
	fn remove_format(&mut self) {
		self.formatted_body = None;
		self.format = None;
	}
	fn format_body(&self) -> String {
		unimplemented!()
	}
}

impl Relation for TextContent {
	fn generate_relation_type(&self) -> Option<String> {
		match &self.relates_to {
			Some(relates_to) => relates_to.generate_relation_type(),
			None => None,
		}
	}
	fn get_in_reply_to(&self) -> Option<String> {
		match &self.relates_to {
			Some(relates_to) => relates_to.get_in_reply_to(),
			None => None,
		}
	}
}

/// Formatted body and format are not pub to ensure with setters that formatted body is only set when a format is
/// also given
#[co]
#[derive(JsonSchema)]
#[non_exhaustive]
pub struct NoticeContent {
	/// A formatted version of the body
	#[serde(skip_serializing_if = "Option::is_none")]
	formatted_body: Option<String>,
	/// The format used in formatted body
	#[serde(skip_serializing_if = "Option::is_none")]
	format: Option<String>,
	/// The body of the message
	pub body: String,
	/// Users that are mentioned in the body
	#[serde(skip_serializing_if = "Option::is_none")]
	pub mentions: Option<Mentions>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub is_silent: Option<bool>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub relates_to: Option<RelatesTo>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub new_content: Option<Box<EventContent>>,
}

impl NoticeContent {
	pub fn new(body: impl Into<String>) -> Self {
		Self {
			body: body.into(),
			formatted_body: Default::default(),
			format: Default::default(),
			mentions: Default::default(),
			is_silent: Default::default(),
			relates_to: Default::default(),
			new_content: Default::default(),
		}
	}
}

impl From<NoticeContent> for EventContent {
	fn from(val: NoticeContent) -> Self {
		MessageType::Notice(val).into()
	}
}

impl Formattable for NoticeContent {
	fn set_format(&mut self, formatted_body: impl Into<String>, format: impl Into<String>) {
		// TODO: validate format
		self.formatted_body = Some(formatted_body.into());
		self.format = Some(format.into());
	}
	fn remove_format(&mut self) {
		self.formatted_body = None;
		self.format = None;
	}
	fn format_body(&self) -> String {
		unimplemented!()
	}
}

impl Relation for NoticeContent {
	fn generate_relation_type(&self) -> Option<String> {
		match &self.relates_to {
			Some(relates_to) => relates_to.generate_relation_type(),
			None => None,
		}
	}
	fn get_in_reply_to(&self) -> Option<String> {
		match &self.relates_to {
			Some(relates_to) => relates_to.get_in_reply_to(),
			None => None,
		}
	}
}

#[co]
#[derive(JsonSchema)]
#[non_exhaustive]
pub struct ImageContent {
	/// A text representing the image in some way
	pub body: String,
	/// CID to the image file
	#[schemars(with = "CoCid")]
	pub file: Cid,
	/// Image metadata
	pub info: ImageInfo,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub is_silent: Option<bool>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub relates_to: Option<RelatesTo>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub new_content: Option<Box<EventContent>>,
}

impl ImageContent {
	pub fn new(body: impl Into<String>, file: Cid, info: ImageInfo) -> Self {
		Self {
			body: body.into(),
			file,
			info,
			is_silent: Default::default(),
			relates_to: Default::default(),
			new_content: Default::default(),
		}
	}
}

impl From<ImageContent> for EventContent {
	fn from(val: ImageContent) -> Self {
		MessageType::Image(val).into()
	}
}

impl Relation for ImageContent {
	fn generate_relation_type(&self) -> Option<String> {
		match &self.relates_to {
			Some(relates_to) => relates_to.generate_relation_type(),
			None => None,
		}
	}
	fn get_in_reply_to(&self) -> Option<String> {
		match &self.relates_to {
			Some(relates_to) => relates_to.get_in_reply_to(),
			None => None,
		}
	}
}

#[co]
#[derive(JsonSchema)]
#[non_exhaustive]
pub struct AudioContent {
	/// A text representing the audio in same way
	pub body: String,
	/// CID to the audio file
	#[schemars(with = "CoCid")]
	pub file: Cid,
	/// Audio metadata
	pub info: AudioInfo,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub is_silent: Option<bool>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub relates_to: Option<RelatesTo>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub new_content: Option<Box<EventContent>>,
}

impl AudioContent {
	pub fn new(body: impl Into<String>, file: Cid, info: AudioInfo) -> Self {
		Self {
			body: body.into(),
			file,
			info,
			is_silent: Default::default(),
			relates_to: Default::default(),
			new_content: Default::default(),
		}
	}
}

impl From<AudioContent> for EventContent {
	fn from(val: AudioContent) -> Self {
		MessageType::Audio(val).into()
	}
}

impl Relation for AudioContent {
	fn generate_relation_type(&self) -> Option<String> {
		match &self.relates_to {
			Some(relates_to) => relates_to.generate_relation_type(),
			None => None,
		}
	}
	fn get_in_reply_to(&self) -> Option<String> {
		match &self.relates_to {
			Some(relates_to) => relates_to.get_in_reply_to(),
			None => None,
		}
	}
}

#[co]
#[derive(JsonSchema)]
#[non_exhaustive]
pub struct VideoContent {
	/// Textual representation of the video
	pub body: String,
	/// CID to the video
	#[schemars(with = "CoCid")]
	pub file: Cid,
	/// Video metadata
	pub info: VideoInfo,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub is_silent: Option<bool>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub relates_to: Option<RelatesTo>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub new_content: Option<Box<EventContent>>,
}

impl VideoContent {
	pub fn new(body: impl Into<String>, file: Cid, info: VideoInfo) -> Self {
		Self {
			body: body.into(),
			file,
			info,
			is_silent: Default::default(),
			relates_to: Default::default(),
			new_content: Default::default(),
		}
	}
}

impl From<VideoContent> for EventContent {
	fn from(val: VideoContent) -> Self {
		MessageType::Video(val).into()
	}
}

impl Relation for VideoContent {
	fn generate_relation_type(&self) -> Option<String> {
		match &self.relates_to {
			Some(relates_to) => relates_to.generate_relation_type(),
			None => None,
		}
	}
	fn get_in_reply_to(&self) -> Option<String> {
		match &self.relates_to {
			Some(relates_to) => relates_to.get_in_reply_to(),
			None => None,
		}
	}
}

#[co]
#[derive(JsonSchema)]
#[non_exhaustive]
pub struct FileContent {
	/// A text representing the file in some way
	pub body: String,
	/// CID to the file
	#[schemars(with = "CoCid")]
	pub file: Cid,
	/// The name of the file
	pub filename: String,
	/// File metadata
	pub info: FileInfo,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub is_silent: Option<bool>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub relates_to: Option<RelatesTo>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub new_content: Option<Box<EventContent>>,
}

impl FileContent {
	pub fn new(body: impl Into<String>, file: Cid, filename: impl Into<String>, info: FileInfo) -> Self {
		Self {
			body: body.into(),
			file,
			filename: filename.into(),
			info,
			is_silent: Default::default(),
			relates_to: Default::default(),
			new_content: Default::default(),
		}
	}
}

impl From<FileContent> for EventContent {
	fn from(val: FileContent) -> Self {
		MessageType::File(val).into()
	}
}

impl Relation for FileContent {
	fn generate_relation_type(&self) -> Option<String> {
		match &self.relates_to {
			Some(relates_to) => relates_to.generate_relation_type(),
			None => None,
		}
	}
	fn get_in_reply_to(&self) -> Option<String> {
		match &self.relates_to {
			Some(relates_to) => relates_to.get_in_reply_to(),
			None => None,
		}
	}
}

#[co]
#[derive(JsonSchema)]
#[non_exhaustive]
pub struct LocationContent {
	/// Textual representation of the location
	pub body: String,
	/// A geo uri by definition of https://datatracker.ietf.org/doc/html/rfc5870
	pub geo_uri: String,
	/// Location metadata
	pub info: LocationInfo,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub is_silent: Option<bool>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub relates_to: Option<RelatesTo>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub new_content: Option<Box<EventContent>>,
}

impl LocationContent {
	pub fn new(body: impl Into<String>, geo_uri: impl Into<String>, info: LocationInfo) -> Self {
		Self {
			body: body.into(),
			geo_uri: geo_uri.into(),
			info,
			is_silent: Default::default(),
			relates_to: Default::default(),
			new_content: Default::default(),
		}
	}
}

impl From<LocationContent> for EventContent {
	fn from(val: LocationContent) -> Self {
		MessageType::Location(val).into()
	}
}

impl Relation for LocationContent {
	fn generate_relation_type(&self) -> Option<String> {
		match &self.relates_to {
			Some(relates_to) => relates_to.generate_relation_type(),
			None => None,
		}
	}
	fn get_in_reply_to(&self) -> Option<String> {
		match &self.relates_to {
			Some(relates_to) => relates_to.get_in_reply_to(),
			None => None,
		}
	}
}

#[cfg(test)]
mod url_preview_tests {
	use super::*;

	#[test]
	fn text_content_without_preview_deserializes_to_none() {
		let tc: TextContent = serde_json::from_str(r#"{"body":"hi"}"#).unwrap();
		assert!(tc.url_preview.is_none());
	}

	#[test]
	fn text_content_with_preview_round_trips() {
		let mut p = UrlPreview::new("https://example.com/a");
		p.title = Some("T".into());
		p.site_name = Some("example.com".into());
		let tc = TextContent::new("look").with_url_preview(p);
		let json = serde_json::to_string(&tc).unwrap();
		let back: TextContent = serde_json::from_str(&json).unwrap();
		assert_eq!(back.body, "look");
		let up = back.url_preview.expect("preview present");
		assert_eq!(up.url, "https://example.com/a");
		assert_eq!(up.title.as_deref(), Some("T"));
		assert!(up.image.is_none());
	}
}
