# Networking and Discovery

## Overview

COkit has a built-in peer-to-peer network stack using the Rust implementation of libp2p.
P2P networking is **entirely optional**. Any protocol (HTTP, file-based sync like NFS, etc.)
can be used instead. There is no lock-in to a single networking strategy.

Network configuration is **per-CO**: each CO can have different connectivity settings.

## Network Configuration Options

Each CO can be configured with one or more of these modes:

| Mode | Description |
|------|-------------|
| **DidDiscovery** | GossipSub-based mesh discovery. Connect to specific DIDs rather than all participants. |
| **CoHeads** | GossipSub topic-based broadcasting. Subscribe to a CO's topic to exchange heads. |
| **Rendezvous** | Register CO to a Rendezvous node for decentralized peer discovery. |
| **Peer** | Direct endpoint (IP/DNS). For dedicated server/cloud hosting. |
| **HTTP** | Direct HTTP endpoint. Coming soon (issue #78). |

The default protocol for participants is **didcontact** (GossipSub-based DID discovery).

## Supported Transport Interfaces

- Ethernet / Wi-Fi (TCP/IP, QUIC) -- available now
- Bluetooth Low Energy (BLE) -- coming soon (issue #79)
- WebRTC / WebSocket -- coming soon (issue #89)
- Wi-Fi Direct -- coming soon (issue #90)

Default transport: **QUIC** (`/ip4/0.0.0.0/udp/0/quic-v1`).

## Protocols in Detail

### mDNS
Local peer discovery via multicast DNS (RFC 6762). Peers broadcast `_p2p._udp.local`
queries. Used for LAN/WiFi discovery without internet.

### Noise
Noise Protocol Framework for encrypted transport. Used as libp2p's encryption layer.

### QUIC
Default transport protocol. Always-encrypted, stream-multiplexed, built on UDP.

### Ping
Simple liveness check between connected peers.

### Identify
Exchange peer information (public keys, known network addresses).

### didcomm
Sends DIDComm-encoded messages to a specific peer. Uses libp2p sub-stream with length-
prefixed streaming. Used for discovery, join, and invite messages.
Spec: DIDComm Messaging Specification v2.1.

### bitswap
Block exchange protocol. Message-based: peers exchange want-lists and blocks.
COkit uses an **extended version with token-based authorization** (IPIP-270).

### didcontact
Discovery protocol that gossips encrypted DIDComm messages via GossipSub.
A connection request from one CO participant to another. The receiver chooses whether
to respond, so no private connection info needs to be shared upfront.

### CoHeads
GossipSub-based protocol. Topics = CO IDs. Publish/subscribe heads. When a CO's heads
change, a gossip message with the new heads is issued to all subscribers.

### Rendezvous
Lightweight generalized peer discovery. Any node can act as a rendezvous point.

## NAT Traversal

Two mechanisms:
1. **Circuit Relay**: Peer dials out to a relay, which keeps a long-lived connection.
   Other peers dial through the relay using `p2p-circuit` addresses.
2. **DCUtR (Direct Connection Upgrade through Relay)**: Hole punching for direct
   connections without a signaling server.

## Bootstrap Configuration

Default bootstrap peer: `/dns4/bootstrap.1io.com/udp/5000/quic-v1/p2p/12D3KooW...`
(a libp2p GossipSub endpoint). Configurable: use any, none, or your own bootstrap peers.

## Application-Level Network Settings

When starting networking via `Application::create_network(NetworkSettings)`:
- **force_new_peer_id**: Create new PeerID on each start (privacy improvement).
- **listen**: Endpoint to listen on (default: all interfaces, random port, QUIC).
- **bootstrap**: Bootstrap endpoints for improved connectivity.

## Running Without Network

COkit works without any network. Data can be synced via filesystem alone (copy files).
All files are content-addressed, so just reading/writing CID-named files suffices.
Cloud storage drives (S3, NFS, etc.) can also serve as a sync mechanism.

## Source Map

- docs/src/reference/network.md (primary: all protocols, configuration, interfaces)
- docs/src/glossary/glossary.md (definitions: mDNS, PeerID, DID Contact)
- docs/src/usage/configuration.md (network settings for apps)
- docs/src/faq/faq.md (Q&A: peer discovery, LAN, no-network, NAT traversal)
- docs/src/introduction/features.md (feature: Flexible Networking Model)
- docs/src/introduction/key-principles.md (principle: BYO infrastructure, P2P)
