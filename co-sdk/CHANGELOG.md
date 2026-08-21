# Changelog

## [Unreleased]

Logging/tracing setup has moved to the dedicated `co-tracing` crate; `co-sdk` keeps only the
`tracing` macro facade.

### Added

- `ApplicationBuilder::{base_path, log_path, identifier}` — helpers for wiring up `co-tracing`.
- `Application::drop_on_shutdown(value)` — keep a value (e.g. a `co_tracing::TracingGuard`) alive
  until the application shuts down, then drop it (so OpenTelemetry flushes).
- `push_heads_to_dids` and `HeadsRecipient` for sending current CO heads to
  selected DIDs with durable retries.
- `CoReducer::join_states(BTreeSet<CoReducerState>)` — join several trusted
  states in one operation that inserts every snapshot before the first join.

### Changed

- `DidKeyProvider` rejects stored private keys whose derived DID does not
  exactly match the requested DID.
- **Breaking.** Changed `join_unrelated_co`, `to_networks` from `BTreeSet<Network>` to `impl Into<CoConnectivity>`.
- **Breaking.** `ApplicationBuilder::build()` no longer installs a tracing subscriber. Set logging
  up via `co-tracing` first — e.g.
  `co_tracing::TracingBuilder::file(builder.identifier(), builder.log_path()).with_optional().init()?`
  as a drop-in for the old `with_bunyan_logging`, then `application.drop_on_shutdown(guard)` (or use
  `co_tracing::LogArgs` for `CO_LOG`-string / CLI configuration).
- **Breaking.** `CoOptions` is now `non_exhaustive`.
- `network_service` function now checks for the new `ephemeral_peer_id` setting.
- Head delivery persists tagged intent before authorization, preparation,
  routing, or network send.
- Pending delivery tasks coalesce atomically by `(task-type, co, recipient)`,
  retaining the latest pending delivery intent for each key.
- Queue claims and completions are conditional; stale operations are no-ops and
  cannot mutate a newer task generation.
- New network tasks now wake the existing queue processor when networking is available.
- Incoming Heads messages prepare independently, so an unresolved head does
  not block later valid updates for the same CO. Ready joins are coalesced.
- **Breaking.** `HeadsDeliveryAttempt` is now `HeadsDeliveryPhase`, with
  `Admission` and `Execution` variants.
- The queue does not provide exactly-once sends or takeover of tasks abandoned
  by crashed executors.

### Removed

- **Breaking.** `ApplicationBuilder::{with_bunyan_logging, with_log_max_level, with_max_level, with_optional_tracing, with_open_telemetry}`.
- **Breaking.** Re-exports `co_sdk::{TracingBuilder, env_filter, LogSink, parse_log_sink, resolve_filter}`.
- **Breaking.** Cargo features `tracing`, `bunyan`, `opentelemetry` (the `native` feature no longer enables them).

## [0.1.0] - 2026-03-31

Initial release.
