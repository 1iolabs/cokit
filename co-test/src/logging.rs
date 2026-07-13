// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use crate::test_log_path;
use co_tracing::{parse_log, LogConfig, LogSink, TracingBuilder};
use std::sync::Once;
use tracing::{subscriber::DefaultGuard, Level};

/// Filter directives for the default verbose file log (quiets the chatty deps).
const DEFAULT_DIRECTIVES: &str = "trace,log=warn,quinn_proto=warn,hickory_proto=warn,co_storage::storage::memory=warn";

static GLOBAL: Once = Once::new();

/// Build a [`TracingBuilder`] from a `CO_LOG`-grammar `spec`. Returns `None` when the spec disables
/// logging (`off`). The `file` sink resolves to [`test_log_path`]; `stderr` defaults to INFO; the
/// default (`on`/empty) is a verbose file log.
fn builder_from_spec(spec: &str) -> Option<TracingBuilder> {
	let mut builder = TracingBuilder::new("co-test");
	match parse_log(spec).unwrap_or(LogConfig::Default) {
		LogConfig::Off => return None,
		LogConfig::Default => {
			builder = builder.with_file(test_log_path(), Level::TRACE, Some(DEFAULT_DIRECTIVES));
		},
		LogConfig::Sinks(sinks) => {
			for entry in sinks {
				builder = match entry.sink {
					LogSink::Stderr => builder.with_stderr(Level::INFO, entry.filter.as_deref()),
					LogSink::File(None) => builder.with_file(test_log_path(), Level::TRACE, entry.filter.as_deref()),
					LogSink::File(Some(path)) => builder.with_file(path, Level::TRACE, entry.filter.as_deref()),
					LogSink::Oslog => builder, // not used in tests
				};
			}
		},
	}
	Some(builder.with_optional())
}

/// Install **global** test logging, once per test binary (idempotent — the first call wins).
///
/// Defaults to a verbose bunyan log at [`test_log_path`]. Override for the whole run with the
/// `CO_LOG` env var (co-tracing grammar), e.g. `CO_LOG=off` or `CO_LOG=stderr:info`. Being global,
/// it captures logs from spawned threads/tasks too — use it for network / multi-thread tests.
///
/// Call it at the top of a test; omit it to run that test without logging.
pub fn init_test_log() {
	GLOBAL.call_once(|| {
		let env = std::env::var("CO_LOG").unwrap_or_default();
		let spec = if env.is_empty() { "on" } else { env.as_str() };
		if let Some(builder) = builder_from_spec(spec) {
			if let Ok(guard) = builder.init() {
				// Keep the subscriber installed for the rest of the process.
				guard.forget();
			}
		}
	});
}

/// Install **scoped** test logging from an explicit `CO_LOG`-grammar `spec` (e.g. `"stderr:info"`,
/// `"off"`, `"file"`). Returns a guard that restores the previous subscriber when dropped — bind it
/// for the test's duration (`let _log = co_test::init_test_log_spec("stderr:info");`).
///
/// Scoped (thread-local): captures the test thread — and, on a current-thread tokio runtime, its
/// tasks — but not logs from separately-spawned threads. For full cross-thread capture use
/// [`init_test_log`]. Returns `None` when the spec is `off`.
#[must_use]
pub fn init_test_log_spec(spec: &str) -> Option<DefaultGuard> {
	let builder = builder_from_spec(spec)?;
	let (default_guard, tracing_guard) = builder.init_scope().ok()??;
	// No OpenTelemetry in tests, so the TracingGuard is inert; the DefaultGuard is the scope.
	let _ = tracing_guard;
	Some(default_guard)
}
