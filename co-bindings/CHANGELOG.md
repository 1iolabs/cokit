# Changelog

## [Unreleased]

### Added

- **Breaking (FFI).** `CoNetworkSettings::dial_redundancy` (Dart: `dialRedundancy`) exposes
  opt-in automatic redundant dialing. Direct record constructors must provide the new field;
  generated defaults use `false`.

### Changed

- **Breaking (FFI).** `CoSettings` configures logging via a single `log` string (the `CO_LOG`
  grammar — `;`-separated `sink[:envfilter]` entries; sinks `file`, `stderr`, `oslog`, or a path;
  defaults to `"file:info"`) instead of `noLog` + `logLevel`. In Dart, replace
  `CoSettings(noLog: false, logLevel: CoLogLevel.info)` with `CoSettings(log: "file:info")`
  (`"off"` to disable, `"stderr:debug"` for console at debug). Regenerate bindings with
  `flutter_rust_bridge_codegen generate` if you build them yourself.

### Removed

- **Breaking (FFI).** The `noLog` and `logLevel` `CoSettings` fields and the `CoLogLevel` enum.

## [0.1.0] - 2026-03-31

Initial release.
