use co_messaging::{ephemeral_event, EventType, MatrixEvent};
use libipld::Cid;

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
