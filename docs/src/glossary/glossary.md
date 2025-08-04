# Glossary

<!-- toc -->

### AGPLv3
(GNU Affero General Public License v3)
### **AGPLv3 (GNU Affero General Public License v3)**
A *strong copyleft* license created by the Free Software Foundation to ensure that not only distributed binaries—but even software accessed over a network—must have source code made available

- **What makes it different from GPLv3**: AGPLv3 closes the "SaaS loophole" by requiring modified source code to be provided when the software is used over a network.

- **Copyleft principle**: Any changes or extensions (including those accessed remotely) must remain under AGPLv3 .

- **Compatibility**: AGPLv3 is compatible with GPLv3. You can combine AGPLv3 and GPLv3 code, but section 13 ensures that the combined work is covered by the AGPL terms.

- **Linking Exception**: An optional addendum that allows proprietary or differently-licensed software to link to AGPLv3 code without forcing the entire application to adopt AGPLv3, granted you don’t modify the interface itself.
---
### cbor
(Concise Binary Object Representation)
**CBOR** is a compact, binary data serialization format based on the JSON data model. It was designed to support extremely small code size, efficient message encoding, and extensibility without requiring version negotiation. Defined by IETF in **RFC 8949**, CBOR excels in scenarios where performance, compactness, and flexibility matter.

- **Binary Format**: Unlike human-readable JSON, CBOR encodes data in binary form, making it faster to parse and more space-efficient.

- **Extensible**: CBOR supports “tags” that identify special data types (e.g., dates, binary blobs), enabling schema-free evolution and custom data additions.
---
### CLA
A **Contributor License Agreement (CLA)** is a legal contract between a contributor and a project that grants the project the necessary permissions to use, distribute, and sublicense the contributor’s code or other contributions.
We use it as an addition to the agreements made within the AGPLv3 License.
It ensures that:

- The contributor **has the rights** to submit the work (e.g., they wrote it or their employer allows it)

- The project **obtains adequate rights** — such as copyright assignment or an irrevocable license — to include and redistribute the contributions under its license terms.
---
### CID
A **CID** is a self-describing, content-addressed identifier used in distributed systems like IPFS and IPLD. Instead of pointing to *where* data is stored, it refers to *what* the data is via a cryptographic hash.

- **Deterministic & Immutable**: Any change, even a single byte, yields a totally different CID, ensuring verifiability and immutable data.

- **Self-Describing Format**: CIDs combine a hash (via *multihash*), a content-type code (*multicodec*), and encoding info (*multibase*), making them flexible and future-proof.

**Why CO-kit uses CIDs**:
CIDs allow consistent, tamper-evident data referencing across decentralized storage backends—be they local, IPFS, or cloud—supporting CO-kit’s file-based, content-addressed architecture.

---
### CO
A CO is a virtual room for collaboration.

CO (virtual data room) is a distributed database whose data is encrypted and is only available to the participants (unique via DID) of the data room. The CO stores references (unique via CID) of the data. The data itself is stored on the participants' devices. The DIDs, permissions and the identities of the participants (PrivateKeys) are stored in a data structure (data structure for states) “COre”. Each CO contains at least one COre. They act as “in-code databases” that store details such as the DIDs of the participants in a CO, their roles (admin, reader, etc.), permissions and status information (states) of systems such as chat rooms.

---
### CO-api
#todo

---
### CO-kit
In essence, CO-kit is a Software Development Kit written in Rust.

With CO-kit, you can easily build...

- decentralized
- hyper-secure
- hyper-scalable
- local-first
- peer-to-peer
- & collaborative ...applications that make full use of your skills - there are virtually no limitations that you might have with cloud providers or other SDKs.

---
### Consensus
In CO-kit, **consensus** refers to the protocols ensuring multiple peers agree on shared state or actions—even in the presence of unreliable networks or malicious actors.

**Key properties of a consensus protocol**:
- **Agreement**: All honest peers must decide on the same value.
- **Validity**: The decision must reflect a value proposed by a peer.
- **Termination**: Every peer eventually makes a decision, even if some fail.

Consensus ensures data integrity, prevents conflicting updates, and supports reliable collaboration in a fully decentralized environment.

#### CO-kit’s Flexible Consensus Modes

CO-kit allows you to choose the level of coordination needed for each Collaborative Object:

- `none`: No explicit agreement rules—relies solely on CRDT merge guarantees.
- `proof-of-authority`: Updates must be approved or signed by designated nodes.
- `manual`: Users or admins explicitly approve changes before commit.
- `shared`: A quorum or team of peers must agree to apply state changes.

These options let you balance **complexity**, **security**, and **performance** based on your application's requirements.

---
### Core
A core (CO reducer) is a piece of data that acts like a state. Cores can be directly added to COs and they work like an in-code database. They implement a reducer function that take actions which have been pushed to a CO. The reducer then changes the cores data accordingly.
#### Core actions
#todo
#### Core schema
#todo
#### Core state
#todo

---
### CRDT
A **CRDT** is a specialized data structure designed for **distributed systems** that allows each replica to be updated **independently and concurrently**, without locking or central coordination, and still achieve **eventual consistency** through deterministic merge rules.

**Core properties**:
- **Conflict‑free**: Operations commute, ensuring that replicas converge to the same state regardless of operation order.

- **Strong eventual consistency**: When all updates are delivered, every replica reaches the same final state.

- **No coordination needed**: Replicas can be updated offline and merge upon reconnection.

**Types**:
- **State-based (CvRDT)**: Each replica periodically sends its full or delta state to others and resolves merges with deterministic lattice joins.

