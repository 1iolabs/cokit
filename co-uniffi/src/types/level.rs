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
