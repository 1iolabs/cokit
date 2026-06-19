# Changelog

## [Unreleased]

### Changed

- **Breaking.** `CoApplicationSettings` replaced the `no_log: bool` field with
  `log: co_tracing::LogArgs`, and `start_application` now installs the subscriber via `co-tracing`
  (shutting it down on application shutdown). Configure logging through `log` — e.g.
  `LogArgs::parse("file:info")`, flattened from your CLI, or `LogArgs::parse("off")` to disable.

## [0.1.0] - 2026-03-31

Initial release.
