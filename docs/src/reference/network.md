# Network

CO-kit got a built-in network stack utilising libp2p.
The actual networking protocols used can be configured for every CO.
The peer-to-peer networking is optional and possibly any other protocols like http or file protocols like NFS can be used with some effort.
There is no lock-in to a single networking strategy.
Just describe your data using a core and easily adapt CO-kit to your infrastructure.

## Supported Interfaces
#todo
- Ethernet
- Wifi
- Bluetooth Low Energy (BLE)

## libp2p
libp2p, (short for “library peer-to-peer”) is a peer-to-peer (P2P) networking framework that enables the development of P2P applications.
It consists of a collection of protocols, specifications, and libraries that facilitate P2P communication between network participants or, in other words, peers.
CO-kit uses the [rust implementation of libp2p](https://github.com/libp2p/rust-libp2p/).

## Protocols
### Protocol: mDNS
Used for **local peer discovery** via multicast DNS ([RFC 6762](https://datatracker.ietf.org/doc/html/rfc6762)).
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
A Network protocol to send [didcomm](https://identity.foundation/didcomm-messaging/spec/v2.1/)-encoded message to a given peer.
It uses a libp2p sub-stream for message transfer with a length prefixed streaming protocol.
CO-kit uses this to send discovery, join and invite messages directly to peers.

### Protocol: bitswap
Bitswap is a protocol for exchanging blocks of data.
It is a message-based protocol where all messages contain want-lists or blocks.
CO-kit uses a extended version of Bitswap which includes token based authorization.

#### See also
- https://docs.ipfs.tech/concepts/bitswap/
- [IPIP-270: Bitswap 1.3.0 - Tokens (and auth) support #270](https://github.com/ipfs/specs/pull/270)
- https://github.com/dkuhnert/libp2p-bitswap/tree/auth

### Protocol: didcontact
Discovery protocol which gossips encrypted didcomm messages using the GossipSub protocol from libp2p.
The didcomm messages denotes a connection request from one CO participant to another one.
The receivicer can choose whether to respond to it or not, so no (potentially private) connection information must be shared beforehand.

#todo sequence diagram
