# Changelog

## [Unreleased]

### Added

- `CoSettings::with_log(LogArgs)` and the `log` field for logging configuration.
- Re-exports `LogArgs`, `LogConfig` (from `co-tracing`).

### Changed

- **Breaking.** Logging is configured via a `co_tracing::LogArgs` instead of `CoLog`/`CoLogLevel`,
  and the subscriber is installed automatically when the context starts (desktop = stderr/file,
  web = browser console, iOS = `oslog`). Migrate with e.g.
  `CoSettings::new(/* … */).with_log(LogArgs::parse("file:info")?)`, or flatten `LogArgs` from your
  CLI with `#[command(flatten)] log: co_dioxus::LogArgs`.

### Removed

- **Breaking.** The `CoLog` and `CoLogLevel` types (and their re-exports).
- **Breaking.** The `tracing` cargo feature; the `js` feature no longer pulls `dioxus/logger`.

## [0.1.0] - 2026-03-31

Initial release.
