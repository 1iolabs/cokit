use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiagnosticMessage {
	// Trace(),
	// Info(),
	// Warning(),
	// Error(),
	Failure(String),
}
impl From<anyhow::Error> for DiagnosticMessage {
	fn from(value: anyhow::Error) -> Self {
		DiagnosticMessage::Failure(format!("{:?}", value))
	}
}
