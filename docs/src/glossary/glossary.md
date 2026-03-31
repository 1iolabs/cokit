# Glossary

Table of Contents:

<!-- toc -->

## AGPLv3
GNU Affero General Public License v3.  
A strong copyleft license created by the [Free Software Foundation](https://www.fsf.org/bulletin/2021/fall/the-fundamentals-of-the-agplv3) to ensure that not only distributed binaries – but even software accessed over a network – must have source code made available.

Overview:
- ***Differs from GPLv3***: AGPLv3 closes the "SaaS loophole" by requiring modified source code to be provided when the software is used over a network.
- ***Copyleft principle***: Any changes or extensions (including those accessed remotely) must remain under AGPLv3.
- ***Compatibility***: AGPLv3 is compatible with GPLv3. You can combine AGPLv3 and GPLv3 code, but section 13 ensures that the combined work is covered by the AGPL terms.

For further information, see:
- [GNU Affero General Public License](https://www.gnu.org/licenses/agpl-3.0.html)
- [Legal notice](../license/legal-notice.md)

## CBOR
Concise Binary Object Representation.  
CBOR is a compact, binary-data serialization format based on the JSON data model.  
It was designed to support extremely small code size, efficient message encoding, and extensibility, without requiring version negotiation.  
Defined by IETF in RFC 8949.

Specs:
- ***Binary Format***: Unlike JSON, CBOR encodes data in binary form.
- ***Extensible***: CBOR supports “tags” that identify special data types (e.g. dates, binary blobs).

For further information, see:
- [cbor.io](https://cbor.io/)
- [RFC 8949: Concise Binary Object Representation (CBOR)](https://www.rfc-editor.org/rfc/rfc8949.html)

## DAG-CBOR
IPLD Data Model as a subset of CBOR.  
DAG-CBOR is the go-to serialization method for data used throughout COKIT.

For further information, see:
- [Core](../reference/core.md#serialization)
- [DAG-CBOR specification](https://ipld.io/specs/codecs/dag-cbor/spec/)
- [Glossary: CID](#cid)
- [Glossary: IPLD](#ipld)
- [Glossary: Merkle-DAG](#merkle-dag)

## DID
Decentralized identifiers (DIDs) are a new type of identifier that enables verifiable, decentralized digital identity.  
A DID refers to any subject (e.g. a person, organization, thing, data model, abstract entity, etc.) as determined by the controller of the DID.

For further information, see:
- [Decentralized Identifiers (DIDs) v1.0](https://www.w3.org/TR/did-1.0/)

## DID Contact
A discovery protocol that gossips encrypted didcomm messages using the libp2p GossipSub protocol.

For further information, see:
- [didcontact protocol](../reference/network.md#protocol-didcontact)

## CLA
A CLA (Contributor License Agreement) is a legal contract between a contributor and a project that grants the project the necessary permissions to use, distribute, and sublicense the contributor’s code or other contributions.  
We use it as an addition to the agreements made within the AGPLv3 License.  

It ensures that:

- The contributor has the rights to submit the work (e.g. they wrote it or their employer allows it)
- The project obtains adequate rights – such as copyright assignment or an irrevocable license – to include and redistribute the contributions under its license terms.

For further information, see:
- [Legal notice](../license/legal-notice.md)

## CID
A CID is a self-describing, content-addressed identifier used in distributed systems like IPFS and IPLD.  
Instead of pointing to *where* the data is stored, it refers to *what* the data is via a cryptographic hash.

Specs:
- ***Deterministic & Immutable***: Any change yields a totally different CID to ensure verifiability and immutable data.
- ***Self-Describing Format***: CIDs combine a hash (via *multihash*), a content-type code (*multicodec*), and encoding info (*multibase*).

Why COKIT uses CIDs:  
CIDs allow consistent, tamper-evident data referencing across decentralized storage backends – be they local, IPFS, cloud–supporting, or COKIT’s file-based, content-addressed architecture.


For further information, see:
- [IPLD - The data model of the content-addressable web](https://ipld.io/)
- [IPLD](#ipld)
- [Merkle-DAG](#merkle-dag)

## CO
A CO is a virtual room for collaboration.  
CO stands for Collaborative Object, and is a fundamentally new concept of distributed collaboration.

A CO is not just another group chat.  
Rather, it serves a multitude of functionalities in a distributed network, while running locally on each participant's device.

For further information, see:
- [CO](../reference/co.md)
- [Core](../reference/core.md)

## CO-API
The CO-API is the foundation package to create COKIT Cores.

For further information, see:
- [Core](../reference/core.md#serialization)
- [Core API Overview](../usage/api-overview-core.md)
- [`co-api`](/crate/co_api/index.html)

## GUARD
GUARD is the optional trust and governance layer for COKIT. It is distributed separately (in the `guard` repository) under a source-available license (not open source). It adds advanced access policy, trust controls, certification hooks, and the Proof-of-Authority consensus guard on top of the open-source COKIT platform.

COKIT works without GUARD.

For further information, see:
- [Guards](../reference/guards.md)
- [Legal notice](../license/legal-notice.md)

## COKIT
In essence, COKIT is a Software Development Kit written in Rust.

With COKIT, you can easily build...

- decentralized
- secure
- scalable
- local-first
- peer-to-peer
- collaborative

...applications that make full use of your skills.  
There are virtually no limitations that you might find with cloud providers or other SDKs.

For further information, see:
- [COKIT](../reference/COKIT.md)
- [COKIT in the FAQs](../faq/faq.md#general)

## Consensus
In COKIT, consensus refers to the protocols ensuring that multiple peers agree on a shared state or actions – even in the presence of unreliable networks or malicious actors.

For further information, see:
- [Consensus](../reference/consensus.md)
- [Proof‑of‑Authority (PoA) Consensus Mechanism](#proofofauthority-consensus-mechanism)

## Core
Core stands for CO Reducer.  
It combines data model with business logic.

For further information, see:
- [Core](../reference/core.md)

## Core Actions
Core actions are operations on the state of a Core.

For further information, see:
- [Actions](../reference/core.md#actions)

## Core Schema
The Core schema is the data model of the Core.

For further information, see:
- [Schema](../reference/core.md#schema)

## Core State
The Core state is the root state of a Core.

For further information, see:
- [State](../reference/core.md#state)

## CRDT
A CRDT (Conflict-Free Replicated Data Type) is a specialized data structure designed for distributed systems.  
It allows each replica to be updated independently and concurrently, without locking or central coordination, and still achieve eventual consistency through deterministic merge rules.

Benefits:
- ***Conflict‑free***: Operations commute, ensuring that replicas converge to the same state regardless of operation order.
- ***Strong eventual consistency***: When all updates are delivered, every replica reaches the same final state.
- ***No coordination needed***: Replicas can be updated offline and merge upon reconnection.

COKIT leverages CRDTs to implement the built-in log-based Merkle-CRDT:
- Distributed nodes stay in sync without locking or conflicts.
- Network partitions or offline work don’t block progress.
- Updates merge correctly once communication resumes.

For further information, see:
- [Log](../reference/log.md)

## Dioxus
Dioxus is a Rust-based framework for building cross-platform user interfaces, supporting web, desktop, mobile, and server environments with a single codebase.

For further information, see:
- [Dioxus docs](https://docs.rs/dioxus/latest/dioxus/)

## Guards
Guards are checks for transactions.

For further information, see:
- [Guards](../reference/guards.md#guards)

## Heads
The heads represent the end of the log and also a specific state of the data.

For further information, see:
- [Log](../reference/log.md)

## IPLD
IPLD is a single namespace for all hash-inspired protocols.  
Through IPLD, links can be traversed across protocols, allowing you to explore data regardless of the underlying protocol.

For further information, see:
- [IPLD - The data model of the content-addressable web](https://ipld.io/)
- [Core](../reference/core.md)
- [Glossary: CID](#cid)
- [Glossary: Merkle-DAG](#merkle-dag)

## Log
The Log is a conflict-free replicated event stream.  
It is immutable, cryptographically verifiable, and eventually consistent.  
It is sorted using a [Merkle-DAG](../glossary/glossary.md#merkle-dag)-based logical clock.

For further information, see:
- [Log](../reference/log.md)

## mDNS
mDNS (Multicast Domain Name System) is a lightweight, zero-configuration networking protocol that resolves hostnames to IP addresses within local networks without the need for a dedicated DNS server.  
It enables devices on the same network to discover each other using human-readable names rather than IP addresses.

COKIT uses mDNS as an optional networking mode to automatically discover peer nodes on a local network.

## Merkle-CRDT
A Merkle‑CRDT combines the benefits of [Merkle-DAGs](../glossary/glossary.md#merkle-dag) (Merkle Directed Acyclic Graphs) and [CRDTs](../glossary/glossary.md#crdt) (Conflict‑Free Replicated Data Types) to create a robust, decentralized synchronization layer.

For further information, see:
- [Log](../reference/log.md#merkle-crdt)

## Merkle-DAG
A Merkle-DAG is a content-addressed Directed Acyclic Graph data structure, where each node is cryptographically hashed based on its payload and its links to child nodes. This creates a self-verifying graph.

Specifics:
- ***Immutable and self-verifying***: Each node's identifier uniquely represents its content and all descendants. Any change produces a new [CID](#cid) and a new graph branch/root.
- ***Graph, not tree***: Unlike strict Merkle trees, Merkle DAGs allow nodes to have multiple parents and include data payloads in non-leaf nodes.
- ***Content-addressed deduplication***: Identical content chunks share the same [CID](#cid) and need not to be stored more than once, reducing storage and bandwidth.

In COKIT, we use Merkle-DAGs as the foundation for the built-in Merkle log-based CRDT and storage.

For further information, see:
- [IPLD - The data model of the content-addressable web](https://ipld.io/)
- [Core](../reference/core.md)
- [Glossary: CID](#cid)
- [Glossary: IPLD](#ipld)
- [Glossary: Merkle-CRDT](#merkle-crdt)

## PeerID
A Peer Identity (often written `PeerID`) is a unique reference to a specific peer within the overall p2p-network.  
It is derived by hashing a node’s public key. The corresponding private key remains secret and is used to sign messages, and to authenticate the identity of the peer.

In COKIT, each node may generate or be assigned a Peer ID, which then acts as a verifiable handle across the decentralized networking layers.

For further information, see:
- [Network](../reference/network.md)
- [Peers - libp2p](https://docs.libp2p.io/concepts/fundamentals/peers/)

## Proof‑of‑Authority Consensus Mechanism
PoA (Proof‑of‑Authority) is a reputation-based consensus mechanism where only a small, pre-approved set of trusted validator–known entities with verifiable identities are permitted to produce and validate transactions.

For further information, see:
- [Consensus](../reference/consensus.md#proof-of-authority)
- [Glossary: Consensus](#consensus)

## Storage
One of the base building blocks of COKIT is the content addressed storage [CID](../glossary/glossary.md#cid).  
The storage is represented as a very simple interface that writes and reads CID/BLOB pairs called Blocks.  

The recommended serialization format (also used throughout COKIT) is [DAG-CBOR](../glossary/glossary.md#dag-cbor), which is a subset of [CBOR](../glossary/glossary.md#cbor) with links to [CIDs](../glossary/glossary.md#cid).  
[Cores](../reference/core.md) are not restricted to [DAG-CBOR](../glossary/glossary.md#dag-cbor), however, and may use any given structure.

For further information, see:
- [Storage](../reference/storage.md)

## Tauri
Tauri is an open-source framework for building cross-platform, lightweight, secure, and fast desktop (and mobile) applications, using web technologies for the UI and Rust for the backend logic.

For further information, see:
- [Tauri docs](https://v2.tauri.app/start/)

## Tokio
Tokio is an asynchronous runtime for the Rust programming language.  
It provides the building blocks needed for writing network applications.  
It gives the flexibility to target a wide range of systems, from large servers with dozens of cores to small embedded devices.

For further information, see:
- [Tokio - An asynchronous Rust runtime](https://tokio.rs/)

## WASM
WebAssembly (WASM) is an open-standard, portable binary format designed for high-performance execution in a sandboxed environment - initially for web browsers, and increasingly for broader contexts, including servers, edge devices, and embedded systems.  
It serves as a compilation target for languages such as Rust, C, C++, and others, enabling near-native speeds while maintaining security and cross-platform compatibility

For COKIT, WASM is a mechanism to compile and execute schema logic.

For further information, see:
- [Core](../reference/core.md)
