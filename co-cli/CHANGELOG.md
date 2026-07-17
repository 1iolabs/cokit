# Changelog

## [Unreleased]

### Added

- `co network listen` accepts `--max-circuit-bytes` and `--max-circuit-duration` when `--relay` is enabled.
- `co network listen --relay` prints its peer ID and one copy-paste relay bootstrap address per external address, independently of verbose logging.
- macOS: `CO_LOG=oslog` routes logs to the unified system log.

### Removed

- **Breaking.** `co network webrtc-signal`. Use `co network listen` with a `/ws` listen address and `--relay`.

### Changed

- **Breaking.** Logging is unified under `--log` / `CO_LOG` using the `co-tracing` grammar — a
  `;`-separated list of `sink[:envfilter]` entries, e.g. `CO_LOG=file:info,co_sdk=trace;stderr:error`,
  with `RUST_LOG` as the per-sink fallback. `-v`/`-q`, `--log-path`, and `--open-telemetry[-endpoint]`
  are unchanged.
- Fixed `ImageInfo` init errors caused by breaking `co-messaging` changes

## [0.1.0] - 2026-03-31

Initial release.
