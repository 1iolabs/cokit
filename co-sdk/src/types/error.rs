// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ErrorContext {
	/// Whether the error is fatal and application has to be restarted.
	pub kind: ErrorKind,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub enum ErrorKind {
	/// Error is a informational warning. Application continues to work properly.
	#[default]
	Warning,

	/// Error is fatal. Application has to be restarted.
	Fatal,
}

impl From<ErrorKind> for ErrorContext {
	fn from(val: ErrorKind) -> Self {
		ErrorContext { kind: val }
	}
}

pub trait IntoAction<T> {
	fn into_action<C: Into<ErrorContext>>(self, context: C) -> T;
}
