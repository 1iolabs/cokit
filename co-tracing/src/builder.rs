// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

#[cfg(feature = "bunyan")]
use std::path::PathBuf;
use tracing::{
	subscriber::{set_default, set_global_default, DefaultGuard},
	Level,
};
#[cfg(feature = "bunyan")]
use tracing_bunyan_formatter::BunyanFormattingLayer;
use tracing_log::LogTracer;
use tracing_subscriber::{filter::LevelFilter, layer::SubscriberExt, EnvFilter, Layer, Registry};

/// LogTracer ignore-list default: wasmer's cranelift backend emits millions of log records per
/// module compilation.
/// Override via [`TracingBuilder::ignore_log_crate`].
const DEFAULT_LOG_IGNORES: &[&str] =
	&["cranelift_codegen", "cranelift_frontend", "cranelift_entity", "cranelift_bforest", "regalloc2"];

#[derive(Clone)]
struct SinkFilter {
	level: Level,
	directives: Option<String>,
}
impl SinkFilter {
	fn env_filter(&self) -> EnvFilter {
		env_filter(self.level, self.directives.as_deref())
	}
}

/// Build a lossy `EnvFilter` with `level` as the global default directive and optional `directives`
/// (e.g. `co_sdk=debug`) layered on top. Invalid directives are ignored with a warning, not a panic.
pub fn env_filter(level: Level, directives: Option<&str>) -> EnvFilter {
	EnvFilter::builder()
		.with_default_directive(LevelFilter::from_level(level).into())
		.parse_lossy(directives.unwrap_or_default())
}

/// Map a `-v` count (and `--quiet`) to a stderr level. `quiet` => no stderr.
/// `0` => `default` (app policy), `1` => DEBUG, `>=2` => TRACE.
pub fn level_from_verbosity(verbose: u8, quiet: bool, default: Option<Level>) -> Option<Level> {
	if quiet {
		None
	} else {
		match verbose {
			0 => default,
			1 => Some(Level::DEBUG),
			_ => Some(Level::TRACE),
		}
	}
}

/// Guard returned by [`TracingBuilder::init`]; holds (optionally) the OpenTelemetry provider so it
/// flushes/shuts down when dropped. Hold it for the program's lifetime.
#[must_use = "hold the guard for the program's lifetime so OpenTelemetry can flush"]
#[derive(Default)]
pub struct TracingGuard {
	/// Held only for its `Drop` (flushes/shuts down the OpenTelemetry provider on program exit).
	#[cfg(feature = "opentelemetry")]
	#[allow(dead_code)]
	pub(crate) open_telemetry: Option<crate::builder_open_telemetry::OpenTelemetryGuard>,
	/// Private zero-sized field to prevent external construction without `Default`.
	_priv: (),
}
impl TracingGuard {
	/// Leak the guard so the installed subscriber stays active for the rest of the process.
	///
	/// Convenience for callers that install tracing once and have nowhere to hold the guard
	/// (e.g. from a constructor or `main` that never returns) — replaces a manual
	/// `std::mem::forget`. Because this skips the guard's `Drop`, an OpenTelemetry sink will not
	/// be flushed/shut down at exit; prefer holding the guard if you rely on OpenTelemetry.
	#[allow(clippy::forget_non_drop)]
	pub fn forget(self) {
		std::mem::forget(self);
	}
}

/// A boxed per-`Registry` layer — the unit downstream callers add via
/// [`TracingBuilder::with_layer`].
pub type BoxedLayer = Box<dyn Layer<Registry> + Send + Sync>;

pub struct TracingBuilder {
	#[cfg_attr(not(feature = "bunyan"), allow(dead_code))]
	identifier: String,
	stderr: Option<SinkFilter>,
	#[cfg(feature = "bunyan")]
	file: Option<(PathBuf, SinkFilter)>,
	#[cfg(feature = "opentelemetry")]
	open_telemetry: Option<String>,
	#[cfg(all(feature = "console", target_arch = "wasm32"))]
	console: Option<SinkFilter>,
	#[cfg(all(feature = "oslog", target_vendor = "apple"))]
	oslog: Option<(String, SinkFilter)>,
	log_ignores: Vec<&'static str>,
	optional: bool,
	extra_layers: Vec<BoxedLayer>,
}
impl TracingBuilder {
	pub fn new(identifier: impl Into<String>) -> Self {
		Self {
			identifier: identifier.into(),
			stderr: None,
			#[cfg(feature = "bunyan")]
			file: None,
			#[cfg(feature = "opentelemetry")]
			open_telemetry: None,
			#[cfg(all(feature = "console", target_arch = "wasm32"))]
			console: None,
			#[cfg(all(feature = "oslog", target_vendor = "apple"))]
			oslog: None,
			log_ignores: DEFAULT_LOG_IGNORES.to_vec(),
			optional: false,
			extra_layers: Vec::new(),
		}
	}

