use opentelemetry::{trace::TraceError, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{runtime, trace as sdktrace, Resource};
use opentelemetry_semantic_conventions::resource::SERVICE_NAME;
use std::path::PathBuf;
use tracing::{subscriber::set_global_default, Level};
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::{fmt::writer::MakeWriterExt, layer::SubscriberExt, EnvFilter, Registry};

pub struct TracingBuilder {
	identifier: String,
	base_path: Option<PathBuf>,
	bunyan: Option<PathBuf>,
	stderr: bool,
	env_filter: Option<Option<String>>,
	/// Trace to open telemetry endpoint.
	open_telemetry: Option<String>,
}
impl TracingBuilder {
	pub fn new(identifier: String, base_path: Option<PathBuf>) -> Self {
		Self { identifier, base_path, bunyan: None, stderr: false, open_telemetry: None, env_filter: None }
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

	pub fn with_open_telemetry(self, endpoint: impl Into<String>) -> Self {
		Self { open_telemetry: Some(endpoint.into()), ..self }
	}

	pub fn with_stderr_logging(self) -> Self {
		Self { stderr: true, ..self }
	}

	pub fn with_env_filter(self, env: Option<String>) -> Self {
		Self { env_filter: Some(env), ..self }
	}

	pub fn init(self) -> Result<(), anyhow::Error> {
		// env_filter
		let env_filter = self.env_filter.map(|env| match env {
			Some(env) => EnvFilter::from_env(env),
			None => EnvFilter::from_default_env(),
		});
		let env_filter = match env_filter {
			Some(env_filter) => Some(
				env_filter
					.add_directive("co_sdk=trace".parse()?)
					.add_directive("co_network=trace".parse()?)
					.add_directive("info".parse()?),
			),
			None => None,
		};

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
			Some(BunyanFormattingLayer::new(self.identifier.clone(), log_file))
		} else {
			None
		};

		// stderr
		let stderr = if self.stderr {
			Some(tracing_subscriber::fmt::layer().with_writer(std::io::stderr.with_max_level(Level::TRACE)))
		} else {
			None
		};

		// init
		if open_telemetry.is_some() || bunyan.is_some() || stderr.is_some() {
			let subscriber = Registry::default()
				.with(open_telemetry)
				.with(env_filter)
				.with(JsonStorageLayer)
				.with(bunyan)
				.with(stderr);
			set_global_default(subscriber)?;
			LogTracer::init()?;
		}

		// result
		Ok(())
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
	opentelemetry_otlp::new_pipeline()
		.tracing()
		.with_exporter(opentelemetry_otlp::new_exporter().tonic().with_endpoint(endpoint.into()))
		.with_trace_config(
			sdktrace::config().with_resource(Resource::new(vec![KeyValue::new(SERVICE_NAME, service_name.into())])),
		)
		.install_batch(runtime::Tokio)
}
