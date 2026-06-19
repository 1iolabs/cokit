// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

#[cfg(not(target_arch = "wasm32"))]
use crate::{level_from_verbosity, resolve_filter, LogSink, SinkSpec};
use crate::{LogConfig, TracingBuilder, TracingGuard};
use std::path::{Path, PathBuf};
use tracing::Level;

/// Reusable logging CLI surface. Flatten into a clap `Parser` with `#[command(flatten)]`.
#[derive(Debug, Default, Clone, clap::Args)]
pub struct LogArgs {
	/// Logging config: `sink[:filter]` entries separated by `;`, or `off`/`on`.
	/// Examples: `file`, `stderr:error`, `file:info,co_sdk=trace;stderr:error`. Quote it in your shell.
	/// Env: CO_LOG
	#[arg(long, env = "CO_LOG", value_parser = crate::parse_log, allow_hyphen_values = true)]
	pub log: Option<LogConfig>,

	/// Increase stderr verbosity (`-v` debug, `-vv` trace). Ignored if `CO_LOG` names a stderr sink.
	#[arg(short, long, action = clap::ArgAction::Count)]
	pub verbose: u8,

	/// Silence stderr.
	#[arg(short, long)]
	pub quiet: bool,

	/// Disable the file sink (legacy; applies only when `CO_LOG` is unset).
	#[arg(long)]
	pub no_log: bool,

	/// Log file path (legacy; applies only when `CO_LOG` is unset).
	#[arg(long)]
	pub log_path: Option<PathBuf>,

	/// Enable OpenTelemetry export.
	#[cfg(feature = "opentelemetry")]
	#[arg(long)]
	pub open_telemetry: bool,

	/// OpenTelemetry endpoint (`stdout` for the stdout exporter).
	#[cfg(feature = "opentelemetry")]
	#[arg(long, default_value = "http://localhost:4317")]
	pub open_telemetry_endpoint: String,
}
impl LogArgs {
	/// Build `LogArgs` from a single `CO_LOG`-grammar string (e.g. `"file:info"`, `"stderr:debug"`,
	/// `"off"`), leaving the other fields (`-v`/`-q`, OpenTelemetry, …) at their defaults.
	pub fn parse(spec: &str) -> Result<Self, anyhow::Error> {
		Ok(Self { log: Some(crate::parse_log(spec).map_err(anyhow::Error::msg)?), ..Default::default() })
	}

	/// Default sink set when `CO_LOG` is unset/`on` (native): on iOS, the oslog sink; on other
	/// native targets, the legacy file sink (honoring `--no-log`/`--log-path`). Stderr is added
	/// by the verbosity shorthand in `resolve_sinks`.
	#[cfg(not(target_arch = "wasm32"))]
	fn default_sinks(&self, context: &LogContext) -> Vec<SinkSpec> {
		#[cfg(target_os = "ios")]
		{
			let _ = context;
			return vec![SinkSpec { sink: LogSink::Oslog, filter: None }];
		}
		#[cfg(not(target_os = "ios"))]
		{
			if self.no_log {
				return Vec::new();
			}
			let path = self
				.log_path
				.clone()
				.or_else(|| context.base_path.map(|b| b.join("log/co.log")));
			match path {
				Some(path) => vec![SinkSpec { sink: LogSink::File(Some(path)), filter: None }],
				None => Vec::new(),
			}
		}
	}

	/// Resolve the effective native sink set: from `CO_LOG` (or the per-app default), plus the
	/// `-v`/`-q` stderr shorthand when `CO_LOG` doesn't name a stderr sink. Each sink's filter is
	/// resolved against `RUST_LOG` (inline wins; bare sink ⇒ `RUST_LOG` ⇒ builder default `info`).
	#[cfg(not(target_arch = "wasm32"))]
	fn resolve_sinks(&self, context: &LogContext, rust_log: Option<&str>) -> Vec<SinkSpec> {
		let mut sinks = match &self.log {
			Some(LogConfig::Off) => return Vec::new(),
			Some(LogConfig::Sinks(s)) => s.clone(),
			Some(LogConfig::Default) | None => self.default_sinks(context),
		};
		// An explicit `CO_LOG` sink list is authoritative: suppress the default stderr baseline
		// (as if `-q` for the default). `-v` can still add stderr; a `stderr:` entry is honored above.
		let stderr_default = if matches!(self.log, Some(LogConfig::Sinks(_))) { None } else { context.default_stderr };
		if !sinks.iter().any(|s| matches!(s.sink, LogSink::Stderr)) {
			if let Some(level) = level_from_verbosity(self.verbose, self.quiet, stderr_default) {
				sinks.push(SinkSpec { sink: LogSink::Stderr, filter: Some(level_directive(level).to_owned()) });
			}
		}
		for spec in &mut sinks {
			spec.filter = resolve_filter(spec.filter.as_deref(), rust_log);
		}
		sinks
	}

