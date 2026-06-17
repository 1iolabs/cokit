// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

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
use std::path::{Path, PathBuf};
use tracing::Level;
use tracing_bunyan_formatter::BunyanFormattingLayer;
use tracing_subscriber::{fmt::writer::MakeWriterExt, layer::SubscriberExt, util::SubscriberInitExt, Layer};

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

	// tracing: log (bunyan file). CO_LOG selects the file target; CO_LOG_FILTER/RUST_LOG + log_level
	// drive a per-layer EnvFilter so the file honors per-target directives. Stderr stays on -v/-q.
	let log = if let Some(path) = file_target(cli.log.as_ref(), cli.no_log, &default_log_path(&cli), &log_path(&cli)) {
		tokio::fs::create_dir_all(path.parent().ok_or(anyhow::anyhow!("no parent"))?).await?;
		let log_file = std::fs::File::create(&path)?;
		let rust_log = std::env::var("RUST_LOG").ok();
		let directives = co_sdk::resolve_filter(cli.log_filter.as_deref(), rust_log.as_deref());
		let env_filter = co_sdk::env_filter(cli.log_level.to_level(), directives.as_deref());
		let formatting_layer = BunyanFormattingLayer::new(cli.instance_id.to_owned(), log_file)
			.serialize_span_id(true)
			.serialize_span_type(true)
			.serialize_span_fields(false);
		Some(formatting_layer.with_filter(env_filter))
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
	if telemetry.is_some() || output.is_some() || log.is_some() {
		tracing_subscriber::registry().with(telemetry).with(output).with(log).init();
	}

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

fn default_log_path(cli: &Cli) -> PathBuf {
	let base_path = if let Some(path) = &cli.base_path { path.clone() } else { ApplicationBuilder::default_path() };
	base_path.join("log/co.log")
}

fn log_path(cli: &Cli) -> PathBuf {
	cli.log_path.clone().unwrap_or_else(|| default_log_path(cli))
}

/// Resolve the bunyan file target. `CO_LOG` (`log`) wins over the legacy `--no-log`/`--log-path`;
/// stderr is governed separately by `-v`/`-q`, so `off`/`stderr` here just mean "no file".
fn file_target(
	log: Option<&co_sdk::LogSink>,
	no_log: bool,
	default_path: &Path,
	legacy_path: &Path,
) -> Option<PathBuf> {
	match log {
		Some(co_sdk::LogSink::Off | co_sdk::LogSink::Stderr) => None,
		Some(co_sdk::LogSink::Default | co_sdk::LogSink::File) => Some(default_path.to_path_buf()),
		Some(co_sdk::LogSink::Path(p)) => Some(p.clone()),
		None if no_log => None,
		None => Some(legacy_path.to_path_buf()),
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use co_sdk::LogSink;
	use std::path::Path;

	const DEF: &str = "/d/co.log";
	const LEG: &str = "/l/co.log";

	#[test]
	fn co_log_off_no_file() {
		assert_eq!(file_target(Some(&LogSink::Off), false, Path::new(DEF), Path::new(LEG)), None);
	}

	#[test]
	fn co_log_stderr_no_file() {
		assert_eq!(file_target(Some(&LogSink::Stderr), false, Path::new(DEF), Path::new(LEG)), None);
	}

	#[test]
	fn co_log_file_uses_default() {
		assert_eq!(file_target(Some(&LogSink::File), true, Path::new(DEF), Path::new(LEG)), Some(PathBuf::from(DEF)));
	}

	#[test]
	fn co_log_path_wins_over_no_log() {
		assert_eq!(
			file_target(Some(&LogSink::Path("/x.log".into())), true, Path::new(DEF), Path::new(LEG)),
			Some(PathBuf::from("/x.log"))
		);
	}

	#[test]
	fn legacy_no_log() {
		assert_eq!(file_target(None, true, Path::new(DEF), Path::new(LEG)), None);
	}

	#[test]
	fn legacy_log_path() {
		assert_eq!(file_target(None, false, Path::new(DEF), Path::new(LEG)), Some(PathBuf::from(LEG)));
	}
}