	/// Add a layer to the subscriber (repeatable).
	pub fn with_layer(mut self, layer: BoxedLayer) -> Self {
		self.extra_layers.push(layer);
		self
	}

	pub fn with_stderr(mut self, level: Level, directives: Option<&str>) -> Self {
		self.stderr = Some(SinkFilter { level, directives: directives.map(str::to_owned) });
		self
	}

	#[cfg(feature = "bunyan")]
	pub fn with_file(mut self, path: PathBuf, level: Level, directives: Option<&str>) -> Self {
		self.file = Some((path, SinkFilter { level, directives: directives.map(str::to_owned) }));
		self
	}

	/// Convenience constructor: a builder logging (bunyan) to `path` at INFO when `Some`. A drop-in
	/// for the old `ApplicationBuilder::with_bunyan_logging` — pass `application_builder.log_path()`.
	/// Chain `.with_optional()` / `.init()` (and other `with_*` sinks) as usual.
	#[cfg(feature = "bunyan")]
	pub fn file(identifier: impl Into<String>, path: Option<PathBuf>) -> Self {
		let builder = Self::new(identifier);
		match path {
			Some(path) => builder.with_file(path, Level::INFO, None),
			None => builder,
		}
	}

	pub fn with_optional(mut self) -> Self {
		self.optional = true;
		self
	}

	#[cfg(feature = "opentelemetry")]
	pub fn with_open_telemetry(mut self, endpoint: impl Into<String>) -> Self {
		self.open_telemetry = Some(endpoint.into());
		self
	}

	pub fn ignore_log_crate(mut self, name: &'static str) -> Self {
		self.log_ignores.push(name);
		self
	}

	// Non-OpenTelemetry stub — used when the `opentelemetry` feature is disabled.
	#[cfg(not(feature = "opentelemetry"))]
	fn build_open_telemetry(&self) -> Result<(Option<tracing_subscriber::layer::Identity>, ()), anyhow::Error> {
		Ok((None, ()))
	}

	#[cfg(feature = "opentelemetry")]
	fn build_open_telemetry(
		&self,
	) -> Result<
		(
			Option<tracing_opentelemetry::OpenTelemetryLayer<Registry, opentelemetry_sdk::trace::Tracer>>,
			Option<crate::builder_open_telemetry::OpenTelemetryGuard>,
		),
		anyhow::Error,
	> {
		match &self.open_telemetry {
			Some(endpoint) => {
				let (layer, guard) = crate::builder_open_telemetry::layer(&self.identifier, endpoint)?;
				Ok((Some(layer), Some(guard)))
			},
			None => Ok((None, None)),
		}
	}

	/// Log to the browser console (wasm). `level` + optional `directives` build a per-layer filter.
	#[cfg(all(feature = "console", target_arch = "wasm32"))]
	pub fn with_console(mut self, level: Level, directives: Option<&str>) -> Self {
		self.console = Some(SinkFilter { level, directives: directives.map(str::to_owned) });
		self
	}

	/// Log to Apple unified logging (`os_log`) under `subsystem`, with optional per-target directives.
	#[cfg(all(feature = "oslog", target_vendor = "apple"))]
	pub fn with_oslog(mut self, subsystem: impl Into<String>, level: Level, directives: Option<&str>) -> Self {
		self.oslog = Some((subsystem.into(), SinkFilter { level, directives: directives.map(str::to_owned) }));
		self
	}

	/// Browser-console layer (wasm), filtered per-sink. Replaces the native stub.
	#[cfg(all(feature = "console", target_arch = "wasm32"))]
	fn build_console(&self) -> Option<impl Layer<Registry> + Send + Sync> {
		self.console.as_ref().map(|s| {
			tracing_subscriber::fmt::layer()
				.with_writer(tracing_web::MakeWebConsoleWriter::new())
				.without_time()
				.with_filter(s.env_filter())
		})
	}

	// Native (non-wasm or no-console) platform: no browser-console sink.
	#[cfg(not(all(feature = "console", target_arch = "wasm32")))]
	fn build_console(&self) -> Option<tracing_subscriber::layer::Identity> {
		None
	}

