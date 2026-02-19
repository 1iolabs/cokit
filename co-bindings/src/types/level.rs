// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use tracing::Level;

/// Binding for [`Level`].
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
#[derive(Debug, Default, Clone)]
pub enum CoLogLevel {
	Error,
	Warn,
	#[default]
	Info,
	Debug,
	Trace,
}
impl From<CoLogLevel> for Level {
	fn from(value: CoLogLevel) -> Self {
		match value {
			CoLogLevel::Error => Level::ERROR,
			CoLogLevel::Warn => Level::WARN,
			CoLogLevel::Info => Level::INFO,
			CoLogLevel::Debug => Level::DEBUG,
			CoLogLevel::Trace => Level::TRACE,
		}
	}
}
