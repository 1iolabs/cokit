#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
	Message(Vec<u8>),
}
impl From<Vec<u8>> for Message {
	fn from(value: Vec<u8>) -> Self {
		Message::Message(value)
	}
}
impl From<String> for Message {
	fn from(value: String) -> Self {
		Message::Message(value.into())
	}
}
impl From<&str> for Message {
	fn from(value: &str) -> Self {
		Message::Message(value.into())
	}
}
impl Message {
	/// Get message as JSON string. Returning None if not JSON.
	///
	/// Note: No verification will be done.
	pub fn json(&self) -> Option<&str> {
		match self {
			Message::Message(data) =>
				if !data.is_empty() && data[0] == '{' as u8 {
					match std::str::from_utf8(&data) {
						Ok(str) => Some(str),
						Err(_) => None,
					}
				} else {
					None
				},
		}
	}
}