	fn build_subscriber(
		&mut self,
	) -> Result<Option<(impl tracing::Subscriber + Send + Sync + 'static, TracingGuard)>, anyhow::Error> {
		let mut layers: Vec<Box<dyn Layer<Registry> + Send + Sync>> = Vec::new();

		// browser console (wasm); native stub returns None
		if let Some(layer) = self.build_console() {
			layers.push(layer.boxed());
		}

		// open telemetry; non-OpenTelemetry build returns (None, ())
		let (open_telemetry_layer, _open_telemetry_guard) = self.build_open_telemetry()?;
		if let Some(layer) = open_telemetry_layer {
			layers.push(layer.boxed());
		}

		// stderr
		if let Some(sink) = &self.stderr {
			layers.push(
				tracing_subscriber::fmt::layer()
					.with_writer(std::io::stderr)
					.with_filter(sink.env_filter())
					.boxed(),
			);
		}

		// bunyan file
		#[cfg(feature = "bunyan")]
		if let Some((path, sink)) = &self.file {
			std::fs::create_dir_all(path.parent().ok_or_else(|| anyhow::anyhow!("no parent"))?)?;
			let log_file = std::fs::File::options().append(true).create(true).open(path)?;
			layers.push(
				BunyanFormattingLayer::new(self.identifier.clone(), log_file)
					.serialize_span_id(true)
					.serialize_span_type(true)
					.serialize_span_fields(false)
					.with_filter(sink.env_filter())
					.boxed(),
			);
		}

		// Apple unified logging (oslog)
		#[cfg(all(feature = "oslog", target_vendor = "apple"))]
		if let Some((subsystem, sink)) = &self.oslog {
			layers.push(
				tracing_oslog::OsLogger::new(subsystem, "default")
					.with_filter(sink.env_filter())
					.boxed(),
			);
		}

		// extra layers
		layers.extend(std::mem::take(&mut self.extra_layers));
		if layers.is_empty() {
			return Ok(None);
		}
		let subscriber = Registry::default().with(layers);
		let guard = TracingGuard {
			#[cfg(feature = "opentelemetry")]
			open_telemetry: _open_telemetry_guard,
			_priv: (),
		};
		Ok(Some((subscriber, guard)))
	}

	#[must_use = "hold the guard for the program's lifetime so OpenTelemetry can flush"]
	pub fn init(mut self) -> Result<TracingGuard, anyhow::Error> {
		let optional = self.optional;
		let ignores = self.log_ignores.clone();
		match self.build_subscriber()? {
			Some((subscriber, guard)) => match set_global_default(subscriber) {
				Ok(()) => {
					init_log_tracer(&ignores);
					Ok(guard)
				},
				Err(err) if optional => {
					tracing::warn!(?err, "tracing-already-initialized");
					Ok(TracingGuard::default())
				},
				Err(err) => Err(err.into()),
			},
			None => Ok(TracingGuard::default()),
		}
	}

	#[must_use = "hold the returned guards for as long as the scoped subscriber should be active"]
	pub fn init_scope(mut self) -> Result<Option<(DefaultGuard, TracingGuard)>, anyhow::Error> {
		let ignores = self.log_ignores.clone();
		match self.build_subscriber()? {
			Some((subscriber, guard)) => {
				let scope = set_default(subscriber);
				init_log_tracer(&ignores);
				Ok(Some((scope, guard)))
			},
			None => Ok(None),
		}
	}
}

fn init_log_tracer(ignores: &[&'static str]) {
	let mut builder = LogTracer::builder();
	for name in ignores {
		builder = builder.ignore_crate(*name);
	}
	// May already be installed (e.g. repeated scoped inits in tests); that's fine.
	if let Err(err) = builder.init() {
		tracing::debug!(?err, "log-tracer-already-initialized");
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn env_filter_is_lossy() {
		let _ = env_filter(Level::INFO, None);
		let _ = env_filter(Level::INFO, Some("co_sdk=debug"));
		let _ = env_filter(Level::INFO, Some("@@@not-valid@@@")); // must not panic
	}

	#[test]
	fn verbosity_mapping() {
		assert_eq!(level_from_verbosity(0, false, Some(Level::INFO)), Some(Level::INFO));
		assert_eq!(level_from_verbosity(0, false, None), None);
		assert_eq!(level_from_verbosity(1, false, None), Some(Level::DEBUG));
		assert_eq!(level_from_verbosity(5, false, Some(Level::INFO)), Some(Level::TRACE));
		assert_eq!(level_from_verbosity(3, true, Some(Level::INFO)), None);
	}

	#[test]
	fn init_scope_builds_a_stderr_subscriber() {
		let guard = TracingBuilder::new("test").with_stderr(Level::INFO, None).init_scope().unwrap();
		assert!(guard.is_some());
	}

	#[test]
	fn empty_builder_installs_nothing() {
		let guard = TracingBuilder::new("test").init_scope().unwrap();
		assert!(guard.is_none());
	}
}
