use super::{
	multimedia::{AudioInfo, FileInfo, ImageInfo, LocationInfo},
	poll_event::PollMessageType,
};
use crate::{matrix_event::relation::RelatesTo, multimedia::VideoInfo, relation::Relation, EventContent};
use co_primitives::{CoCid, Did};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use typeshare::typeshare;

/**
 * Events that sent actual messages that can be seen by all participants in a room.
 */
// #[typeshare]
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
#[serde(tag = "type")]
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
	#[serde(untagged)]
	Poll(PollMessageType),
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
			MessageType::Poll(PollMessageType::Start(_)) => String::from("m.poll.start"),
			MessageType::Poll(PollMessageType::Response(_)) => String::from("m.poll.response"),
			MessageType::Poll(PollMessageType::End(_)) => String::from("m.poll.end"),
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
			MessageType::Poll(content) => content.generate_relation_type(),
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
			MessageType::Poll(content) => content.get_in_reply_to(),
		}
	}
}

impl From<MessageType> for EventContent {
	fn from(val: MessageType) -> Self {
		EventContent::Message(val)
	}
}

/**
 * Simple trait for all events that have text that is formatted in a specific way. Usually these events need to
 * have fields for the format and the formatted text. Provides declaration for simple functions that provide
 * formatting functionality as well as getters and setters.
 */
pub trait Formattable {
	fn format_body(&self) -> String;
	fn set_format(&mut self, formatted_body: impl Into<String>, format: impl Into<String>);
	fn remove_format(&mut self);
}

/**
 * Used to describe which users got mentioned in the body of a message
 */
#[typeshare]
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
pub struct Mentions {
	pub user_ids: Vec<Did>,
}

/**
 * Formatted body and format are not pub to ensure with setters that formatted body is only set when a format is
 * also given.
 */
#[typeshare]
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
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
}

impl TextContent {
	pub fn new(body: impl Into<String>) -> Self {
		Self {
			body: body.into(),
			formatted_body: None,
			format: None,
			is_silent: None,
			relates_to: None,
			mentions: None,
			new_content: None,
		}
	}
}

impl From<TextContent> for EventContent {
	fn from(val: TextContent) -> Self {
		MessageType::Text(val).into()
	}
}

impl Formattable for TextContent {
	fn set_format(&mut self, formatted_body: impl Into<String>, format: impl Into<String>) {
		// todo: validate format
		self.formatted_body = Some(formatted_body.into());
		self.format = Some(format.into());
	}
	fn remove_format(&mut self) {
		self.formatted_body = None;
		self.format = None;
	}
	fn format_body(&self) -> String {
		todo!()
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

/**
 * formatted body and format are not pub to ensure with setters that formatted body is only set when a format is
 * also given
 */
#[typeshare]
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
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
			formatted_body: None,
			format: None,
			is_silent: Default::default(),
			relates_to: None,
			mentions: None,
			new_content: None,
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
		// todo validate format
		self.formatted_body = Some(formatted_body.into());
		self.format = Some(format.into());
	}
	fn remove_format(&mut self) {
		self.formatted_body = None;
		self.format = None;
	}
	fn format_body(&self) -> String {
		todo!()
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

#[typeshare]
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
pub struct ImageContent {
	/// a text representing the image in some way
	pub body: String,
	/// CID to the image file
	pub file: CoCid,
	/// image metadata
	pub info: ImageInfo,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub is_silent: Option<bool>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub relates_to: Option<RelatesTo>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub new_content: Option<Box<EventContent>>,
}

impl ImageContent {
	pub fn new(body: impl Into<String>, file: impl Into<CoCid>, info: ImageInfo) -> Self {
		Self { body: body.into(), file: file.into(), info, is_silent: None, relates_to: None, new_content: None }
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

#[typeshare]
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
pub struct AudioContent {
	/// a text representing the audio in same way
	pub body: String,
	/// CID to the audio file
	pub file: CoCid,
	/// audio metadata
	pub info: AudioInfo,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub is_silent: Option<bool>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub relates_to: Option<RelatesTo>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub new_content: Option<Box<EventContent>>,
}

impl AudioContent {
	pub fn new(body: impl Into<String>, file: impl Into<CoCid>, info: AudioInfo) -> Self {
		Self { body: body.into(), file: file.into(), info, is_silent: None, relates_to: None, new_content: None }
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

#[typeshare]
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
pub struct VideoContent {
	/// textual representation of the video
	pub body: String,
	/// CID to the video
	pub file: CoCid,
	/// video metadata
	pub info: VideoInfo,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub is_silent: Option<bool>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub relates_to: Option<RelatesTo>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub new_content: Option<Box<EventContent>>,
}

impl VideoContent {
	pub fn new(body: impl Into<String>, file: impl Into<CoCid>, info: VideoInfo) -> Self {
		Self { body: body.into(), file: file.into(), info, is_silent: None, relates_to: None, new_content: None }
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

#[typeshare]
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
pub struct FileContent {
	/// a text representing the file in some way
	pub body: String,
	/// CID to the file
	pub file: CoCid,
	/// the name of the file
	pub filename: String,
	/// file metadata
	pub info: FileInfo,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub is_silent: Option<bool>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub relates_to: Option<RelatesTo>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub new_content: Option<Box<EventContent>>,
}

impl FileContent {
	pub fn new(body: impl Into<String>, file: impl Into<CoCid>, filename: impl Into<String>, info: FileInfo) -> Self {
		Self {
			body: body.into(),
			file: file.into(),
			filename: filename.into(),
			info,
			is_silent: None,
			relates_to: None,
			new_content: None,
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

#[typeshare]
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
pub struct LocationContent {
	/// textual representation of the location
	pub body: String,
	/// a geo uri by definition of https://datatracker.ietf.org/doc/html/rfc5870
	pub geo_uri: String,
	/// location metadata
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
		Self { body: body.into(), geo_uri: geo_uri.into(), info, is_silent: None, relates_to: None, new_content: None }
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
