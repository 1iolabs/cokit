// CONFIDENTIAL — © 1io BRANDGUARDIAN GmbH. Proprietary CoKIT code/docs for internal use within our company domain and authorized users/tools only; do not copy, disclose, or transmit any part outside this domain.
// No license is granted by access (any AGPLv3 references are non-operative until official publication); prohibited for AI/model training or retention—approved secure tools may process solely for internal use.

use clap::Parser;
use cli::Cli;
use co_sdk::ApplicationBuilder;
use opentelemetry::{
	trace::{TraceError, TracerProvider as _},
	KeyValue,
};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{runtime, trace as sdktrace, trace::TracerProvider, Resource};
use opentelemetry_semantic_conventions::resource::SERVICE_NAME;
use std::path::PathBuf;
use tracing::Level;
use tracing_bunyan_formatter::BunyanFormattingLayer;
use tracing_subscriber::{fmt::writer::MakeWriterExt, layer::SubscriberExt, util::SubscriberInitExt};

mod cli;
mod commands;
mod library;

fn main() {
	let result = tokio::runtime::Builder::new_multi_thread()
		.enable_all()
		.build()
		.unwrap()
		.block_on(async { app_main().await });
	std::process::exit(result.unwrap());
}

async fn app_main() -> anyhow::Result<exitcode::ExitCode> {
	let cli = cli::Cli::parse();

	// tracing: verbose
	let output = if !cli.quiet {
		let writer = match cli.verbose {
			0 => std::io::stderr.with_max_level(Level::WARN),
			1 => std::io::stderr.with_max_level(Level::INFO),
			2 => std::io::stderr.with_max_level(Level::DEBUG),
			_ => std::io::stderr.with_max_level(Level::TRACE),
		};
		Some(tracing_subscriber::fmt::layer().with_writer(writer))
	} else {
		None
	};

	// tracing: log
	let log = if !cli.no_log {
		let log_path = log_path(&cli);
		tokio::fs::create_dir_all(log_path.parent().ok_or(anyhow::anyhow!("no parent"))?).await?;
		let log_file = std::fs::File::create(log_path)?;
		let formatting_layer =
			BunyanFormattingLayer::new(cli.instance_id.to_owned(), log_file.with_max_level(cli.log_level.to_level()))
				.serialize_span_id(true)
				.serialize_span_type(true)
				.serialize_span_fields(false);
		Some(formatting_layer)
	} else {
		None
	};

	// tracing: open telemetry
	let (telemetry, _telemetry_flush) = if cli.open_telemetry {
		struct TracerCleanup {}
		impl Drop for TracerCleanup {
			fn drop(&mut self) {
				opentelemetry::global::shutdown_tracer_provider()
			}
		}

		// telemetry
		let telemetry = if cli.open_telemetry_endpoint == "stdout" {
			let provider = TracerProvider::builder()
				.with_simple_exporter(opentelemetry_stdout::SpanExporter::default())
				.build();
			tracing_opentelemetry::layer().with_tracer(provider.tracer(cli.instance_id.clone()))
		} else {
			tracing_opentelemetry::layer().with_tracer(
				init_tracer(cli.instance_id.clone(), cli.open_telemetry_endpoint.clone())
					.expect("open telementry tracer"),
			)
		};
		// opentelemetry::global::set_text_map_propagator(opentelemetry_sdk::propagation::TraceContextPropagator::new());
		// opentelemetry::global::tracer(cli.instance_id.clone()).in_span("test", |cx| {
		// 	cx.span().add_event("test", vec![]);
		// });
		println!("tracing: {}", cli.open_telemetry_endpoint);
		(Some(telemetry), Some(TracerCleanup {}))
	} else {
		(None, None)
	};

	// tracing
	tracing_subscriber::registry().with(telemetry).with(output).with(log).init();

	// execute
	cli::command(&cli).await
}

/// See:
/// - https://github.com/open-telemetry/opentelemetry-rust/blob/main/examples/tracing-jaeger/src/main.rs
/// - https://quickwit.io/blog/observing-rust-app-with-quickwit-jaeger-grafana
fn init_tracer(service_name: String, endpoint: String) -> Result<opentelemetry_sdk::trace::Tracer, TraceError> {
	opentelemetry_otlp::new_pipeline()
		.tracing()
		.with_exporter(opentelemetry_otlp::new_exporter().tonic().with_endpoint(endpoint))
		.with_trace_config(
			sdktrace::config().with_resource(Resource::new(vec![KeyValue::new(SERVICE_NAME, service_name)])),
		)
		.install_batch(runtime::Tokio)
}

fn log_path(cli: &Cli) -> PathBuf {
	if let Some(path) = &cli.log_path {
		return path.clone();
	}
	let base_path = if let Some(path) = &cli.base_path { path.clone() } else { ApplicationBuilder::default_path() };
	base_path.join("log/co.log")
}
