use crate::{matrix_event::relation::RelatesTo, message_event::MessageType, relation::Relation, EventContent};
use co_macros::common_event_content;
use serde::{Deserialize, Serialize};

/**
 * All events that interact with or create a poll
 */
#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(tag = "msgtype")]
pub enum PollMessageType {
	#[serde(rename = "m.poll.start")]
	Start(PollStartContent),
	#[serde(rename = "m.poll.response")]
	Response(PollResponseContent),
	#[serde(rename = "m.poll.end")]
	End(PollEndContent),
}

impl Into<EventContent> for PollMessageType {
	fn into(self) -> EventContent {
		MessageType::Poll(self).into()
	}
}

impl Relation for PollMessageType {
	fn generate_relation_type(&self) -> Option<String> {
		match self {
			PollMessageType::Start(content) => content.generate_relation_type(),
			PollMessageType::Response(content) => content.generate_relation_type(),
			PollMessageType::End(content) => content.generate_relation_type(),
		}
	}
	fn get_in_reply_to(&self) -> Option<String> {
		match self {
			PollMessageType::Start(content) => content.get_in_reply_to(),
			PollMessageType::Response(content) => content.get_in_reply_to(),
			PollMessageType::End(content) => content.get_in_reply_to(),
		}
	}
}

/**
 * Event used to create a poll.
 */
#[common_event_content]
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct PollStartContent {
	pub body: String,           // A textual representation of the poll, i.e. the question
	pub info: PollCreationInfo, // Information about the created poll
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

impl Into<EventContent> for PollStartContent {
	fn into(self) -> EventContent {
		PollMessageType::Start(self).into()
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

/**
 * metadata for poll creation event
 */
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct PollCreationInfo {
	pub question: String,         // the question the poll was created for
	pub answers: Vec<PollAnswer>, // vector with possible answers
	pub kind: PollKind,           // what kind of poll this is
	max_selections: u8,           // the maximum number of answers users can select. Default is 1 and cannot be less
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

/**
 * One possible answer in a poll. ID should be unique across answers.
 */
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct PollAnswer {
	pub id: String,     // Unique ID to identify an answer
	pub answer: String, // Text of the answer
}

impl PollAnswer {
	pub fn new(id: impl Into<String>, answer: impl Into<String>) -> Self {
		Self { id: id.into(), answer: answer.into() }
	}
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum PollKind {
	#[serde(rename = "disclosed")]
	Disclosed, // In disclosed polls all participants can see the already cast votes (including who cast them)
	#[serde(rename = "undisclosed")]
	Undisclosed, // In undisclosed polls the votes will only appear when the poll has ended
	#[serde(rename = "anonymous")]
	Anonymous, // As undisclosed but voters will stay hidden even after poll has ended
}

#[common_event_content]
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct PollResponseContent {
	pub body: String,         // Textual representation of the answers
	pub answers: Vec<String>, // List of IDs of the answers the user has responded with
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

impl Into<EventContent> for PollResponseContent {
	fn into(self) -> EventContent {
		PollMessageType::Response(self).into()
	}
}

/**
 * Event that closes the poll. For undisclosed and anonymous polls, this is the point where the reults are shown.
 */
#[common_event_content]
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct PollEndContent {
	pub body: String, // Textual representation of the poll ending
}

impl PollEndContent {
	pub fn new(body: impl Into<String>, poll_event: impl Into<String>) -> Self {
		Self { body: body.into(), is_silent: None, relates_to: Some(RelatesTo::poll(poll_event)), new_content: None }
	}
}

impl Into<EventContent> for PollEndContent {
	fn into(self) -> EventContent {
		PollMessageType::End(self).into()
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
