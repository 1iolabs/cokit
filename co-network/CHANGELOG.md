# Changelog

## [Unreleased]

### Changed

- **Breaking:** [`NetworkSettings::listen`] is now a `BTreeSet<Multiaddr>` instead of a single `Multiaddr`, so a node can listen on several addresses at once. The default is dual-stack QUIC: `/ip4/0.0.0.0/udp/0/quic-v1` and `/ip6/::/udp/0/quic-v1`.

### Added
- [`NetworkSettings::with_listens`]
- [`NetworkSettings::with_added_listen`]

## [0.1.0] - 2026-03-31

Initial release.
