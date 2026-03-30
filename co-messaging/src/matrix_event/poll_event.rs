// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and
// authorized users/tools only; do not copy, disclose, or transmit any part outside this domain. No license is granted
// by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or
// retention—approved secure tools may process solely for internal use.

use crate::{matrix_event::relation::RelatesTo, message_event::MessageType, relation::Relation, EventContent};
use co_macros::co;
use schemars::JsonSchema;

/// Event used to create a poll.
#[co]
#[derive(JsonSchema)]
pub struct PollStartContent {
	/// A textual representation of the poll, i.e. the question
	pub body: String,
	/// Information about the created poll
	pub info: PollCreationInfo,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub is_silent: Option<bool>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub relates_to: Option<RelatesTo>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub new_content: Option<Box<EventContent>>,
}

impl PollStartContent {
	pub fn new(question: impl Into<String>, answers: Vec<PollAnswer>, kind: PollKind) -> Self {
		let question: String = question.into();
		Self {
			body: question.clone(),
			info: PollCreationInfo::new(question, answers, kind),
			is_silent: None,
			relates_to: None,
			new_content: None,
		}
	}
	pub fn add_answer(&mut self, answer: PollAnswer) {
		self.info.add_answer(answer)
	}
	pub fn set_max_selection(&mut self, max_selections: u8) {
		self.info.set_max_selection(max_selections);
	}
}

impl From<PollStartContent> for EventContent {
	fn from(val: PollStartContent) -> Self {
		MessageType::Start(val).into()
	}
}

impl Relation for PollStartContent {
	fn generate_relation_type(&self) -> Option<String> {
		match &self.relates_to {
			Some(content) => content.generate_relation_type(),
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

/// Metadata for poll creation event
#[co]
#[derive(JsonSchema)]
pub struct PollCreationInfo {
	/// The question the poll was created for
	pub question: String,
	/// Vector with possible answers
	pub answers: Vec<PollAnswer>,
	/// What kind of poll this is
	pub kind: PollKind,
	/// The maximum number of answers users can select. Default is 1 and cannot be less
	max_selections: u8,
}

impl PollCreationInfo {
	pub fn new(question: impl Into<String>, answers: Vec<PollAnswer>, kind: PollKind) -> Self {
		Self { question: question.into(), answers, kind, max_selections: 1 }
	}
	pub fn add_answer(&mut self, answer: PollAnswer) {
		self.answers.push(answer);
	}
	pub fn set_max_selection(&mut self, max_selections: u8) {
		if max_selections >= 1 {
			self.max_selections = max_selections;
		}
	}
}

/// One possible answer in a poll. ID should be unique across answers.
#[co]
#[derive(JsonSchema)]
pub struct PollAnswer {
	/// Unique ID to identify an answer
	pub id: String,
	/// Text of the answer
	pub answer: String,
}

impl PollAnswer {
	pub fn new(id: impl Into<String>, answer: impl Into<String>) -> Self {
		Self { id: id.into(), answer: answer.into() }
	}
}

#[co]
#[derive(JsonSchema)]
pub enum PollKind {
	/// In disclosed polls all participants can see the already cast votes (including who cast them)
	#[serde(rename = "disclosed")]
	Disclosed,
	/// In undisclosed polls the votes will only appear when the poll has ended
	#[serde(rename = "undisclosed")]
	Undisclosed,
	/// As undisclosed but voters will stay hidden even after poll has ended
	#[serde(rename = "anonymous")]
	Anonymous,
}

#[co]
#[derive(JsonSchema)]
pub struct PollResponseContent {
	/// Textual representation of the answers
	pub body: String,
	/// List of IDs of the answers the user has responded with
	pub answers: Vec<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub is_silent: Option<bool>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub relates_to: Option<RelatesTo>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub new_content: Option<Box<EventContent>>,
}

impl PollResponseContent {
	pub fn new(body: impl Into<String>, answers: Vec<String>, poll_event: impl Into<String>) -> Self {
		Self {
			body: body.into(),
			answers,
			is_silent: None,
			relates_to: Some(RelatesTo::poll(poll_event)),
			new_content: None,
		}
	}
	pub fn add_answer(&mut self, answer: String) {
		self.answers.push(answer);
	}
}

impl Relation for PollResponseContent {
	fn generate_relation_type(&self) -> Option<String> {
		match &self.relates_to {
			Some(content) => content.generate_relation_type(),
			None => None,
		}
	}
	fn get_in_reply_to(&self) -> Option<String> {
		// Poll response cannot be in reply to other events
		None
	}
}

impl From<PollResponseContent> for EventContent {
	fn from(val: PollResponseContent) -> Self {
		MessageType::Response(val).into()
	}
}

/// Event that closes the poll. For undisclosed and anonymous polls, this is the point where the reults are shown.
#[co]
#[derive(JsonSchema)]
pub struct PollEndContent {
	/// Textual representation of the poll ending
	pub body: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub is_silent: Option<bool>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub relates_to: Option<RelatesTo>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub new_content: Option<Box<EventContent>>,
}

impl PollEndContent {
	pub fn new(body: impl Into<String>, poll_event: impl Into<String>) -> Self {
		Self { body: body.into(), is_silent: None, relates_to: Some(RelatesTo::poll(poll_event)), new_content: None }
	}
}

impl From<PollEndContent> for EventContent {
	fn from(val: PollEndContent) -> Self {
		MessageType::End(val).into()
	}
}

impl Relation for PollEndContent {
	fn generate_relation_type(&self) -> Option<String> {
		match &self.relates_to {
			Some(content) => content.generate_relation_type(),
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
