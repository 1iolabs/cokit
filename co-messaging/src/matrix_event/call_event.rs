use serde::{Deserialize, Serialize};

use crate::{EventContent, EventType};

/**
 * Session description object for sdp offers and answers
 */
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct SessionDescription {
    pub sdp: String,
    #[serde(rename = "type")]
    pub offer_type: String,
}

impl SessionDescription {
    pub fn new(sdp: impl Into<String>, offer_type: impl Into<String>) -> Self {
        Self {
            sdp: sdp.into(),
            offer_type: offer_type.into(),
        }
    }
}

/**
 * ICE candidate for WebRTC exchange protocol
 */
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct ICECandidate {
    pub candidate: String, // SDP 'a' line of the candidate
    #[serde(rename = "sdpMLineIndex")]
    pub sdp_m_line_index: i64, // index of the SDP 'm' line this candidate is intended for
    #[serde(rename = "sdpMid")]
    pub sdp_m_id: String, // the SDP media type this candidate is intended for
}

impl ICECandidate {
    pub fn new(
        candidate: impl Into<String>,
        sdp_m_id: impl Into<String>,
        sdp_m_line_index: i64,
    ) -> Self {
        Self {
            candidate: candidate.into(),
            sdp_m_id: sdp_m_id.into(),
            sdp_m_line_index,
        }
    }
}

/**
 * All call events share the call_id, party_id and version fields
 * call_id: A unique ID that that are used to determine which call events correspond to each other
 * party_id: A unique ID to identify a call participant. May but not must be used for multiple calls. Must be unique across all participants.
 * version: The version of the VoIP specs used for the message. This version is "1". A string is used for experimental versions.
 */
#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(tag = "type", content = "content")]
pub enum CallType {
    #[serde(rename = "m.call.invite")]
    Invite(CallInviteContent),
    #[serde(rename = "m.call.answer")]
    Answer(AnswerCallContent),
    #[serde(rename = "m.call.candidates")]
    Candidates(CallCandidatesContent),
    #[serde(rename = "m.call.select_answer")]
    SelectAnswer(SelectCallAnswerContent),
    #[serde(rename = "m.call.negotiate")]
    Negotioation(CallNegotiationContent),
    #[serde(rename = "m.call.reject")]
    Reject(RejectCallContent),
    #[serde(rename = "m.call.hangup")]
    Hangup(HangupCallContent),
}

impl EventType for CallType {
    fn generate_event_type(&self) -> String {
        match self {
            CallType::Invite(content) => content.generate_event_type(),
            CallType::Answer(content) => content.generate_event_type(),
            CallType::Candidates(content) => content.generate_event_type(),
            CallType::SelectAnswer(content) => content.generate_event_type(),
            CallType::Negotioation(content) => content.generate_event_type(),
            CallType::Reject(content) => content.generate_event_type(),
            CallType::Hangup(content) => content.generate_event_type(),
        }
    }
}

impl Into<EventContent> for CallType {
    fn into(self) -> EventContent {
        EventContent::Call(self)
    }
}

/**
 * Initial event to invite other parties to a call
 */
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct CallInviteContent {
    pub call_id: String,
    pub party_id: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invitee: Option<String>, // DID of the called user. Any user in room may answer if omitted
    pub lifetime: i64, // Time in ms during which invite is valid after sending this event
    pub offer: SessionDescription, // Session description object
}

