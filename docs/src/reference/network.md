# Network

CO-kit has a built-in peer-to-peer network stack utilizing [libp2p](https://libp2p.io/).

The peer-to-peer networking is optional.
Any other protocol like http or file protocols like NFS can be used (with some effort).
There is no lock-in to a single networking strategy; just describe your data using a core and easily adapt CO-kit to your infrastructure.

The actual networking protocols used can be configured for every CO.

## Network Configuration
For each [CO](../reference/co.md), a variety of network configurations for connectivity can be made.

For further information see:
- [co-core-co: Co: network](/crate/co_core_co/struct.Co.html#structfield.network)
- [co-core-co: CoAction: NetworkInsert](/crate/co_core_co/enum.CoAction.html#variant.NetworkInsert)
- [co-core-co: CoAction: NetworkRemove](/crate/co_core_co/enum.CoAction.html#variant.NetworkRemove)

### DidDiscovery
Gossipsub-based mesh networking discovery.
This allows to configure explicit DIDs to connect, instead of (by default) all participants in a CO.

For further information see:
- [DidContact](../reference/network.md#protocol-didcontact)
- [co-primitives: Network: DidDiscovery](/crate/co_primitives/enum.Network.html#variant.DidDiscovery)

### CoHeads
GossipSub-based broadcasting and subscription-based connectivity.

For further information see:
- [Protocol: Rendezvous](#protocol-coheads)
- [co-primitives: Network: CoHeads](/crate/co_primitives/enum.Network.html#variant.CoHeads)

### Rendezvous
Register the CO to a rendezvous node.

For further information see:
- [Protocol: Rendezvous](#protocol-rendezvous)
- [co-primitives: Network: DidDiscovery](/crate/co_primitives/enum.Network.html#variant.DidDiscovery)

### Peer
Direct configured endpoints (IP/DNS).

This can be used to host the CO on a dedicated server/cloud or other infrastructure.

For further information see:
- [co-primitives: Network: Peer](/crate/co_primitives/enum.Network.html#variant.Peer)

### HTTP
Directly configured HTTP endpoint.

This can be used to host the CO on a dedicated server/cloud or other infrastructure.

Coming soon[^issue-78].

[^issue-78]: [Network: HTTP (#78)](https://gitlab.1io.com/1io/co-sdk/-/issues/78)

## Supported Interfaces
- Ethernet / Wi-Fi (TCP/IP, QUIC)
- Bluetooth Low Energy (BLE) (Coming soon[^issue-79])
- WebRTC / WebSocket (Coming soon[^issue-89])
- Wi-Fi Direct (Coming soon[^issue-90])

[^issue-79]: [Network: Bluetooth (BLE) (#79)](https://gitlab.1io.com/1io/co-sdk/-/issues/79)
[^issue-89]: [Network: WebRTC (#89)](https://gitlab.1io.com/1io/co-sdk/-/issues/89)
[^issue-90]: [Network: Wi-Fi Direct (#90)](https://gitlab.1io.com/1io/co-sdk/-/issues/90)

## libp2p
libp2p is a networking framework that enables the development of P2P applications.
It consists of a collection of protocols, specifications, and libraries that facilitate P2P communication between network participants or, in other words, peers.
We use the [rust implementation of libp2p](https://github.com/libp2p/rust-libp2p/).

## Protocols
### Protocol: mDNS
Used for local peer discovery via multicast DNS ([RFC 6762](https://datatracker.ietf.org/doc/html/rfc6762)).
Peers broadcast `_p2p._udp.local` PTR queries, and libp2p-capable nodes respond with their multi-addresses.

CO-kit uses the libp2p mDNS client.

### Protocol: Noise
The [Noise Protocol Framework](https://noiseprotocol.org/) is a widely-used encryption scheme that allows for secure communication by combining cryptographic primitives into patterns with verifiable security properties.
CO-kit uses the [libp2p noise transport](https://docs.libp2p.io/concepts/secure-comm/noise/).

### Protocol: QUIC
QUIC is a new transport protocol that provides an always-encrypted, stream-multiplexed connection built on top of UDP.
CO-kit uses the [libp2p QUIC transport](https://docs.libp2p.io/concepts/transports/quic/) by default.

### Protocol: Ping
The libp2p ping protocol is a simple liveness check that peers can use to test the connectivity and performance between two peers. The libp2p ping protocol is different from the ping command line utility, as it requires an already established libp2p connection.

### Protocol: Identify
The identify protocol allows peers to exchange information about each other, most notably their public keys and known network addresses.

### Protocol: didcomm
A Network protocol to send didcomm-encoded message to a given peer.
It uses a libp2p sub-stream for message transfer with a length prefixed streaming protocol.
CO-kit uses this to send discovery, join and invite messages directly to peers.

For further information see:
- [DIDComm Messaging Specification v2.1](https://identity.foundation/didcomm-messaging/spec/v2.1/)

### Protocol: bitswap
Bitswap is a protocol for exchanging blocks of data.
It is a message-based protocol where all messages contain want-lists (which blocks we are interested in) or blocks.
CO-kit uses an extended version of Bitswap which includes token based authorization.

For further information see:
- [Bitswap](https://docs.ipfs.tech/concepts/bitswap/)
- [IPIP-270: Bitswap 1.3.0 - Tokens (and auth) support #270](https://github.com/ipfs/specs/pull/270)
- [libp2p-bitswap](https://github.com/dkuhnert/libp2p-bitswap/tree/auth)

### Protocol: didcontact
A discovery protocol which gossips encrypted didcomm messages using the libp2p GossipSub protocol.
The didcomm messages denote a connection request from one CO participant to another one.
The receiver can choose whether to respond to it or not, so no (potentially private) connection information must be shared beforehand.

### Protocol: Rendezvous
Provides a lightweight mechanism for generalized peer discovery.
Any node implementing the rendezvous protocol can act as a rendezvous point, allowing the discovery of relevant peers in a decentralized manner.

For further information see:
- [Rendezvous Protocol](https://github.com/libp2p/specs/blob/master/rendezvous/README.md)

### Protocol: CoHeads
A GossipSub-based protocol with topics (e.g. ID of the CO) that you can subscribe to or publish heads to in a permissionless manner.
When other participants are subscribed to the topic, they can be used to exchange heads.

Whenever a CO (respectively its heads) changes, a new gossip message with the changed heads will be issued.

For further information see:
- [What is Publish/Subscribe](https://docs.libp2p.io/concepts/pubsub/overview/#gossip)

## References
- [Flexible Networking Model](../introduction/features.md#flexible-networking-model)
