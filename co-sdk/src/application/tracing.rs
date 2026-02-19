// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary COkit code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use opentelemetry::{trace::TraceError, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{runtime, trace as sdktrace, Resource};
use opentelemetry_semantic_conventions::resource::SERVICE_NAME;
use std::path::PathBuf;
use tracing::{
	subscriber::{set_default, set_global_default, DefaultGuard},
	Level,
};
use tracing_bunyan_formatter::BunyanFormattingLayer;
use tracing_log::LogTracer;
use tracing_subscriber::{fmt::writer::MakeWriterExt, layer::SubscriberExt, EnvFilter, Registry};

pub struct TracingBuilder {
	identifier: String,
	base_path: Option<PathBuf>,
	bunyan: Option<PathBuf>,
	log_max_level: Level,
	stderr: bool,
	env_filter: Option<EnvFilter>,
	/// Trace to open telemetry endpoint.
	open_telemetry: Option<String>,
	/// If true do not fail if a other tracing is already registered.
	optional: bool,
}
impl TracingBuilder {
	pub fn new(identifier: String, base_path: Option<PathBuf>) -> Self {
		Self {
			identifier,
			base_path,
			bunyan: None,
			log_max_level: Level::TRACE,
			stderr: false,
			open_telemetry: None,
			env_filter: None,
			optional: false,
		}
	}

	/// Enable bunyan logging to log_path.
	/// If no path is specified {path}/log/application.log is used.
	/// Command read without network stuff:
	/// ```sh
	/// tail -0f ~/Application\ Support/co.app/log/application.log | bunyan -c '!/^(libp2p|hickory_proto)/.test(this.target)'
	/// ```
	pub fn with_bunyan_logging(self, log_path: Option<PathBuf>) -> Self {
		let bunyan = match (log_path, &self.base_path) {
			(Some(p), _) => Some(p),
			//None => self.path.join("log").join(format!("{}.log", &self.identifier)),
			(None, Some(base_path)) => Some(base_path.join("log").join("co.log")),
			_ => None,
		};
		Self { bunyan, ..self }
	}

	pub fn with_max_level(self, log_max_level: Level) -> Self {
		Self { log_max_level, ..self }
	}

	pub fn with_open_telemetry(self, endpoint: impl Into<String>) -> Self {
		Self { open_telemetry: Some(endpoint.into()), ..self }
	}

	pub fn with_stderr_logging(self) -> Self {
		Self { stderr: true, ..self }
	}

	pub fn with_optional_tracing(self) -> Self {
		Self { optional: true, ..self }
	}

	pub fn with_env_filter(self) -> Self {
		Self { env_filter: Some(EnvFilter::from_default_env()), ..self }
	}

	pub fn with_env_filter_env(self, env: String) -> Self {
		Self { env_filter: Some(EnvFilter::from_env(env)), ..self }
	}

	pub fn with_env_filter_directives(self, directives: &str) -> Result<Self, anyhow::Error> {
		Ok(Self { env_filter: Some(EnvFilter::try_new(directives)?), ..self })
	}

	fn build_subscriber(self) -> Result<Option<impl tracing::Subscriber + Send + Sync + 'static>, anyhow::Error> {
		// open telemetry
		let open_telemetry = if let Some(endpoint) = &self.open_telemetry {
			Some(open_telemetry_endpoint(self.identifier.clone(), endpoint.clone())?)
		} else {
			None
		};

		// bunyan
		let bunyan = if let Some(log_path) = &self.bunyan {
			std::fs::create_dir_all(log_path.parent().ok_or(anyhow::anyhow!("no parent"))?)?;
			let log_file = std::fs::File::options().append(true).create(true).open(log_path)?;
			Some(
				BunyanFormattingLayer::new(self.identifier.clone(), log_file.with_max_level(self.log_max_level))
					.serialize_span_id(true)
					.serialize_span_type(true)
					.serialize_span_fields(false),
			)
		} else {
			None
		};

		// stderr
		let stderr = if self.stderr {
			Some(tracing_subscriber::fmt::layer().with_writer(std::io::stderr.with_max_level(self.log_max_level)))
		} else {
			None
		};

		if open_telemetry.is_some() || bunyan.is_some() || stderr.is_some() {
			Ok(Some(
				Registry::default()
					.with(open_telemetry)
					.with(self.env_filter)
					.with(bunyan)
					.with(stderr),
			))
		} else {
			Ok(None)
		}
	}

	pub fn init(self) -> Result<(), anyhow::Error> {
		// init
		let optional = self.optional;
		if let Some(subscriber) = self.build_subscriber()? {
			let result = set_global_default(subscriber);
			match result {
				Ok(_) => {
					LogTracer::init()?;
					Ok(())
				},
				Err(err) if optional => {
					tracing::warn!(?err, "tracing-already-initialized");
					Ok(())
				},
				Err(err) => Err(err),
			}?;
		}

		// result
		Ok(())
	}

	pub fn init_scope(self) -> Result<Option<DefaultGuard>, anyhow::Error> {
		// init
		if let Some(subscriber) = self.build_subscriber()? {
			let result = set_default(subscriber);
			LogTracer::init()?;
			Ok(Some(result))
		} else {
			// result
			Ok(None)
		}
	}
}

fn open_telemetry_endpoint(
	service_name: impl Into<String>,
	endpoint: impl Into<String>,
) -> Result<tracing_opentelemetry::OpenTelemetryLayer<tracing_subscriber::Registry, sdktrace::Tracer>, anyhow::Error> {
	Ok(tracing_opentelemetry::layer().with_tracer(init_tracer(service_name, endpoint)?))
}

/// See:
/// - https://github.com/open-telemetry/opentelemetry-rust/blob/main/examples/tracing-jaeger/src/main.rs
/// - https://quickwit.io/blog/observing-rust-app-with-quickwit-jaeger-grafana
fn init_tracer(
	service_name: impl Into<String>,
	endpoint: impl Into<String>,
) -> Result<opentelemetry_sdk::trace::Tracer, TraceError> {
	let pipeline = opentelemetry_otlp::new_pipeline()
		.tracing()
		.with_exporter(opentelemetry_otlp::new_exporter().tonic().with_endpoint(endpoint.into()))
		.with_trace_config(
			sdktrace::config().with_resource(Resource::new(vec![KeyValue::new(SERVICE_NAME, service_name.into())])),
		);
	if cfg!(test) {
		// we can not reliably detect when the test is finshed so flush every span
		pipeline.install_simple()
	} else {
		pipeline.install_batch(runtime::Tokio)
	}
}
