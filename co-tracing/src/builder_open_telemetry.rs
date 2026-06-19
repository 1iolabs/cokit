// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 1io BRANDGUARDIAN GmbH

use opentelemetry::{trace::TracerProvider as _, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{runtime, trace as sdktrace, trace::TracerProvider, Resource};
use opentelemetry_semantic_conventions::resource::SERVICE_NAME;

/// Shuts the global tracer provider down (flushing spans) on drop.
pub struct OpenTelemetryGuard;
impl Drop for OpenTelemetryGuard {
	fn drop(&mut self) {
		opentelemetry::global::shutdown_tracer_provider();
	}
}

/// Build an OpenTelemetry tracing layer. `endpoint == "stdout"` uses the stdout exporter; otherwise
/// an OTLP/tonic exporter to `endpoint`. Typed against the base `Registry` (it is composed first).
pub(crate) fn layer(
	service_name: &str,
	endpoint: &str,
) -> Result<
	(tracing_opentelemetry::OpenTelemetryLayer<tracing_subscriber::Registry, sdktrace::Tracer>, OpenTelemetryGuard),
	anyhow::Error,
> {
	let tracer = if endpoint == "stdout" {
		let provider = TracerProvider::builder()
			.with_simple_exporter(opentelemetry_stdout::SpanExporter::default())
			.build();
		provider.tracer(service_name.to_owned())
	} else {
		opentelemetry_otlp::new_pipeline()
			.tracing()
			.with_exporter(opentelemetry_otlp::new_exporter().tonic().with_endpoint(endpoint.to_owned()))
			.with_trace_config(
				sdktrace::config()
					.with_resource(Resource::new(vec![KeyValue::new(SERVICE_NAME, service_name.to_owned())])),
			)
			.install_batch(runtime::Tokio)?
	};
	Ok((tracing_opentelemetry::layer().with_tracer(tracer), OpenTelemetryGuard))
}
