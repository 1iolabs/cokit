# FAQ

<!-- toc -->

## General
### What is COkit and what are its primary use cases?
COkit has been built to provide an SDK that finally allows you to stop worrying about backends and dependencies, and lets you focus on the task at hand.

Each and every application built on COkit shall be:
- easy to build and easy to use
- decentralized
- usable on local infrastructure
- secure
- non-reliant on middlemen through peer-to-peer networking
- a place where users may keep control of (and keep track of) their data

COkit is especially useful for:
- Collaborative applications with a focus on usability and data-ownership for the users
- Communication platforms and messengers in general, with a focus on privacy and data security
- Dual-use in the defence sector: communication for the battlefield and catastrophe management

For further information, see:
- [1io.com](https://1io.com/)

### Is COkit open-source? Under what license is it distributed?
Yes COkit is open-source.

We use APLGv3 as the license of choice.

For further information, see:
- [Legal Notice](../license/legal-notice.md)

### What platforms does COkit support? (e.g. Linux, Windows, macOS, etc.)
COkit is platform-agnostic and supports all major OSes.

And of course, because COkit is written in Rust, it should run on other platforms.

### How does COkit differ from other CRDT or P2P-based frameworks?
COkit not only provides end-to-end encryption in processes, but also end-to-end verification, meaning:
- Cryptographic integrity verification (e.g. This message is unaltered)
- Cryptographic identity verification (e.g. This person is who they claim to be)

Included in COkit (thus significantly reducing programming time when building apps) are the following:
- Identity through [DID](../reference/identity.md#what-is-a-did)
- Auditable and non-repudiable history of states
- Zero-trust environment
- Decentralization from the get-go
- Private data handling – not even encrypted data is shared with unknown peers

### Can COkit-based applications be used offline and sync later?
Yes.  

Every change happens only locally and is eventually distributed to other participants over time.  
Secondly, every piece of data is content-addressed, making it easy to sync between peers.

If too many peers appear to be offline, consensus within a CO is possibly blocked.  
However, this is not an issue as it is both optional and on demand.  
When a consensus is blocked, work can still continue (also between peers), and finality will eventually recover.  
Consensus depends on the CO and the application setup.

For further information, see:
- [Consensus](../reference/consensus.md)
- [FAQ: What happens during a network partition or peer disconnect?](#what-happens-during-a-network-partition-or-peer-disconnect)
- [Glossary: Content addressing](../glossary/glossary.md#cid)


## Architecture & Design
### What does "file-based" mean in the context of COkit?
All data is stored as files without special filesystem requirements like locks, links, or consistency levels.  
Therefore, you can store COkit-managed files on a local disk, on cloud storages, or on object stores.  
All files are stored verifiable using content addressing.

For further information, see:
- [Storage](../reference/storage.md)
- [Glossary: Content addressing](../glossary/glossary.md#cid)

### How are COs structured and persisted?
COs consist of [Cores](../reference/core.md), which are data models.  
A CO is represented as a graph of content-addressed objects called a DAG (Directed Acyclic Graph).  
At the root of a CO is its DAG root, which models all of its data.  
The content-addressed objects are referenced by CIDs/Data pairs, and are stored in a storage backend.

For further information, see:
- [CO](../reference/co.md)
- [Storage](../reference/storage.md)
- [Glossary: DAG-CBOR](../glossary/glossary.md#dag-cbor)
- [Glossary: Merkle-DAG](../glossary/glossary.md#merkle-dag)

### What storage backends are supported out-of-the-box?
A storage backend works as key/value store of content-addressed blocks, where the key is a CID and the value is a binary described by this CID.  
Currently, the following backends are supported:
- Filesystem: Basically uses CID as the filename and the binary as the file contents in a configurable folder.
- Memory: In-memory hash-map-based CID/Binary structure.

For further information, see:
- [Key Principles](../introduction/key-principles.md#file-based)
- [Storage](../reference/storage.md)

### How does the decentralized architecture of COkit ensure data integrity?
COkit uses a data graph per CO, which we call the [Log](../reference/log.md).  
The Log is powered by a Merkle-CRDT, which deterministically orders transactions based on a logical clock.  
Each piece of data is stored as a content-addressed block, which allows for cryptographic verification – at anytime.

For further information, see:
- [Log](../reference/log.md)
- [Glossary: Merkle-CRDT](../glossary/glossary.md#merkle-crdt)

### Can COkit be integrated with traditional centralized systems?
Yes.

COs would be serving as an added layer of trust and security when in use in centralized systems.

The library can be integrated anywhere, and you can also build your backend/data models with COkit.

Another useful scenario is COkit-built apps in processes that are fed back into traditional centralized systems.  
As COs are lightweight, they can be stored and used as receipts (leveraging non-repudiability).

It can also be used to amplify the edge.  
Use low-overhead edge caching through content-addressing, which allows for efficient syncing.  
Let the edge work locally, occasionally syncing to server/cloud/infrastructure in a batch.

## Networking & Synchronization
### How does peer discovery work in COkit's networking model?
For general discovery, COkit uses a GossipSub-based protocol to discover/contact peers.  
Local peer discovery is done by utilizing mDNS.

The networking is entirely optional, and one could just use HTTP or a plain filesystem for syncing and transferring blocks.

For further information, see:
- [Network: DID discovery](../reference/network.md#diddiscovery)
- [Glossary: mDNS](../glossary/glossary.md#mdns)

### What networking mode should I use for local, LAN-only collaboration?
By default, COkit uses mDNS for local device discovery.  

For discovery over the internet, a bootstrap peer can be configured that defaults to `bootstrap.1io.com`. Technically, this bootstrap peer exposes a libp2p gossip-sub endpoint.  
You are free to choose any or no bootstrap peer(s) at all, or to use your own device as bootstrap for your other devices.

Both methods can be used to discover new peers, but they can be changed or disabled depending on your demands.

### Is it possible to run COkit without any network connectivity?
Yes.

It is possible to use COkit only with files.  
COkit requires no strong feature set for file systems - just files.  
As each file is content-addressed using its CID, we just need to read and write them.  
This also makes it possible to just use any cloud storage drive to share the COkit-managed files.

For further information, see:
- [Glossary: content addressing](../glossary/glossary.md#cid)
- [Key Principles](../introduction/key-principles.md#file-based)
- [Storage](../reference/storage.md)

### How does COkit handle NAT traversal and firewalls in P2P mode?
This is handled through a variety of possibilities. 

The first option is through a circuit relay.  
libp2p [defines a protocol called p2p-circuit](https://github.com/libp2p/specs/tree/master/relay).  
When a peer isn’t able to listen on a public address, it can dial out to a relay peer, which will keep a long-lived connection open.  
Other peers will be able to dial through the relay peer using a `p2p-circuit` address, which will forward traffic to its destination.

The second option is by [Direct Connection Upgrade through Relay (DCUtR)](https://docs.libp2p.io/concepts/nat/dcutr/) via Hole Punching.  
It is a protocol for establishing direct connections between nodes through hole punching, without a signalling server.  
DCUtR involves synchronizing and opening connections to each peer’s predicted external addresses.

For further information, see:
- [What are NATs - libp2p](https://docs.libp2p.io/concepts/nat/overview/)

### What happens during a network partition or peer disconnect?
Each peer continues locally by writing operations as a content-addressed event to the [Log](../reference/log.md).  
This may cause participants heads to diverge.  
When a peer reconnects, the heads are shared again and the Log joins new heads, deterministically sorting all events to a consistent order, which is then used to recalculate the state.

## Data & Consensus
### How are conflicts resolved using Merkle-CRDTs in COkit?
Conflicts are resolved by the [Log](../reference/log.md), which sorts the stream of events by the event's logical clock.

### Can I define custom consensus logic for my application?
Yes. 

A consensus is implemented as a Core with an additional guard.

For further information, see:
- [Guards](../reference/guards.md)
- [Consensus](../reference/consensus.md)

### What level of schema validation or migration support does COkit offer?
Schema validation is up to the developer of the [Core](../reference/core.md), and can be implemented as desired.  
Cores can be migrated between versions. The migration itself is just another event that can be implemented in code.

For further information, see:
- [Core](../reference/core.md#migrations)

### How does COkit manage partial data availability across distributed nodes?
As all data is represented as a graph, more precisely as a DAG (directed acyclic graph), the data is always accessed top-down.  
This means that we can fetch more data as we walk down the graph.  

In addition, content addressing ensures validity of the data.  
Distribution happens organically, but you can always opt to fetch all the data if needed.

For further information, see:
- [Glossary: DAG-CBOR](../glossary/glossary.md#dag-cbor)
- [Glossary: Merkle-DAG](..glossary/glossary.md#merkle-dag)
