# Glossary

<!-- toc -->

## AGPLv3
GNU Affero General Public License v3.
A *strong copyleft* license created by the Free Software Foundation to ensure that not only distributed binaries—but even software accessed over a network—must have source code made available

Key points:
- What makes it different from GPLv3: AGPLv3 closes the "SaaS loophole" by requiring modified source code to be provided when the software is used over a network.
- Copyleft principle: Any changes or extensions (including those accessed remotely) must remain under AGPLv3.
- Compatibility: AGPLv3 is compatible with GPLv3. You can combine AGPLv3 and GPLv3 code, but section 13 ensures that the combined work is covered by the AGPL terms.
- Linking Exception: An optional addendum that allows proprietary or differently-licensed software to link to AGPLv3 code without forcing the entire application to adopt AGPLv3, granted you don’t modify the interface itself.

For further information see:
- [GNU Affero General Public License](https://www.gnu.org/licenses/agpl-3.0.de.html)

## CBOR
Concise Binary Object Representation.
CBOR is a compact, binary data serialization format based on the JSON data model.
It was designed to support extremely small code size, efficient message encoding, and extensibility without requiring version negotiation.
Defined by IETF in RFC 8949, CBOR excels in scenarios where performance, compactness, and flexibility matter.

Key points:
- Binary Format: Unlike human-readable JSON, CBOR encodes data in binary form, making it faster to parse and more space-efficient.
- Extensible: CBOR supports “tags” that identify special data types (e.g., dates, binary blobs), enabling schema-free evolution and custom data additions.

For further information see:
- [cbor.io](https://cbor.io/)
- [RFC 8949: Concise Binary Object Representation (CBOR)](https://www.rfc-editor.org/rfc/rfc8949.html)

## DAG-CBOR
IPLD Data Model as a subset of CBOR.
DAG-CBOR is the goto serialization method for data used throughout CO-kit.

For further information see:
- [Core](../reference/core.md#serialization)
- [DAG-CBOR specification](https://ipld.io/specs/codecs/dag-cbor/spec/)
- [Glossary: Cid](#cid)
- [Glossary: IPLD](#ipld)
- [Glossary: Merkle-DAG](#merkle-dag)

## didcontact
A discovery protocol which gossips encrypted didcomm messages using the libp2p GossipSub protocol.

For further information see:
- [didcontact protocol](../reference/network.md#protocol-didcontact)

## CLA
A Contributor License Agreement (CLA) is a legal contract between a contributor and a project that grants the project the necessary permissions to use, distribute, and sublicense the contributor’s code or other contributions.
We use it as an addition to the agreements made within the AGPLv3 License.
It ensures that:

- The contributor has the rights to submit the work (e.g., they wrote it or their employer allows it)
- The project obtains adequate rights — such as copyright assignment or an irrevocable license — to include and redistribute the contributions under its license terms.

## CID
A CID is a self-describing, content-addressed identifier used in distributed systems like IPFS and IPLD. Instead of pointing to *where* data is stored, it refers to *what* the data is via a cryptographic hash.

- Deterministic & Immutable: Any change, even a single byte, yields a totally different CID, ensuring verifiability and immutable data.
- Self-Describing Format: CIDs combine a hash (via *multihash*), a content-type code (*multicodec*), and encoding info (*multibase*), making them flexible and future-proof.

Why CO-kit uses CIDs:
CIDs allow consistent, tamper-evident data referencing across decentralized storage backends—be they local, IPFS, or cloud—supporting CO-kit’s file-based, content-addressed architecture.

For further information see:
- [IPLD - The data model of the content-addressable web](https://ipld.io/)
- [IPLD](#ipld)
- [Merkle-DAG](#merkle-dag)

## CO
A CO is a virtual room for collaboration.

CO (virtual data room) is a distributed database whose data is encrypted and is only available to the participants (unique via DID) of the data room. The CO stores references (unique via CID) of the data. The data itself is stored on the participants' devices. The DIDs, permissions and the identities of the participants (PrivateKeys) are stored in a data structure (data structure for states) “COre”. Each CO contains at least one COre. They act as “in-code databases” that store details such as the DIDs of the participants in a CO, their roles (admin, reader, etc.), permissions and status information (states) of systems such as chat rooms.

For further information see:
- [CO](../reference/co.md)

## CO-API
The CO-API is the foundation package to create CO-kit cores.

For further information see:
- [Core](../reference/core.md#serialization)
- [Core API Overview](../usage/api-overview-core.md)

## CO-kit
In essence, CO-kit is a Software Development Kit written in Rust.

With CO-kit, you can easily build...

- decentralized
- hyper-secure
- hyper-scalable
- local-first
- peer-to-peer
- & collaborative ...applications that make full use of your skills - there are virtually no limitations that you might have with cloud providers or other SDKs.

For further information see:
- [CO-kit in the FAQs](../faq/faq.md#general)

## Consensus
In CO-kit, consensus refers to the protocols ensuring multiple peers agree on shared state or actions—even in the presence of unreliable networks or malicious actors.

Key properties of a consensus protocol:
- Agreement: All honest peers must decide on the same value.
- Validity: The decision must reflect a value proposed by a peer.
- Termination: Every peer eventually makes a decision, even if some fail.

Consensus ensures data integrity, prevents conflicting updates, and supports reliable collaboration in a fully decentralized environment.

CO-kit allows you to choose the level of coordination needed for each CO:
- `none`: No explicit agreement rules - relies solely on CRDT merge guarantees.
- `proof-of-authority`: Updates must be approved or signed by designated participants.
- `manual`[^issue-87]: Users or admins explicitly approve changes before commit.
- `shared`[^issue-88]: A quorum or team of peers must agree to apply state changes.

These options let you balance complexity, security, and performance based on your application's requirements.

[^issue-87]: https://gitlab.1io.com/1io/co-sdk/-/issues/87
[^issue-88]: https://gitlab.1io.com/1io/co-sdk/-/issues/88

For further information see:
- [Consensus](../reference/consensus.md)
- [Proof‑of‑Authority (PoA) Consensus Mechanism](#proofofauthority-poa-consensus-mechanism)

## Core
A core (CO reducer) is a piece of data that acts like a state. Cores can be directly added to COs and they work like an in-code database. They implement a reducer function that take actions which have been pushed to a CO. The reducer then changes the cores data accordingly.

For further information see:
- [Core](../reference/core.md)

## Core schema
The core schema is data model of the core.

For further information see:
- [Schema](../reference/core.md#schema)

## Core actions
Core actions are operations on the state of a core.

For further information see:
- [Actions](../reference/core.md#actions)

## Core state
The core state is the root state of a core.

For further information see:
- [State](../reference/core.md#state)

## CRDT
A CRDT (Conflict Free Replicated Data Type) is a specialized data structure designed for distributed systems that allows each replica to be updated independently and concurrently, without locking or central coordination, and still achieve eventual consistency through deterministic merge rules.

Key points:
- Conflict‑free: Operations commute, ensuring that replicas converge to the same state regardless of operation order.
- Strong eventual consistency: When all updates are delivered, every replica reaches the same final state.
- No coordination needed: Replicas can be updated offline and merge upon reconnection.

CO-kit leverages CRDTs to implement the built-in log-based Merkle-CRDT.
- Distributed nodes stay in sync without locking or conflicts.
- Network partitions or offline work don’t block progress.
- Updates merge correctly once communication resumes.

For further information see:
- [Log](../reference/log.md)

## Dioxus
Dioxus is a Rust-based framework for building cross-platform user interfaces, supporting web, desktop, mobile, and server environments with a single codebase.

For further information see:
- [Dioxus docs](https://docs.rs/dioxus/latest/dioxus/)

## Guards
Guards are checks for transactions.
They serve as a sort of "Police" for transactions and decide which transactions will make it into the [Log](../reference/log.md) and which don't.
New transactions will be checked by the configured guards of a CO and will be rejected if not all guards succeed.
Just like [Cores](../reference/core.md), Guards are pure functions, are compiled to WebAssembly, and registered to COs.

Important notice: Guards are not permissions.

For further information see:
- [Guards](../reference/guards.md#guards)

## IPLD
IPLD is a single namespace for all hash-inspired protocols. Through IPLD, links can be traversed across protocols, allowing you to explore data regardless of the underlying protocol.

For further information see:
- [IPLD - The data model of the content-addressable web](https://ipld.io/)
- [Core](../reference/core.md)
- [Glossary: CID](#cid)
- [Glossary: Merkle-DAG](#merkle-dag)

## Log
The Log is a conflict-free replicated event stream. It is immutable and cryptographically verifiable.
It is (eventually consistent) sorted using a Merkle-DAG-based logical clock.

For further information see:
- [Log](../reference/log.md)

## Heads
The heads represent the end of the log and also a specific state of the data.

For further information see:
- [Log](../reference/log.md)

## mDNS
mDNS (Multicast Domain Name System) is a lightweight, zero-configuration networking protocol that resolves hostnames to IP addresses within local networks without the need for a dedicated DNS server.
It enables devices on the same network to discover each other using human-readable names rather than IP addresses.

CO-kit uses mDNS as an optional networking mode to automatically discover peer nodes on a local network simplifying setup and fostering seamless peer-to-peer collaboration without manual endpoint configuration.

## Merkle-CRDT
A Merkle‑CRDT combines the strengths of Merkle‑DAGs (Directed Acyclic Graphs) and CRDTs (Conflict‑Free Replicated Data Types) to create a robust, decentralized synchronization layer.

For further information see:
- [Log](../reference/log.md#merkle-crdt)

## Merkle-DAG
A Merkle-DAG is a content-addressed directed acyclic data structure where each node is cryptographically hashed based on its payload and its links to child nodes. This creates a self-verifying graph.

Specifics:
- Immutable and self-verifying: Each node's identifier uniquely represents its content and all descendants. Any change produces a new [CID](#cid) and a new graph branch/root.
- Graph, not tree: Unlike strict merkle trees, merkle DAGs allow nodes to have multiple parents and include data payloads in non-leaf nodes.
- Content-addressed deduplication: Identical content chunks share the same [CID](#cid) and need not to be stored more than once, reducing storage and bandwidth.

In CO-kit, we use Merkle DAGs as the foundation for the built-in Merkle log-based CRDT and storage.

For further information see:
- [IPLD - The data model of the content-addressable web](https://ipld.io/)
- [Core](../reference/core.md)
- [Glossary: Cid](#cid)
- [Glossary: IPLD](#ipld)

## PeerID
A Peer Identity (often written `PeerID`) is a unique reference to a specific peer within the overall p2p-network.
It is derived by hashing a node’s public key, and the corresponding private key remains secret and is used to sign messages and authenticate the identity of the peer.

In CO-kit, each node may generate or be assigned a Peer ID, which then acts as a verifiable handle across the decentralised networking layers.

For further information see:
- [Peers - libp2p](https://docs.libp2p.io/concepts/fundamentals/peers/)

## Proof‑of‑Authority (PoA) Consensus Mechanism
Proof‑of‑Authority (PoA) is a reputation-based consensus mechanism where only a small, pre-approved set of trusted validators—known entities with verifiable identities—are empowered to produce and validate transactions.

For further information see:
- [Consensus](../reference/consensus.md)
- [Glossary: Consensus](#consensus)

## Storage
One of the base building blocks of CO-kit is the content addressed storage [CID](../glossary/glossary.md#cid).
The storage is represented as a very simple interface which writes and reads CID/BLOB pairs called Blocks.
The recommended serialization format (also used throughout CO-kit) is DAG-CBOR which is a subset of CBOR with links to CIDs.
A [core](../reference/core.md) is not restricted to [DAG-CBOR](../glossary/glossary.md#dag-cbor) and may use any given structure.

For further information see:
- [Storage](../reference/storage.md)

## Tauri
Tauri is an open-source framework for building cross-platform, lightweight, secure, and fast desktop (and mobile) applications using web technologies for the UI and Rust for the backend logic.

For further information see:
- [Tauri docs](https://v2.tauri.app/start/)

## Tokio
Tokio is an asynchronous runtime for the Rust programming language. It provides the building blocks needed for writing network applications. It gives the flexibility to target a wide range of systems, from large servers with dozens of cores to small embedded devices.

For further information see:
- [Tokio - An asynchronous Rust runtime](https://tokio.rs/)

## WASM
WebAssembly (WASM) is an open-standard, portable binary format designed for high-performance execution in a sandboxed environment - initially for web browsers, and increasingly for broader contexts including servers, edge devices, and embedded systems.
It serves as a compilation target for languages such as Rust, C, C++, and others, enabling near-native speed while maintaining security and cross-platform compatibility

For CoKit, WASM offers a powerful mechanism to compile and execute schema logic in a safe, efficient, and portable manner—supporting modular, decentralized functionality across diverse environments
