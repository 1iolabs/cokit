#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
	Message(Vec<u8>),
}
