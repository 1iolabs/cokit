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