	/// Compose a [`TracingBuilder`]. Target-aware: native = stderr/file/otel; wasm = browser console.
	pub fn tracing_builder(&self, context: &LogContext) -> TracingBuilder {
		#[allow(unused_mut)]
		let mut builder = TracingBuilder::new(context.identifier);

		#[cfg(all(target_arch = "wasm32", feature = "console"))]
		{
			if !matches!(self.log, Some(LogConfig::Off)) {
				let directives = match &self.log {
					Some(LogConfig::Sinks(s)) => s.iter().find_map(|sp| sp.filter.clone()),
					_ => None,
				};
				builder = builder.with_console(Level::INFO, directives.as_deref());
			}
		}

		#[cfg(not(target_arch = "wasm32"))]
		{
			let rust_log = std::env::var("RUST_LOG").ok();
			for spec in self.resolve_sinks(context, rust_log.as_deref()) {
				match spec.sink {
					LogSink::Stderr => builder = builder.with_stderr(Level::INFO, spec.filter.as_deref()),
					#[cfg(feature = "bunyan")]
					LogSink::File(path) => {
						let path = path.or_else(|| context.base_path.map(|b| b.join("log/co.log")));
						if let Some(path) = path {
							builder = builder.with_file(path, Level::INFO, spec.filter.as_deref());
						}
					},
					#[cfg(not(feature = "bunyan"))]
					LogSink::File(_) => {},
					#[cfg(all(feature = "oslog", target_vendor = "apple"))]
					LogSink::Oslog => {
						let subsystem = context.oslog_subsystem.unwrap_or(context.identifier);
						builder = builder.with_oslog(subsystem, Level::INFO, spec.filter.as_deref());
					},
					#[cfg(not(all(feature = "oslog", target_vendor = "apple")))]
					LogSink::Oslog => {},
				}
			}
			#[cfg(feature = "opentelemetry")]
			if self.open_telemetry {
				builder = builder.with_open_telemetry(self.open_telemetry_endpoint.clone());
			}
		}

		builder
	}

	#[must_use = "hold the guard for the program's lifetime"]
	pub fn init(&self, context: &LogContext) -> Result<TracingGuard, anyhow::Error> {
		self.tracing_builder(context).init()
	}
}

/// App-supplied context/defaults — the per-app configurability knob.
///
/// Construct with [`LogContext::new`] and the `with_*` setters; the struct is `#[non_exhaustive]`
/// so new context fields can be added without breaking callers.
#[non_exhaustive]
pub struct LogContext<'a> {
	pub identifier: &'a str,
	pub base_path: Option<&'a Path>,
	/// stderr level when `CO_LOG` is silent on stderr and no `-v`/`-q`. `None` ⇒ no stderr by default.
	pub default_stderr: Option<Level>,
	/// `os_log` subsystem (reverse-DNS) for an `oslog` sink; falls back to `identifier` when `None`.
	pub oslog_subsystem: Option<&'a str>,
}
impl<'a> LogContext<'a> {
	/// A context for an app identified by `identifier` (the otel service name and the `oslog`
	/// subsystem fallback). Every other field defaults to `None`; set them with the `with_*` setters.
	pub fn new(identifier: &'a str) -> Self {
		Self { identifier, base_path: None, default_stderr: None, oslog_subsystem: None }
	}

	/// Base path for the default file sink (the legacy `<base>/log/co.log`).
	#[must_use]
	pub fn with_base_path(mut self, base_path: Option<&'a Path>) -> Self {
		self.base_path = base_path;
		self
	}

	/// stderr level when `CO_LOG` is silent on stderr and no `-v`/`-q` is given.
	#[must_use]
	pub fn with_default_stderr(mut self, level: Option<Level>) -> Self {
		self.default_stderr = level;
		self
	}

	/// `os_log` subsystem (reverse-DNS) for an `oslog` sink; falls back to `identifier` when `None`.
	#[must_use]
	pub fn with_oslog_subsystem(mut self, subsystem: Option<&'a str>) -> Self {
		self.oslog_subsystem = subsystem;
		self
	}
}

#[cfg(not(target_arch = "wasm32"))]
fn level_directive(level: Level) -> &'static str {
	match level {
		Level::ERROR => "error",
		Level::WARN => "warn",
		Level::INFO => "info",
		Level::DEBUG => "debug",
		Level::TRACE => "trace",
	}
}

