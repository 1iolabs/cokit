// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use cid::Cid;
use co_messaging::{ephemeral_event, EventType, MatrixEvent};

#[test]
fn test_typing() {
	let event_content = ephemeral_event::TypingContent::new(vec!["did:some:user".into()]);
	let event = MatrixEvent::new("some_event", 1577836800000, "some_room", event_content);
	assert_eq!(event.generate_event_type(), "m.typing");
	let json = serde_json::to_string_pretty(&event).unwrap();
	println!("{json}");
	assert_eq!(event, serde_json::from_str(&json).unwrap());
}

#[test]
fn test_presence() {
	let event_content = ephemeral_event::PresenceContent::new(
		ephemeral_event::PresenceType::Online,
		5000,
		false,
		Cid::default(),
		"Some User",
		"Enjoying some coffee",
	);
	let event = MatrixEvent::new("some_event", 1577836800000, "some_room", event_content);
	assert_eq!(event.generate_event_type(), "m.presence");
	let json = serde_json::to_string_pretty(&event).unwrap();
	println!("{json}");
	assert_eq!(event, serde_json::from_str(&json).unwrap());
}
