use crate::CoErrorSignal;
use dioxus::hooks::use_signal_sync;

/// Create custom error signal.
pub fn use_co_error_signal() -> CoErrorSignal {
	use_signal_sync(|| Vec::new())
}
