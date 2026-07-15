# Changelog

## [Unreleased]

### Changed

- **Breaking:** [`NetworkSettings::listen`] is now a `BTreeSet<Multiaddr>` instead of a single `Multiaddr`, so a node can listen on several addresses at once. The default is dual-stack QUIC: `/ip4/0.0.0.0/udp/0/quic-v1` and `/ip6/::/udp/0/quic-v1`.
- **Breaking:** DID connections use unique leases so overlapping `DidUse` calls require matching releases. `DidUseAction` and `DidReleaseAction` now carry a `DidUseLeaseId`, and `DidConnection` now exposes a set of active lease IDs.
- `ConnectionMessage::did_use` now owns and automatically releases one DID connection lease when its returned stream is dropped. Raw `DidUse` messages must retain `DidUseAction::release()`, close or drop their response receiver, and then dispatch that exact lease release.
- Authoritative DID route failures close the failed response streams in the same actor turn. Later `DidReleased` notifications cannot close a newly acquired route.
- Stale close completions no longer disconnect networks whose CO or DID routes were reacquired. Failure and timeout completions remain authoritative.
- Builds that enable the platform-neutral network API without a transport backend now compile. Network initialization reports that `native` or wasm `web` must be enabled.

### Added
- [`NetworkSettings::with_listens`]
- [`NetworkSettings::with_added_listen`]
- [`NetworkSettings::ephemeral_peer_id`] sets if new peer ids should be created without saving on startup of network
- [`NetworkSettings::with_ephemeral_peer_id`] sets the new setting

## [0.1.0] - 2026-03-31

Initial release.