#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod tests {
	use super::*;

	fn context() -> LogContext<'static> {
		LogContext {
			identifier: "test",
			base_path: Some(Path::new("/base")),
			default_stderr: Some(Level::INFO),
			oslog_subsystem: None,
		}
	}

	#[test]
	fn off_yields_no_sinks() {
		let a = LogArgs { log: Some(LogConfig::Off), ..Default::default() };
		assert!(a.resolve_sinks(&context(), None).is_empty());
	}

	#[test]
	fn explicit_sinks_with_inline_filters() {
		let a = LogArgs {
			log: Some(LogConfig::Sinks(vec![
				SinkSpec { sink: LogSink::File(None), filter: Some("info,co_sdk=trace".into()) },
				SinkSpec { sink: LogSink::Stderr, filter: Some("error".into()) },
			])),
			..Default::default()
		};
		let sinks = a.resolve_sinks(&context(), None);
		assert_eq!(sinks.len(), 2);
		assert_eq!(sinks[0].filter.as_deref(), Some("info,co_sdk=trace"));
		assert_eq!(sinks[1].filter.as_deref(), Some("error"));
	}

	#[test]
	fn stderr_shorthand_added_when_not_named() {
		let a = LogArgs {
			log: Some(LogConfig::Sinks(vec![SinkSpec { sink: LogSink::File(None), filter: None }])),
			verbose: 1,
			..Default::default()
		};
		let sinks = a.resolve_sinks(&context(), None);
		assert_eq!(sinks.len(), 2);
		assert!(matches!(sinks[1].sink, LogSink::Stderr));
		assert_eq!(sinks[1].filter.as_deref(), Some("debug"));
	}

	#[test]
	fn co_log_stderr_entry_suppresses_shorthand() {
		let a = LogArgs {
			log: Some(LogConfig::Sinks(vec![SinkSpec { sink: LogSink::Stderr, filter: Some("error".into()) }])),
			verbose: 2,
			..Default::default()
		};
		let sinks = a.resolve_sinks(&context(), None);
		assert_eq!(sinks.len(), 1);
		assert_eq!(sinks[0].filter.as_deref(), Some("error"));
	}

	#[test]
	fn default_sink_set_when_unset() {
		let a = LogArgs::default();
		let sinks = a.resolve_sinks(&context(), None);
		assert!(sinks
			.iter()
			.any(|s| matches!(&s.sink, LogSink::File(Some(p)) if p == Path::new("/base/log/co.log"))));
		assert!(sinks
			.iter()
			.any(|s| matches!(s.sink, LogSink::Stderr) && s.filter.as_deref() == Some("info")));
	}

	#[test]
	fn bare_sink_uses_rust_log_and_quiet_suppresses_stderr() {
		let a = LogArgs {
			log: Some(LogConfig::Sinks(vec![SinkSpec { sink: LogSink::File(None), filter: None }])),
			quiet: true,
			..Default::default()
		};
		let sinks = a.resolve_sinks(&context(), Some("co_sdk=trace"));
		assert_eq!(sinks.len(), 1);
		assert_eq!(sinks[0].filter.as_deref(), Some("co_sdk=trace"));
	}

	#[test]
	fn explicit_co_log_suppresses_default_stderr() {
		// CO_LOG names only a file, no -v ⇒ default stderr is suppressed (just the file).
		let a = LogArgs {
			log: Some(LogConfig::Sinks(vec![SinkSpec { sink: LogSink::File(None), filter: None }])),
			..Default::default()
		};
		let sinks = a.resolve_sinks(&context(), None);
		assert_eq!(sinks.len(), 1);
		assert!(matches!(sinks[0].sink, LogSink::File(None)));
	}

	#[test]
	fn explicit_co_log_plus_verbose_still_adds_stderr() {
		let a = LogArgs {
			log: Some(LogConfig::Sinks(vec![SinkSpec { sink: LogSink::File(None), filter: None }])),
			verbose: 1,
			..Default::default()
		};
		let sinks = a.resolve_sinks(&context(), None);
		assert_eq!(sinks.len(), 2);
		assert!(matches!(sinks[1].sink, LogSink::Stderr));
		assert_eq!(sinks[1].filter.as_deref(), Some("debug"));
	}

	#[test]
	fn cli_accepts_hyphen_leading_log_value() {
		use clap::Parser;
		#[derive(Parser)]
		struct Harness {
			#[command(flatten)]
			log: LogArgs,
		}
		// `--log -:trace` → stderr sink with inline filter "trace"
		let h = Harness::try_parse_from(["x", "--log", "-:trace"]).unwrap();
		assert_eq!(
			h.log.log,
			Some(LogConfig::Sinks(vec![SinkSpec { sink: LogSink::Stderr, filter: Some("trace".into()) }]))
		);
		// bare `--log -` → stderr, no filter
		let h2 = Harness::try_parse_from(["x", "--log", "-"]).unwrap();
		assert_eq!(h2.log.log, Some(LogConfig::Sinks(vec![SinkSpec { sink: LogSink::Stderr, filter: None }])));
	}

	#[test]
	fn oslog_entry_is_a_sink() {
		let a = LogArgs {
			log: Some(LogConfig::Sinks(vec![SinkSpec { sink: LogSink::Oslog, filter: Some("debug".into()) }])),
			..Default::default()
		};
		let sinks = a.resolve_sinks(&context(), None);
		assert_eq!(sinks.len(), 1);
		assert!(matches!(sinks[0].sink, LogSink::Oslog));
		assert_eq!(sinks[0].filter.as_deref(), Some("debug"));
	}
}