impl Into<EventContent> for CallInviteContent {
    fn into(self) -> EventContent {
        CallType::Invite(self).into()
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
        lifetime: i64,
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

/**
 * Event used when answering an invite event
 */
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct AnswerCallContent {
    pub call_id: String,
    pub party_id: String,
    pub version: String,
    pub answer: SessionDescription,
}

impl Into<EventContent> for AnswerCallContent {
    fn into(self) -> EventContent {
        CallType::Answer(self).into()
    }
}

impl EventType for AnswerCallContent {
    fn generate_event_type(&self) -> String {
        "m.call.answer".into()
    }
}

impl AnswerCallContent {
    pub fn new(
        call_id: impl Into<String>,
        party_id: impl Into<String>,
        answer_sdp: impl Into<String>,
    ) -> Self {
        Self {
            call_id: call_id.into(),
            party_id: party_id.into(),
            version: "1".into(),
            answer: SessionDescription::new(answer_sdp, "answer"),
        }
    }
}

/**
 * Event used to exchange viable ICE candidates with the other party upon answering a call
 */
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct CallCandidatesContent {
    pub call_id: String,
    pub party_id: String,
    pub version: String,
    pub candidates: Vec<ICECandidate>,
}

impl Into<EventContent> for CallCandidatesContent {
    fn into(self) -> EventContent {
        CallType::Candidates(self).into()
    }
}

impl EventType for CallCandidatesContent {
    fn generate_event_type(&self) -> String {
        "m.call.candidates".into()
    }
}

impl CallCandidatesContent {
    pub fn new(
        call_id: impl Into<String>,
        party_id: impl Into<String>,
        candidates: Vec<ICECandidate>,
    ) -> Self {
        Self {
            call_id: call_id.into(),
            party_id: party_id.into(),
            version: "1".into(),
            candidates,
        }
    }
}

/**
 * Event used to select one of possibly multiple call answers
 */
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct SelectCallAnswerContent {
    pub call_id: String,
    pub party_id: String,
    pub version: String,
    pub selected_party_id: String, // party id of the participant whose answer has been selected
}

impl Into<EventContent> for SelectCallAnswerContent {
    fn into(self) -> EventContent {
        CallType::SelectAnswer(self).into()
    }
}

impl EventType for SelectCallAnswerContent {
    fn generate_event_type(&self) -> String {
        "m.call.select_answer".into()
    }
}

impl SelectCallAnswerContent {
    pub fn new(
        call_id: impl Into<String>,
        party_id: impl Into<String>,
        selected_party_id: impl Into<String>,
    ) -> Self {
        Self {
            call_id: call_id.into(),
            party_id: party_id.into(),
            version: "1".into(),
            selected_party_id: selected_party_id.into(),
        }
    }
}

/**
 * Event used to renegotiate between participants. First an offer containing a lifetime is sent. Other participants then send an answer.
 * Offer and answer should never both be set. To ensure this they are not public to force the users to use the setters.
 */
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct CallNegotiationContent {
    pub call_id: String,
    pub party_id: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    answer: Option<SessionDescription>, // session description object for negotioation answers
    #[serde(skip_serializing_if = "Option::is_none")]
    offer: Option<SessionDescription>, // Session description object for negotioation offers
    #[serde(skip_serializing_if = "Option::is_none")]
    lifetime: Option<i64>, // Time in ms before offer timeout
}

impl Into<EventContent> for CallNegotiationContent {
    fn into(self) -> EventContent {
        CallType::Negotioation(self).into()
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
        lifetime: i64,
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
    pub fn answer(
        call_id: impl Into<String>,
        party_id: impl Into<String>,
        answer_sdp: impl Into<String>,
    ) -> Self {
        Self {
            call_id: call_id.into(),
            party_id: party_id.into(),
            version: "1".into(),
            answer: Some(SessionDescription::new(answer_sdp, "answer")),
            offer: None,
            lifetime: None,
        }
    }
    pub fn set_offer(&mut self, offer_sdp: impl Into<String>, lifetime: i64) {
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
    pub fn get_lifetime(&self) -> Option<i64> {
        self.lifetime
    }
}

/**
 * Event sent if call was rejected by a user.
 */
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct RejectCallContent {
    pub call_id: String,
    pub party_id: String,
    pub version: String,
}

impl Into<EventContent> for RejectCallContent {
    fn into(self) -> EventContent {
        CallType::Reject(self).into()
    }
}

impl EventType for RejectCallContent {
    fn generate_event_type(&self) -> String {
        "m.call.reject".into()
    }
}

impl RejectCallContent {
    pub fn new(call_id: impl Into<String>, party_id: impl Into<String>) -> Self {
        Self {
            call_id: call_id.into(),
            party_id: party_id.into(),
            version: "1".into(),
        }
    }
}

/**
 * Enum containg possible reasons for a hangup event
 */
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum HangupCallReason {
    #[serde(rename = "ice_failed")]
    IceFailed, // ICE negotiation has failed and connection could not be established
    #[serde(rename = "ice_timeout")]
    IceTimeout, // Connection failed after some media was exchanged. Includes when renegotiation fails if media was sent prviously
    #[serde(rename = "invite_timeout")]
    InviteTimeout, // The other party did not answer in time
    #[serde(rename = "user_hangup")]
    UserHangup, // User actively chooses to end the call
    #[serde(rename = "user_media_failed")]
    UserMediaFailed, // Client was unable to start capturing media in such a way that it is unable to continue the call
    #[serde(rename = "user_busy")]
    UserBusy, // User is busy. Exists primarily for bridging and does not include when user is in a call already
    #[serde(rename = "unknown_error")]
    UnknownError, // Some other error occured that is not described by the above
}

/**
 * Hangup event used to signal the termination of the call.
 */
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct HangupCallContent {
    pub call_id: String,
    pub party_id: String,
    pub version: String,
    pub reason: HangupCallReason,
}

impl Into<EventContent> for HangupCallContent {
    fn into(self) -> EventContent {
        CallType::Hangup(self).into()
    }
}

impl EventType for HangupCallContent {
    fn generate_event_type(&self) -> String {
        "m.call.hangup".into()
    }
}

impl HangupCallContent {
    pub fn new(
        call_id: impl Into<String>,
        party_id: impl Into<String>,
        reason: HangupCallReason,
    ) -> Self {
        Self {
            call_id: call_id.into(),
            party_id: party_id.into(),
            version: "1".into(),
            reason,
        }
    }
}
