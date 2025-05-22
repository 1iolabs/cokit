mod matrix_event;

// TODO
pub static FORMATTED_BODY_FORMAT: &str = "some.html.standard.format";

pub use crate::matrix_event::{
	call_event, ephemeral_event, message_event, multimedia, poll_event, receipts, relation, state_event, user_events,
};
use matrix_event::{
	call_event::{
		AnswerCallContent, CallCandidatesContent, CallInviteContent, CallNegotiationContent, HangupCallContent,
		RejectCallContent, SelectCallAnswerContent,
	},
	ephemeral_event::{PresenceContent, TypingContent},
	message_event::MessageType,
	receipts::ReceiptType,
	relation::{ReactionContent, RedactionContent, Relation},
	state_event::{PinnedEventsContent, RoomAvatarContent, RoomNameContent, RoomTopicContent},
	user_events::{PostUserStoryContent, UpdateProfileContent, ViewUserStoryContent},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub trait EventType {
	fn generate_event_type(&self) -> String;
}

/// Collection of all possible actions for the room core
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
pub struct MatrixEvent {
	pub event_id: String,
	pub timestamp: u128,
	pub room_id: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub state_key: Option<String>,
	#[serde(flatten)]
	pub content: EventContent,
}

impl MatrixEvent {
	pub fn new(
		event_id: impl Into<String>,
		timestamp: u128,
		room_id: impl Into<String>,
		content: impl Into<EventContent>,
	) -> Self {
		Self { event_id: event_id.into(), timestamp, room_id: room_id.into(), content: content.into(), state_key: None }
	}
	pub fn event_type(&self) -> String {
		self.content.generate_event_type()
	}
	pub fn set_state_key(&mut self, state_key: String) {
		// todo filter for event types that can have a state key
		self.state_key = Some(state_key);
	}
	pub fn set_timestamp(&mut self, ts: u128) {
		self.timestamp = ts;
	}
}

impl Relation for MatrixEvent {
	fn generate_relation_type(&self) -> Option<String> {
		self.content.generate_relation_type()
	}
	fn get_in_reply_to(&self) -> Option<String> {
		self.content.get_in_reply_to()
	}
}

impl EventType for MatrixEvent {
	fn generate_event_type(&self) -> String {
		self.content.generate_event_type()
	}
}

/// Simple enum to fit the different possible contents.
/// Unique event type string can be generated from this using pattern matching.
///
/// # State events
///
/// All events that in some way alter the state of a room
///
/// - Room name
/// - Room topic
/// - Room avatar
/// - Pinned events in room
///
/// # Call Events
///
/// All call events share the call_id, party_id and version fields
/// call_id: A unique ID that that are used to determine which call events correspond to each other
/// party_id: A unique ID to identify a call participant. May but not must be used for multiple calls. Must be
/// unique across all participants. version: The version of the VoIP specs used for the message. This version
/// is "1". A string is used for experimental versions.
///
/// # Ephemeral events
///
/// Ephemeral events are once-off events that do not need to be saved.
///
/// # User events
///
/// Events for interacting with users profiles
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
#[serde(tag = "type", content = "content")]
pub enum EventContent {
	#[serde(rename = "m_room_message")]
	Message(MessageType),
	#[serde(rename = "m_reaction")]
	Reaction(ReactionContent),
	#[serde(rename = "m_room_redaction")]
	Redaction(RedactionContent),
	#[serde(rename = "m_receipt")]
	Receipt(ReceiptType),

	#[serde(rename = "room_name")]
	RoomName(RoomNameContent),
	#[serde(rename = "room_topic")]
	RoomTopic(RoomTopicContent),
	#[serde(rename = "room_avatar")]
	RoomAvatar(RoomAvatarContent),
	#[serde(rename = "room_pinned_events")]
	PinnedEvents(PinnedEventsContent),

	#[serde(rename = "call_invite")]
	Invite(CallInviteContent),
	#[serde(rename = "call_answer")]
	Answer(AnswerCallContent),
	#[serde(rename = "call_candidates")]
	Candidates(CallCandidatesContent),
	#[serde(rename = "call_select_answer")]
	SelectAnswer(SelectCallAnswerContent),
	#[serde(rename = "call_negotiate")]
	Negotioation(CallNegotiationContent),
	#[serde(rename = "call_reject")]
	Reject(RejectCallContent),
	#[serde(rename = "call_hangup")]
	Hangup(HangupCallContent),

	#[serde(rename = "typing")]
	Typing(TypingContent),
	#[serde(rename = "presence")]
	Presence(PresenceContent),

	#[serde(rename = "user_story_post")]
	PostStory(PostUserStoryContent),
	#[serde(rename = "user_story_view")]
	ViewStory(ViewUserStoryContent),
	#[serde(rename = "user_profile_update")]
	UpdateProfile(UpdateProfileContent),
}

impl EventContent {
	// will generate the message type if it is a EventContent::Message. Returns an error otherwise
	pub fn generate_message_type(&self) -> Result<String, String> {
		match self {
			EventContent::Message(m) => Ok(m.generate_message_type()),
			_ => Err("Not a message".into()),
		}
	}
}

impl EventType for EventContent {
	fn generate_event_type(&self) -> String {
		match self {
			EventContent::Message(_) => "m.room.message".into(),
			EventContent::Reaction(_) => "m.reaction".into(),
			EventContent::Redaction(_) => "m.room.redaction".into(),
			EventContent::Receipt(_) => "m.receipt".into(),
			EventContent::RoomName(content) => content.generate_event_type(),
			EventContent::RoomTopic(content) => content.generate_event_type(),
			EventContent::RoomAvatar(content) => content.generate_event_type(),
			EventContent::PinnedEvents(content) => content.generate_event_type(),
			EventContent::Invite(content) => content.generate_event_type(),
			EventContent::Answer(content) => content.generate_event_type(),
			EventContent::Candidates(content) => content.generate_event_type(),
			EventContent::SelectAnswer(content) => content.generate_event_type(),
			EventContent::Negotioation(content) => content.generate_event_type(),
			EventContent::Reject(content) => content.generate_event_type(),
			EventContent::Hangup(content) => content.generate_event_type(),
			EventContent::Typing(content) => content.generate_event_type(),
			EventContent::Presence(content) => content.generate_event_type(),
			EventContent::PostStory(content) => content.generate_event_type(),
			EventContent::ViewStory(content) => content.generate_event_type(),
			EventContent::UpdateProfile(content) => content.generate_event_type(),
		}
	}
}

impl Relation for EventContent {
	fn generate_relation_type(&self) -> Option<String> {
		match self {
			EventContent::Message(content) => content.generate_relation_type(),
			EventContent::Reaction(content) => content.generate_relation_type(),
			_ => None,
		}
	}
	fn get_in_reply_to(&self) -> Option<String> {
		match self {
			EventContent::Message(content) => content.get_in_reply_to(),
			EventContent::Reaction(content) => content.get_in_reply_to(),
			_ => None,
		}
	}
}
