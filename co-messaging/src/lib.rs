mod matrix_event;

// todo
pub static FORMATTED_BODY_FORMAT: &str = "some.html.standard.format";

pub use crate::matrix_event::{
	call_event, ephemeral_event, message_event, multimedia, poll_event, receipts, relation, state_event, user_events,
};
use matrix_event::{
	call_event::CallType,
	ephemeral_event::EphemeralType,
	message_event::MessageType,
	receipts::ReceiptType,
	relation::{ReactionContent, RedactionContent, Relation},
	state_event::StateType,
	user_events::UserType,
};
use serde::{Deserialize, Serialize};

pub trait EventType {
	fn generate_event_type(&self) -> String;
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct MatrixEvent {
	pub event_id: String,
	pub timestamp: i64,
	pub room_id: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub state_key: Option<String>,
	#[serde(flatten)]
	pub content: EventContent,
}

impl MatrixEvent {
	pub fn new(
		event_id: impl Into<String>,
		timestamp: i64,
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
	pub fn set_timestamp(&mut self, ts: i64) {
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

/**
 * Simple enum to fit the different possible contents.
 * Unique event type string can be generated from this using pattern matching.
 */
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(tag = "type", content = "content")]
pub enum EventContent {
	#[serde(rename = "m.room.message")]
	Message(MessageType),
	#[serde(rename = "m.reaction")]
	Reaction(ReactionContent),
	#[serde(rename = "m.room.redaction")]
	Redaction(RedactionContent),
	#[serde(rename = "m.receipt")]
	Receipt(ReceiptType),
	#[serde(untagged)]
	State(StateType),
	#[serde(untagged)]
	Call(CallType),
	#[serde(untagged)]
	Ephemeral(EphemeralType),
	#[serde(untagged)]
	User(UserType),
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
			EventContent::State(state) => state.generate_event_type(),
			EventContent::Call(call) => call.generate_event_type(),
			EventContent::Ephemeral(content) => content.generate_event_type(),
			EventContent::User(content) => content.generate_event_type(),
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
