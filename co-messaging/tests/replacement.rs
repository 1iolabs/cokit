// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use co_messaging::{
	message_event::{MessageType, TextContent},
	EventContent, MatrixEvent,
};

#[test]
fn test_replace_text_content() {
	let original_content = TextContent::new("Some message");
	let original_event = MatrixEvent::new("some_event", 1577836800000, "@some.room", original_content);

	let mut replacement_event_content = (match original_event.content {
		EventContent::Message(MessageType::Text(c)) => Ok(c),
		_ => Err(""),
	})
	.unwrap();
	replacement_event_content.new_content = Some(Box::new(TextContent::new("Some new fancy body").into()));
	let event = MatrixEvent::new("some_event", 1577836805000, "@some.room", replacement_event_content);

	let json = serde_json::to_string_pretty(&event).unwrap();
	println!("JSON: {}", json);
	let serded_event: MatrixEvent = serde_json::from_str(&json).unwrap();
	assert_eq!(event, serded_event);
}
