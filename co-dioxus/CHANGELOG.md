# Changelog

## [Unreleased]

### Added

- `CoSettings::with_log(LogArgs)` and the `log` field for logging configuration.
- `CoSettings::with_log_layer(factory)` — attach an app-provided `tracing` layer, built lazily
  when co-dioxus installs the global subscriber.
- Re-exports `LogArgs`, `LogConfig` (from `co-tracing`).
- `tracing` (default) and `oslog` cargo features. `tracing` gates the optional `co-tracing`
  dependency and the logging API; `oslog` enables the Apple `oslog` sink.

### Changed

- **Breaking.** Logging is configured via a `co_tracing::LogArgs` instead of `CoLog`/`CoLogLevel`,
  and the subscriber is installed automatically when the context starts (desktop = stderr/file,
  web = browser console). Migrate with e.g.
  `CoSettings::new(/* … */).with_log(LogArgs::parse("file:info")?)`, or flatten `LogArgs` from your
  CLI with `#[command(flatten)] log: co_dioxus::LogArgs`.
- **Breaking.** `co-tracing` is now an optional dependency behind the new (default) `tracing`
  feature. Disabling it (`default-features = false` without `tracing`) compiles out the logging
  API — `with_log`/`with_log_layer`, the `log` field, and the `LogArgs`/`LogConfig` re-exports.
- **Breaking.** `oslog` is no longer enabled automatically by `mobile`; opt in via the new `oslog`
  feature.
- **Breaking.** `CoSettings` is now `#[non_exhaustive]`; construct it via `CoSettings::new`/
  `from_cli` and the builder methods.

### Removed

- **Breaking.** The `CoLog` and `CoLogLevel` types (and their re-exports).
- **Breaking.** The built-in `dioxus/logger` integration; the `js` feature no longer pulls it
  (logging now goes through `co-tracing`).

## [0.1.0] - 2026-03-31

Initial release.
