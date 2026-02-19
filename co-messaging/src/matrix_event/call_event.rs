// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use crate::{EventContent, EventType};
use co_macros::co_data;
use schemars::JsonSchema;

/// Session description object for sdp offers and answers
#[co_data]
#[derive(JsonSchema)]
pub struct SessionDescription {
	pub sdp: String,
	#[serde(rename = "type")]
	pub offer_type: String,
}

impl SessionDescription {
	pub fn new(sdp: impl Into<String>, offer_type: impl Into<String>) -> Self {
		Self { sdp: sdp.into(), offer_type: offer_type.into() }
	}
}

/// ICE candidate for WebRTC exchange protocol
#[co_data]
#[derive(JsonSchema)]
pub struct ICECandidate {
	pub candidate: String, // SDP 'a' line of the candidate
	#[serde(rename = "sdpMLineIndex")]
	pub sdp_m_line_index: u32, // index of the SDP 'm' line this candidate is intended for
	#[serde(rename = "sdpMid")]
	pub sdp_m_id: String, // the SDP media type this candidate is intended for
}

impl ICECandidate {
	pub fn new(candidate: impl Into<String>, sdp_m_id: impl Into<String>, sdp_m_line_index: u32) -> Self {
		Self { candidate: candidate.into(), sdp_m_id: sdp_m_id.into(), sdp_m_line_index }
	}
}

/// Initial event to invite other parties to a call
#[co_data]
#[derive(JsonSchema)]
pub struct CallInviteContent {
	pub call_id: String,
	pub party_id: String,
	pub version: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	/// DID of the called user. Any user in room may answer if omitted
	pub invitee: Option<String>,
	/// Time in ms during which invite is valid after sending this event
	pub lifetime: u32,
	/// Session description object
	pub offer: SessionDescription,
}

impl From<CallInviteContent> for EventContent {
	fn from(val: CallInviteContent) -> Self {
		EventContent::Invite(val)
	}
}

impl EventType for CallInviteContent {
	fn generate_event_type(&self) -> String {
		"m.call.invite".into()
	}
}

impl CallInviteContent {
	pub fn new(
		call_id: impl Into<String>,
		party_id: impl Into<String>,
		invitee: Option<String>,
		lifetime: u32,
		offer_sdp: impl Into<String>,
	) -> Self {
		Self {
			call_id: call_id.into(),
			party_id: party_id.into(),
			version: "1".into(),
			invitee,
			lifetime,
			offer: SessionDescription::new(offer_sdp, "offer"),
		}
	}
}

///Event used when answering an invite event
#[co_data]
#[derive(JsonSchema)]
pub struct AnswerCallContent {
	pub call_id: String,
	pub party_id: String,
	pub version: String,
	pub answer: SessionDescription,
}

impl From<AnswerCallContent> for EventContent {
	fn from(val: AnswerCallContent) -> Self {
		EventContent::Answer(val)
	}
}

impl EventType for AnswerCallContent {
	fn generate_event_type(&self) -> String {
		"m.call.answer".into()
	}
}

impl AnswerCallContent {
	pub fn new(call_id: impl Into<String>, party_id: impl Into<String>, answer_sdp: impl Into<String>) -> Self {
		Self {
			call_id: call_id.into(),
			party_id: party_id.into(),
			version: "1".into(),
			answer: SessionDescription::new(answer_sdp, "answer"),
		}
	}
}

/// Event used to exchange viable ICE candidates with the other party upon answering a call
#[co_data]
#[derive(JsonSchema)]
pub struct CallCandidatesContent {
	pub call_id: String,
	pub party_id: String,
	pub version: String,
	pub candidates: Vec<ICECandidate>,
}

impl From<CallCandidatesContent> for EventContent {
	fn from(val: CallCandidatesContent) -> Self {
		EventContent::Candidates(val)
	}
}

impl EventType for CallCandidatesContent {
	fn generate_event_type(&self) -> String {
		"m.call.candidates".into()
	}
}

impl CallCandidatesContent {
	pub fn new(call_id: impl Into<String>, party_id: impl Into<String>, candidates: Vec<ICECandidate>) -> Self {
		Self { call_id: call_id.into(), party_id: party_id.into(), version: "1".into(), candidates }
	}
}

/// Event used to select one of possibly multiple call answers
#[co_data]
#[derive(JsonSchema)]
pub struct SelectCallAnswerContent {
	pub call_id: String,
	pub party_id: String,
	pub version: String,
	/// Party id of the participant whose answer has been selected
	pub selected_party_id: String,
}

impl From<SelectCallAnswerContent> for EventContent {
	fn from(val: SelectCallAnswerContent) -> Self {
		EventContent::SelectAnswer(val)
	}
}

impl EventType for SelectCallAnswerContent {
	fn generate_event_type(&self) -> String {
		"m.call.select_answer".into()
	}
}

impl SelectCallAnswerContent {
	pub fn new(call_id: impl Into<String>, party_id: impl Into<String>, selected_party_id: impl Into<String>) -> Self {
		Self {
			call_id: call_id.into(),
			party_id: party_id.into(),
			version: "1".into(),
			selected_party_id: selected_party_id.into(),
		}
	}
}

