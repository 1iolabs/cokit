// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use co_messaging::{
	call_event::{
		AnswerCallContent, CallCandidatesContent, CallInviteContent, CallNegotiationContent, HangupCallContent,
		HangupCallReason, ICECandidate, RejectCallContent, SelectCallAnswerContent,
	},
	EventType, MatrixEvent,
};

#[test]
fn init_call() {
	let mut event_content = CallInviteContent::new("call_1", "some_device_address", None, 10000, "some_sdp_string");
	event_content.invitee = Some("did:some:invitee".into());
	let event = MatrixEvent::new("some_event", 1577836800000, "some_room", event_content);
	assert_eq!(event.generate_event_type(), "m.call.invite");
	let json = serde_json::to_string_pretty(&event).unwrap();
	println!("{json}");
	assert_eq!(event, serde_json::from_str(&json).unwrap())
}

#[test]
fn answer_call() {
	let event_content = AnswerCallContent::new("call_1", "some_other_device_address", "some_sdp_string");
	let event = MatrixEvent::new("some_event", 1577836800000, "some_room", event_content);
	assert_eq!(event.generate_event_type(), "m.call.answer");
	let json = serde_json::to_string_pretty(&event).unwrap();
	println!("{json}");
	assert_eq!(event, serde_json::from_str(&json).unwrap())
}

#[test]
fn call_candidates() {
	let event_content = CallCandidatesContent::new(
		"call_1",
		"some_device_address",
		vec![ICECandidate::new("some_candidate", "video", 0), ICECandidate::new("some_other_candidate", "audio", 1)],
	);
	let event = MatrixEvent::new("some_event", 1577836800000, "some_room", event_content);
	assert_eq!(event.generate_event_type(), "m.call.candidates");
	let json = serde_json::to_string_pretty(&event).unwrap();
	println!("{json}");
	assert_eq!(event, serde_json::from_str(&json).unwrap())
}

#[test]
fn select_call_answer() {
	let event_content = SelectCallAnswerContent::new("call_1", "some_device_address", "some_other_device_address");
	let event = MatrixEvent::new("some_event", 1577836800000, "some_room", event_content);
	assert_eq!(event.generate_event_type(), "m.call.select_answer");
	let json = serde_json::to_string_pretty(&event).unwrap();
	println!("{json}");
	assert_eq!(event, serde_json::from_str(&json).unwrap())
}

#[test]
fn call_negotiation() {
	// create offer event content
	let mut event_content_offer =
		CallNegotiationContent::offer("call_1", "some_device_address", "some_sdp_offer_string", 10000);
	// assert that answer is not set
	assert_eq!(event_content_offer.get_answer(), None);
	// swap to answer
	event_content_offer.set_answer("some_answer_sdp");
	// assert that offer fields are not set
	assert_eq!(event_content_offer.get_offer(), None);
	assert_eq!(event_content_offer.get_lifetime(), None);

	// create answer event content
	let mut event_content = CallNegotiationContent::answer("call_1", "some_device_address", "some_sdp_answer_string");
	// assert that offer fields are not set
	assert_eq!(event_content.get_offer(), None);
	assert_eq!(event_content.get_lifetime(), None);
	// swap to offer
	event_content.set_offer("some_offer_sdp", 10000);
	// assert that answer is not set
	assert_eq!(event_content.get_answer(), None);
	let event = MatrixEvent::new("some_event", 1577836800000, "some_room", event_content);
	assert_eq!(event.generate_event_type(), "m.call.negotiate");
	let json = serde_json::to_string_pretty(&event).unwrap();
	println!("{json}");
	assert_eq!(event, serde_json::from_str(&json).unwrap())
}

#[test]
fn reject_call() {
	let event_content = RejectCallContent::new("call_1", "some_device_address");
	let event = MatrixEvent::new("some_event", 1577836800000, "some_room", event_content);
	assert_eq!(event.generate_event_type(), "m.call.reject");
	let json = serde_json::to_string_pretty(&event).unwrap();
	println!("{json}");
	assert_eq!(event, serde_json::from_str(&json).unwrap())
}

#[test]
fn hangup_call() {
	let event_content = HangupCallContent::new("call_1", "some_device_address", HangupCallReason::UserHangup);
	let event = MatrixEvent::new("some_event", 1577836800000, "some_room", event_content);
	assert_eq!(event.generate_event_type(), "m.call.hangup");
	let json = serde_json::to_string_pretty(&event).unwrap();
	println!("{json}");
	assert_eq!(event, serde_json::from_str(&json).unwrap())
}
