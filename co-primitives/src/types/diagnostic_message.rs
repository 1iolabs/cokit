// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiagnosticMessage {
	// Trace(),
	// Info(),
	// Warning(),
	// Error(),
	Failure(String),
}
impl DiagnosticMessage {
	pub fn to_error(&self) -> Option<anyhow::Error> {
		match self {
			DiagnosticMessage::Failure(diagnostic) => Some(anyhow::anyhow!(diagnostic.clone())),
		}
	}
}
impl From<anyhow::Error> for DiagnosticMessage {
	fn from(value: anyhow::Error) -> Self {
		DiagnosticMessage::Failure(format!("{:?}", value))
	}
}
