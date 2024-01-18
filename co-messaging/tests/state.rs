use co_messaging::{state_event, MatrixEvent};

#[test]
fn room_name() {
	let content = state_event::RoomNameContent::new("Some name");
	let event = MatrixEvent::new("event1234", 5000, "$some:room", "@some:did", content);
	let json = serde_json::to_string_pretty(&event).unwrap();
	println!("{json}");
	assert_eq!(event, serde_json::from_str(&json).unwrap());
}
