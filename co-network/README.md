# co-network

COKIT Networking related primitives and peer-to-peer implementation using libp2p.

## Diagnostics

`NetworkApi::overview()` returns a read-only [`NetworkOverview`] snapshot — local peer id, current listeners and connection state (peers, COs, DIDs, networks, bootstrap) - for status/debug views.