- **Operation-based (CmRDT)**: Updates are propagated as individual commutative operations, relying on reliable distribution.

**Why CO-kit uses CRDTs**:
CO-kit leverages CRDTs to implement its **built-in Merkle log-based CRDT sync**, ensuring that:
- Distributed nodes stay in sync **without locking or conflicts**.
- **Network partitions or offline work** don’t block progress.
- Updates merge correctly **once communication resumes**.
---
### Dioxus
**Dioxus** is a modern, **Rust-based framework for building cross-platform user interfaces**, supporting web, desktop, mobile, and server environments with a single codebase.

- **Declarative UI with RSX**: Uses an `rsx!` macro similar to JSX, allowing you to write HTML-like layouts directly in Rust code (e.g., `rsx! { h1 { "Hello World!" } }`)
- **Cross-platform target support**:
  - **Web**: via WebAssembly, including SSR and hydration
  - **Desktop**: through WebView or native renderers
  - **Mobile**: supports Android and iOS via JNI/Objective-C interop.
- **Ergonomic reactivity**: Inspired by React, Solid, and Svelte, it uses signals/hooks like `use_signal` for state management.
- **Productive developer workflow**:
  - Integrated **hot-reloading** and CLI tool (`dx`) for instant iteration
  - Built-in **bundler** for optimized, compact builds (< 50 KB web apps, < 5 MB desktop/mobile)
- **Full-stack and backend integration**: Includes server-side functions, routing, and streaming support—letting frontend invoke backend logic with type safety.

**Why CO-kit docs reference Dioxus**: It exemplifies how Rust can power reactive, real-time, decentralized UIs—making it a natural companion for illustrating CO-kit integrations.

---
### Guards
#todo

### IPLD
IPLD is a single namespace for all hash-inspired protocols. Through IPLD, links can be traversed across protocols, allowing you to explore data regardless of the underlying protocol.

[see more](https://ipld.io/)

---
### Logs
#todo
#### Heads
#todo

---
### mDNS
**mDNS** is a lightweight, zero-configuration networking protocol that resolves hostnames to IP addresses within local networks without the need for a dedicated DNS server.

#### How it works
- A client sends a DNS query via **UDP multicast** to all peers on the local subnet (IPv4 224.0.0.251, IPv6: FF02::FB) over port 5353.
- The device owning the requested hostname responds with its IP, allowing all peers to update their local mDNS cache
- All mDNS hostnames typically end in `.local`, emphasizing its scope as link-local name resolution.

#### Benefits
- **Zero configuration**: No DNS server or special setup required
- **Dynamic service discovery**: Ideal for local ad hoc setups, IoT devices, file servers, printers, etc.

#### mDNS in CO-kit
CO-kit uses mDNS as an optional networking mode to automatically discover peer nodes and COs on a local subnet - simplifying setup and fostering seamless peer-to-peer collaboration without manual endpoint configuration.

---
### Merkle Dags
A **Merkle DAG** is a content-addressed directed acyclic data structure where each node is cryptographically hashed based on its payload and its links to child nodes. This creates a self-verifying graph ideal for the distributed system of CO-kit.

#### Features
- **Immutable and self-verifying**: Each node's identifier uniquely represents its content and all descendants. Any change produces a new [CID](./) and a new graph branch.
- **Graph, not tree**: Unlike strict merkle trees, merkle DAGs allow nodes to have multiple parents and include data payloads in non-leaf nodes.
- **Content-addressed deduplication**: Identical content chunks share the same CID and need not to be stored more than once, reducing storage and bandwidth.

In CO-kit, we use Merkle DAGs as the foundation for the built-in Merkle log-based CRDT-syncing, especially to enable lightweight storage and content deduplication across COs.

---
### peerID
A Peer Identity (often written `PeerID`) is a unique reference to a specific peer within the overall p2p-network.

- A Peer ID is derived by hashing a node’s **public key**, and the corresponding **private key** remains secret and is used to sign messages and authenticate the identity of the peer.
- Typically represented as a **base58‑encoded multihash** (CIDv0)
- More modern encodings (CIDv1, Base32) are emerging—but the legacy base58 multihash remains widely supported.

In CO-kit, each `CO` or node may generate or be assigned a Peer ID, which then acts as a verifiable handle across the decentralised syncing and networking layers.

[See more](https://docs.libp2p.io/concepts/fundamentals/peers/)

---
### Storage
#todo

### Tauri
**Tauri** is an open-source framework for building **lightweight, secure, and fast** desktop (and mobile) applications using web technologies for the UI and Rust for the backend logic

#### Features

- **Cross-platform support**: Target apps for **Windows, macOS, Linux**, and—starting from Tauri v2—**iOS and Android** from a single codebase
- **Tiny binary size and low memory usage**: Unlike Chromium-based frameworks, Tauri uses the OS's native WebView (e.g., WebView2, WKWebView), resulting in ultra-compact executables (often just a few megabytes) and significantly reduced runtime overhead
- **Security-first architecture**: Built with Rust, Tauri offers memory safety, a minimal attack surface, explicit API permissioning, and optional isolation patterns for untrusted code

#### Architecture & Internals
- Uses Rust-based crates—like **TAO** (for window management) and **WRY** (for WebView integration)—to power the native shell and system interactions
- Your frontend app (React, Vue, Svelte, or vanilla HTML/JS) runs inside a WebView shell that communicates securely with Rust backend via IPC commands (`#[tauri::command]` / `invoke`)

## Tokio
#todo 

### WASM
WebAssembly. 
#todo