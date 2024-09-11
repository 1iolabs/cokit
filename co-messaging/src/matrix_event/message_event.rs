use super::{
	multimedia::{AudioInfo, FileInfo, ImageInfo, LocationInfo},
	poll_event::PollMessageType,
};
use crate::{matrix_event::relation::RelatesTo, multimedia::VideoInfo, relation::Relation, EventContent};
use co_primitives::Did;
use libipld::Cid;
use serde::{Deserialize, Serialize};
use typeshare::typeshare;

/**
 * Events that sent actual messages that can be seen by all participants in a room.
 */
#[typeshare]
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(tag = "type")]
pub enum MessageType {
	#[serde(rename = "m.text")]
	Text(TextContent),
	#[serde(rename = "m.notice")]
	Notice(NoticeContent),
	#[serde(rename = "m.image")]
	Image(ImageContent),
	#[serde(rename = "m.audio")]
	Audio(AudioContent),
	#[serde(rename = "m.video")]
	Video(VideoContent),
	#[serde(rename = "m.file")]
	File(FileContent),
	#[serde(rename = "m.location")]
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
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Mentions {
	pub user_ids: Vec<Did>,
}

/**
 * Formatted body and format are not pub to ensure with setters that formatted body is only set when a format is
 * also given.
 */
#[typeshare]
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct TextContent {
	#[serde(skip_serializing_if = "Option::is_none")]
	formatted_body: Option<String>, // A formatted version of the body
	#[serde(skip_serializing_if = "Option::is_none")]
	format: Option<String>, // The format used in formatted body
	pub body: String, // The body of the message
	#[serde(skip_serializing_if = "Option::is_none")]
	pub mentions: Option<Mentions>, // Users that are mentioned in the body
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
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct NoticeContent {
	#[serde(skip_serializing_if = "Option::is_none")]
	formatted_body: Option<String>, // A formatted version of the body
	#[serde(skip_serializing_if = "Option::is_none")]
	format: Option<String>, // The format used in formatted body
	pub body: String, // The body of the message
	#[serde(skip_serializing_if = "Option::is_none")]
	pub mentions: Option<Mentions>, // Users that are mentioned in the body
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
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct ImageContent {
	pub body: String,    // a text representing the image in some way
	pub file: Cid,       // CID to the image file
	pub info: ImageInfo, // image metadata
	#[serde(skip_serializing_if = "Option::is_none")]
	pub is_silent: Option<bool>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub relates_to: Option<RelatesTo>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub new_content: Option<Box<EventContent>>,
}

impl ImageContent {
	pub fn new(body: impl Into<String>, file: Cid, info: ImageInfo) -> Self {
		Self { body: body.into(), file, info, is_silent: None, relates_to: None, new_content: None }
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
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct AudioContent {
	pub body: String,    // a text representing the audio in same way
	pub file: Cid,       // CID to the audio file
	pub info: AudioInfo, // audio metadata
	#[serde(skip_serializing_if = "Option::is_none")]
	pub is_silent: Option<bool>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub relates_to: Option<RelatesTo>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub new_content: Option<Box<EventContent>>,
}

impl AudioContent {
	pub fn new(body: impl Into<String>, file: Cid, info: AudioInfo) -> Self {
		Self { body: body.into(), file, info, is_silent: None, relates_to: None, new_content: None }
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
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct VideoContent {
	pub body: String,    // textual representation of the video
	pub file: Cid,       // CID to the video
	pub info: VideoInfo, // video metadata
	#[serde(skip_serializing_if = "Option::is_none")]
	pub is_silent: Option<bool>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub relates_to: Option<RelatesTo>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub new_content: Option<Box<EventContent>>,
}

impl VideoContent {
	pub fn new(body: impl Into<String>, file: Cid, info: VideoInfo) -> Self {
		Self { body: body.into(), file, info, is_silent: None, relates_to: None, new_content: None }
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
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct FileContent {
	pub body: String,     // a text representing the file in some way
	pub file: Cid,        // CID to the file
	pub filename: String, // the name of the file
	pub info: FileInfo,   // file metadata
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
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct LocationContent {
	pub body: String,       // textual representation of the location
	pub geo_uri: String,    // a geo uri by definition of https://datatracker.ietf.org/doc/html/rfc5870
	pub info: LocationInfo, // location metadata
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
