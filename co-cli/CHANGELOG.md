# Changelog

## [Unreleased]

### Added

- macOS: `CO_LOG=oslog` routes logs to the unified system log.

### Changed

- **Breaking.** Logging is unified under `--log` / `CO_LOG` using the `co-tracing` grammar — a
  `;`-separated list of `sink[:envfilter]` entries, e.g. `CO_LOG=file:info,co_sdk=trace;stderr:error`,
  with `RUST_LOG` as the per-sink fallback. `-v`/`-q`, `--log-path`, and `--open-telemetry[-endpoint]`
  are unchanged.

## [0.1.0] - 2026-03-31

Initial release.
