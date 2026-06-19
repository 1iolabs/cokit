# Changelog

## [Unreleased]

### Added

- Initial release: opinionated logging/tracing setup for COKIT, extracted from `co-sdk`.
- `TracingBuilder` + `TracingGuard` — compose per-sink layers (stderr, bunyan file, OpenTelemetry,
  browser console, Apple `oslog`) and install them globally (`init`) or scoped (`init_scope`).
- `TracingBuilder::file(identifier, path)` — a one-call bunyan-file builder; drop-in for the old
  `ApplicationBuilder::with_bunyan_logging` (pass `application_builder.log_path()`).
- Unified `CO_LOG` grammar (`parse_log`): a `;`-separated list of `sink[:envfilter]` entries (e.g.
  `file:info,co_sdk=trace;stderr:error`); `RUST_LOG` is the per-sink fallback.
- `LogArgs` — a reusable `clap` flag group (`#[command(flatten)]`) with `LogArgs::parse` — plus
  `LogContext` for app-supplied defaults (`identifier`, `base_path`, `default_stderr`, `oslog_subsystem`).
- Cargo features: `subscriber`, `bunyan` (default), `opentelemetry`, `console` (wasm), `oslog`
  (Apple), `clap`.
