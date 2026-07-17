# Changelog

## [Unreleased]

### Changed

- **Breaking:** [`NetworkSettings::listen`] is now a `BTreeSet<Multiaddr>` instead of a single `Multiaddr`, so a node can listen on several addresses at once. The default is dual-stack QUIC: `/ip4/0.0.0.0/udp/0/quic-v1` and `/ip6/::/udp/0/quic-v1`.
- Automatic Identify direct-address upgrades are now opt-in through the default-disabled [`NetworkSettings::dial_redundancy`] setting.

### Added
- [`NetworkSettings::with_listens`]
- [`NetworkSettings::with_added_listen`]
- [`NetworkSettings::ephemeral_peer_id`] sets if new peer ids should be created without saving on startup of network
- [`NetworkSettings::with_ephemeral_peer_id`] sets the new setting
- [`NetworkSettings::dial_redundancy`] and [`NetworkSettings::with_dial_redundancy`] enable best-effort redundant COKIT-owned automatic dials.

### Fixed
- Identify dial (automatic upgrade to a direct connection on a peer's listen addresses): a failed upgrade dial now releases its addresses and is retried on the peer's next identify, instead of staying blocked until the peer fully disconnects. Dial attempts and failures are logged at `debug`/`info` (`network-identify-dial`, `network-identify-dial-error`) for on-device diagnosis.
- `DialNetworkTask` now completes when libp2p rejects a dial because its `PeerCondition` is false, instead of hanging when the peer is disconnected.
- Empty known-peer address lists now retain `NetworkBehaviour` address resolution, including addresses stored by mDNS.

## [0.1.0] - 2026-03-31

Initial release.
