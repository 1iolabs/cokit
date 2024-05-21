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
	/// Get message as JSON string. Returning None if not JSON Object.
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

	/// Get message as CBOR. Returning None if not CBOR Map.
	///
	/// Note: No verification will be done.
	pub fn cbor(&self) -> Option<&[u8]> {
		match self {
			Message::Message(data) =>
				if !data.is_empty() && data[0] == 5 {
					Some(data)
				} else {
					None
				},
		}
	}
}

#[cfg(test)]
mod tests {
	use super::Message;
	use serde::Serialize;

	#[derive(Debug, Serialize)]
	struct Test {
		count: i32,
	}

	#[test]
	fn test_json() {
		let data = Test { count: 10 };
		let json = serde_json::to_string(&data).unwrap();
		let message: Message = json.clone().into();
		assert_eq!(Some(json.as_str()), message.json());
	}

	#[test]
	fn test_cbor() {
		let data = Test { count: 10 };
		let cbor = serde_ipld_dagcbor::to_vec(&data).unwrap();
		let message: Message = cbor.clone().into();
		// println!("f: {}", cbor[0] as u8);
		assert_eq!(Some(&cbor[..]), message.cbor());
	}
}
