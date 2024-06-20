use opentelemetry::{trace::TraceError, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{runtime, trace as sdktrace, Resource};
use opentelemetry_semantic_conventions::resource::SERVICE_NAME;
use std::path::PathBuf;
use tracing::{level_filters::LevelFilter, subscriber::set_global_default};
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::{layer::SubscriberExt, Registry};

pub struct TracingBuilder {
	identifier: String,
	base_path: Option<PathBuf>,
	log: Logging,
	/// Trace to open telemetry endpoint.
	open_telemetry: Option<String>,
}
impl TracingBuilder {
	pub fn new(identifier: String, base_path: Option<PathBuf>) -> Self {
		Self { identifier, base_path, log: Logging::None, open_telemetry: None }
	}

	/// Enable bunyan logging to log_path.
	/// If no path is specified {path}/log/application.log is used.
	/// Command read without network stuff:
	/// ```sh
	/// tail -0f ~/Application\ Support/co.app/log/application.log | bunyan -c '!/^(libp2p|hickory_proto)/.test(this.target)'
	/// ```
	pub fn with_bunyan_logging(self, log_path: Option<PathBuf>) -> Self {
		let log = match (log_path, &self.base_path) {
			(Some(p), _) => Logging::Bunyan(p),
			//None => self.path.join("log").join(format!("{}.log", &self.identifier)),
			(None, Some(base_path)) => Logging::Bunyan(base_path.join("log").join("co.log")),
			_ => Logging::None,
		};
		Self { log, ..self }
	}

	pub fn with_open_telemetry(self, endpoint: impl Into<String>) -> Self {
		Self { open_telemetry: Some(endpoint.into()), ..self }
	}

	pub fn init(self) -> Result<(), anyhow::Error> {
		// open telemetry
		let open_telemetry = if let Some(endpoint) = &self.open_telemetry {
			Some(open_telemetry_endpoint(self.identifier.clone(), endpoint.clone())?)
		} else {
			None
		};

		// log
		match &self.log {
			Logging::Bunyan(log_path) => {
				std::fs::create_dir_all(log_path.parent().ok_or(anyhow::anyhow!("no parent"))?)?;
				let log_file = std::fs::File::options().append(true).create(true).open(log_path)?;
				// let formatting_layer = BunyanFormattingLayer::new("co-daemon".into(), std::io::stdout);
				let formatting_layer = BunyanFormattingLayer::new(self.identifier.clone(), log_file);
				let subscriber = Registry::default()
					.with(open_telemetry)
					.with(LevelFilter::TRACE)
					.with(JsonStorageLayer)
					.with(formatting_layer);
				set_global_default(subscriber)?;
				LogTracer::init()?;
			},
			_ => {},
		}

		// result
		Ok(())
	}
}

#[derive(Debug, Clone)]
enum Logging {
	None,
	Bunyan(PathBuf),
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