/// Event used to renegotiate between participants. First an offer containing a lifetime is sent. Other participants
/// then send an answer. Offer and answer should never both be set. To ensure this they are not public to force the
/// users to use the setters.
#[co_data]
#[derive(JsonSchema)]
pub struct CallNegotiationContent {
	pub call_id: String,
	pub party_id: String,
	pub version: String,
	/// Session description object for negotioation answers
	#[serde(skip_serializing_if = "Option::is_none")]
	answer: Option<SessionDescription>,
	/// Session description object for negotioation offers
	#[serde(skip_serializing_if = "Option::is_none")]
	offer: Option<SessionDescription>,
	/// Time in ms before offer timeout
	#[serde(skip_serializing_if = "Option::is_none")]
	lifetime: Option<u32>,
}

impl From<CallNegotiationContent> for EventContent {
	fn from(val: CallNegotiationContent) -> Self {
		EventContent::Negotioation(val)
	}
}

impl EventType for CallNegotiationContent {
	fn generate_event_type(&self) -> String {
		"m.call.negotiate".into()
	}
}

impl CallNegotiationContent {
	pub fn offer(
		call_id: impl Into<String>,
		party_id: impl Into<String>,
		offer_sdp: impl Into<String>,
		lifetime: u32,
	) -> Self {
		Self {
			call_id: call_id.into(),
			party_id: party_id.into(),
			version: "1".into(),
			answer: None,
			offer: Some(SessionDescription::new(offer_sdp, "offer")),
			lifetime: Some(lifetime),
		}
	}
	pub fn answer(call_id: impl Into<String>, party_id: impl Into<String>, answer_sdp: impl Into<String>) -> Self {
		Self {
			call_id: call_id.into(),
			party_id: party_id.into(),
			version: "1".into(),
			answer: Some(SessionDescription::new(answer_sdp, "answer")),
			offer: None,
			lifetime: None,
		}
	}
	pub fn set_offer(&mut self, offer_sdp: impl Into<String>, lifetime: u32) {
		self.offer = Some(SessionDescription::new(offer_sdp, "offer"));
		self.lifetime = Some(lifetime);
		self.answer = None;
	}
	pub fn set_answer(&mut self, answer_sdp: impl Into<String>) {
		self.answer = Some(SessionDescription::new(answer_sdp, "answer"));
		self.offer = None;
		self.lifetime = None;
	}
	pub fn get_offer(&self) -> Option<SessionDescription> {
		self.offer.clone()
	}
	pub fn get_answer(&self) -> Option<SessionDescription> {
		self.answer.clone()
	}
	pub fn get_lifetime(&self) -> Option<u32> {
		self.lifetime
	}
}

/// Event sent if call was rejected by a user.
#[co_data]
#[derive(JsonSchema)]
pub struct RejectCallContent {
	pub call_id: String,
	pub party_id: String,
	pub version: String,
}

impl From<RejectCallContent> for EventContent {
	fn from(val: RejectCallContent) -> Self {
		EventContent::Reject(val)
	}
}

impl EventType for RejectCallContent {
	fn generate_event_type(&self) -> String {
		"m.call.reject".into()
	}
}

impl RejectCallContent {
	pub fn new(call_id: impl Into<String>, party_id: impl Into<String>) -> Self {
		Self { call_id: call_id.into(), party_id: party_id.into(), version: "1".into() }
	}
}

/// Enum containg possible reasons for a hangup event
#[co_data]
#[derive(JsonSchema)]
pub enum HangupCallReason {
	/// ICE negotiation has failed and connection could not be established
	#[serde(rename = "ice_failed")]
	IceFailed,
	/// Connection failed after some media was exchanged. Includes when renegotiation fails if media was sent prviously
	#[serde(rename = "ice_timeout")]
	IceTimeout,
	/// The other party did not answer in time
	#[serde(rename = "invite_timeout")]
	InviteTimeout,
	/// User actively chooses to end the call
	#[serde(rename = "user_hangup")]
	UserHangup,
	/// Client was unable to start capturing media in such a way that it is unable to continue the call
	#[serde(rename = "user_media_failed")]
	UserMediaFailed,
	/// User is busy. Exists primarily for bridging and does not include when user is in a call already
	#[serde(rename = "user_busy")]
	UserBusy,
	/// Some other error occured that is not described by the above
	#[serde(rename = "unknown_error")]
	UnknownError,
}

/// Hangup event used to signal the termination of the call.
#[co_data]
#[derive(JsonSchema)]
pub struct HangupCallContent {
	pub call_id: String,
	pub party_id: String,
	pub version: String,
	pub reason: HangupCallReason,
}

impl From<HangupCallContent> for EventContent {
	fn from(val: HangupCallContent) -> Self {
		EventContent::Hangup(val)
	}
}

impl EventType for HangupCallContent {
	fn generate_event_type(&self) -> String {
		"m.call.hangup".into()
	}
}

impl HangupCallContent {
	pub fn new(call_id: impl Into<String>, party_id: impl Into<String>, reason: HangupCallReason) -> Self {
		Self { call_id: call_id.into(), party_id: party_id.into(), version: "1".into(), reason }
	}
}
