# Changelog

## [Unreleased]

Logging/tracing setup has moved to the dedicated `co-tracing` crate; `co-sdk` keeps only the
`tracing` macro facade.

### Added

- `ApplicationBuilder::{base_path, log_path, identifier}` — helpers for wiring up `co-tracing`.
- `Application::drop_on_shutdown(value)` — keep a value (e.g. a `co_tracing::TracingGuard`) alive
  until the application shuts down, then drop it (so OpenTelemetry flushes).

### Changed

- **Breaking.** Changed `join_unrelated_co`, `to_networks` from `BTreeSet<Network>` to `impl Into<CoConnectivity>`.
- **Breaking.** `ApplicationBuilder::build()` no longer installs a tracing subscriber. Set logging
  up via `co-tracing` first — e.g.
  `co_tracing::TracingBuilder::file(builder.identifier(), builder.log_path()).with_optional().init()?`
  as a drop-in for the old `with_bunyan_logging`, then `application.drop_on_shutdown(guard)` (or use
  `co_tracing::LogArgs` for `CO_LOG`-string / CLI configuration).
- **Breaking.** `CoOptions` is now `non_exhaustive`.
- `network_service` function now checks for the new `ephemeral_peer_id` setting

### Removed

- **Breaking.** `ApplicationBuilder::{with_bunyan_logging, with_log_max_level, with_max_level, with_optional_tracing, with_open_telemetry}`.
- **Breaking.** Re-exports `co_sdk::{TracingBuilder, env_filter, LogSink, parse_log_sink, resolve_filter}`.
- **Breaking.** Cargo features `tracing`, `bunyan`, `opentelemetry` (the `native` feature no longer enables them).

## [0.1.0] - 2026-03-31

Initial release.
