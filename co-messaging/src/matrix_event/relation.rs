use crate::{EventContent, EventType};
use co_macros::common_event_content;
use serde::{Deserialize, Serialize};

pub trait Relation {
	fn generate_relation_type(&self) -> Option<String>;
	fn get_in_reply_to(&self) -> Option<String>;
}

/// Empty content as the only purpose is holding a relation to another event.
/// Mostly used for annotation events
#[common_event_content]
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct ReactionContent {}

impl ReactionContent {
	pub fn new(relation: RelatesTo) -> Self {
		Self { is_silent: None, relates_to: Some(relation), new_content: None }
	}
}

impl From<ReactionContent> for EventContent {
	fn from(val: ReactionContent) -> Self {
		EventContent::Reaction(val)
	}
}

impl Relation for ReactionContent {
	fn generate_relation_type(&self) -> Option<String> {
		match &self.relates_to {
			Some(content) => content.generate_relation_type(),
			None => None,
		}
	}
	fn get_in_reply_to(&self) -> Option<String> {
		match &self.relates_to {
			Some(content) => content.get_in_reply_to(),
			None => None,
		}
	}
}

impl EventType for ReactionContent {
	fn generate_event_type(&self) -> String {
		"m.reaction".into()
	}
}

/**
 * Used in some event contents to define a relation to other events
 */
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(rename = "m.relates_to")]
pub struct RelatesTo {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub rel_type: Option<RelationType>, // The type of the relation
	#[serde(skip_serializing_if = "Option::is_none")]
	pub event_id: Option<String>, // The ID of the event that is being related to
	#[serde(rename = "m.in_reply_to")]
	#[serde(skip_serializing_if = "Option::is_none")]
	pub in_reply_to: Option<ReplyContent>, /* Special relation to depict replies. Listed extra as this can happen
	                                        * with the other relations simultaneously */
	#[serde(skip_serializing_if = "Option::is_none")]
	pub room_id: Option<String>, // The ID of the room the related-to event is in. Only needed for forwarding.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub key: Option<String>, // Used for annotations. Defines the type of emoji that has been reacted with.
}

impl RelatesTo {
	/// Helper function to create a RelatesTo body used for replies
	pub fn in_reply_to(event_id: impl Into<String>) -> Self {
		Self {
			event_id: None,
			rel_type: None,
			in_reply_to: Some(ReplyContent { event_id: event_id.into() }),
			room_id: None,
			key: None,
		}
	}

	/// Helper function to create a RelatesTo body used for general relations
	pub fn relation(event_id: impl Into<String>, rel_type: RelationType) -> Self {
		Self { event_id: Some(event_id.into()), rel_type: Some(rel_type), in_reply_to: None, room_id: None, key: None }
	}

	/// Helper function to create a RelatesTo body for annotations
	pub fn annotation(event_id: impl Into<String>, key: impl Into<String>) -> Self {
		let mut body = Self::relation(event_id, RelationType::Annotation);
		body.key = Some(key.into());
		body
	}

	/// Helper function to create a RelatesTo body for replacements (edits)
	pub fn replacement(event_id: impl Into<String>) -> Self {
		Self::relation(event_id, RelationType::Replace)
	}

	/// Helper function to create a RelatesTo body for forwarding
	pub fn forward(event_id: impl Into<String>, room_id: impl Into<String>) -> Self {
		let mut body = Self::relation(event_id, RelationType::Forward);
		body.room_id = Some(room_id.into());
		body
	}

	/// Helper function to create a RelatesTo body for threading
	pub fn thread(event_id: impl Into<String>) -> Self {
		Self::relation(event_id, RelationType::Thread)
	}

	/// Helper function to create a RelatesTo body for poll responses
	pub fn poll(event_id: impl Into<String>) -> Self {
		Self::relation(event_id, RelationType::Poll)
	}

	pub fn set_relation(&mut self, event_id: impl Into<String>, rel_type: RelationType) {
		self.event_id = Some(event_id.into());
		self.rel_type = Some(rel_type);
		self.room_id = None;
	}

	pub fn set_forward(&mut self, event_id: impl Into<String>, room_id: impl Into<String>) {
		self.event_id = Some(event_id.into());
		self.rel_type = Some(RelationType::Forward);
		self.room_id = Some(room_id.into());
	}

	pub fn set_in_reply_to(&mut self, event_id: String) {
		self.in_reply_to = Some(ReplyContent { event_id });
	}

	pub fn get_reply_event(&self) -> Option<String> {
		self.in_reply_to.as_ref().map(|reply| reply.event_id.clone())
	}
}

impl Relation for RelatesTo {
	fn generate_relation_type(&self) -> Option<String> {
		match &self.rel_type {
			Some(r) => r.generate_relation_type(),
			None => None,
		}
	}
	fn get_in_reply_to(&self) -> Option<String> {
		self.in_reply_to.as_ref().map(|content| content.event_id.clone())
	}
}

/// Simple enum containing all different types of relation that events can have to other events
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum RelationType {
	#[serde(rename = "m.annotation")]
	Annotation,
	#[serde(rename = "m.replace")]
	Replace,
	#[serde(rename = "m.forward")]
	Forward,
	#[serde(rename = "m.thread")]
	Thread,
	#[serde(rename = "m.poll")]
	Poll,
}

impl Relation for RelationType {
	fn generate_relation_type(&self) -> Option<String> {
		match self {
			RelationType::Annotation => Some("m.annotation".into()),
			RelationType::Replace => Some("m.replace".into()),
			RelationType::Forward => Some("m.forward".into()),
			RelationType::Thread => Some("m.thread".into()),
			RelationType::Poll => Some("m.poll".into()),
		}
	}
	fn get_in_reply_to(&self) -> Option<String> {
		None
	}
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct ReplyContent {
	pub event_id: String,
}

/// Event content used to redact other events. Sender of this event must be either the same as the sender of the
/// original event or a user with the necessary permissions.
/// Redactions are idempotent and irreversible. They do not use the same relation fields as other events
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct RedactionContent {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub reason: Option<String>, // An optional reason field mostly used when event got redacted by another user
	pub redacts: String, // Event ID of the redacted event
}

impl From<RedactionContent> for EventContent {
	fn from(val: RedactionContent) -> Self {
		EventContent::Redaction(val)
	}
}

impl RedactionContent {
	pub fn new(redacts: impl Into<String>, reason: Option<String>) -> Self {
		Self { reason, redacts: redacts.into() }
	}
}
