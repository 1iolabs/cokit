# FAQ

<!-- toc -->

## General
### What is CO-kit and what are its primary use cases?
- CO-kit has been built to provide an SDK that finally allows you to stop worrying about backends and dependencies and lets you focus on the task at hand.
- Each and every application built on CO-kit shall be:
	- easy to build and easy to use
	- decentralized
	- usable on local infrastructure
	- secure
	- non-reliant on middlemen through peer-to-peer
	- a place where users keep control and track over their data

CO-kit is especially useful for:
- Collaborative Applications with focus on usability and data-ownership for the users.
- Communications platforms and messengers with focus on privacy and data security
- Dual-use in the defence sector: Communication for the battlefield and catastrophe management
- [See more on our website](https://1io.com/de)

### Is CO-kit open-source? Under what license is it distributed?
- Yes CO-kit is open-source
- We use APLGv3 as the license of choice

For further information see:
- [Legal Notice](../license/legal-notice.md)

### What platforms does CO-kit support? (e.g., Linux, Windows, macOS, etc.)
- CO-kit is fully platform agnostic and can be supported by any OS

### How does CO-kit differ from other CRDT or P2P-based frameworks?
#todo #review
Not only does CO-kit provide end-to-end encryption in processes, but also end-to-end verification, meaning:
- Cryptographic integrity verification
- Cryptographic identity verification

Included in CO-kit and thus significantly reducing programming time when building apps are the following:
- Identity through [DID](../reference/Identity.md##What is a DID)
- Auditable and non-repudiable history of states
- Zero-trust environment 
- Decentralization from the get-go
- Fully private data handling as not even encrypted data is shared with unknown peers

### Can CO-kit be used offline and sync later?
#todo 
Yes. Every change happens only local and are eventually distributed to other participants over time. Secondly, every piece of data is content-addressed; this makes it easy to sync between peers.
If too many peers appear to be offline, consensus within a CO is possibly block, but this is not an issue and depends on the CO-setup, is optional and on demand. 

- See [[#What happens during a network partition or peer disconnect?]]

## Architecture & Design
### What does "file-based" mean in the context of CO-kit?
All data is stored as files without special filesystem requirements like locks, links or consistency levels.
Therefore you can store CO-kit managed files on local disk, cloud storages or object stores.
All files are stored verifiable using content addressing.

### How are COs structured and persisted?
COs consist of [Cores](../reference/core.md) which are data models. 
A CO is represented as a graph of content-addressed objects called a [DAG](../glossary/glossary.md#dag-cbor)(Directed Acyclic Graph).
To root of a CO is its DAG root which models all of its data.
The content-addressed objects are referenced by CIDs/Data pairs and stored into an storage backend.

For further information see:
- [CO](../reference/co.md)
- [Storage](../reference/storage.md)

### What storage backends are supported out-of-the-box?
A storage backend works as key/value store of content addressed blocks where the key is a CID and the value a binary described by this CID.
Currently, the following backends are supported:
- Filesystem: Basically uses CID as filename and the binary as file contents in a configurable folder.
- Memory: In memory hash map based CID/Binary structure.

For further information see:
- [Key Principles](../introduction/key-principles.md#file-based)
- [Storage](../reference/storage.md)

### How does the decentralized architecture of CO-kit ensure data integrity?
CO-kit uses a data graph per CO which we call the log.
The log is powered by a Merkle-CRDT. This deterministically orders transactions based on a logical clock.
Each piece of data is stored as a content addressed block which allows for cryptographic verification – at anytime.

For further information see:
- [Log](../reference/log.md)

### Can CO-kit be integrated with traditional centralized systems?
#todo #review
Yes. COs would be serving as an added layer of trust and security when in use in centralized systems. The library can be integrated anywhere and you can also build your backend/data models with CO-kit. 
Another useful scenario is CO-kit-built apps in processes that are fed back into traditional systems. In that case, COs can be stored and used as receipts (leveraging non-reputability).   


- Library can be integrated everywhere
- Can be used as backend/data
- Usable for processes that will be fed back into traditional systems
	- COs can be stored and used as receipt (leveraging non-reputability)
- Amplify the edge #question -> please explain

## Networking & Synchronization
### How does peer discovery work in CO-kit's networking model?
Local peer discovery is done by utilizing mDNS.
CO-kit got built in GossipSub protocol for peer discovery.
However the networking is entire optional and one could just use HTTP for transferring blocks.

### What networking mode should I use for local, LAN-only collaboration?
#todo 
By default, CO-kit uses mDNS for local device discovery.
For discovery over the internet, a bootstrap peer can be configured which defaults to bootstrap.1io.com. Technically, this bootstrap peer exposes a libp2p gossip-sub endpoint. You are free to choose any or no bootstrap peer(s) at all, or to use your own device as bootstrap for your other devices.
Both methods are used to discover new peers but they can be changed or disabled using configuration depending on your demands.

### Is it possible to run CO-kit without any network connectivity?
Yes.
It is possible to use CO-kit only with files.
We also require no strong feature set for file systems - just files.
As each file is Content addressed using its CID we just need to read and write them.
This even makes is possible to just use any cloud storage drive to share the CO-kit managed files.

### How does CO-kit handle NAT traversal and firewalls in P2P mode?
#todo 
This is handled through a variety of possibilities. First option is through a circuit relay. libp2p [defines a protocol called p2p-circuit](https://github.com/libp2p/specs/tree/master/relay). When a peer isn’t able to listen on a public address, it can dial out to a relay peer, which will keep a long-lived connection open. Other peers will be able to dial through the relay peer using a `p2p-circuit` address, which will forward traffic to its destination. 

Second option is by Direct Connection Upgrade through Relay [(DCUtR)](https://docs.libp2p.io/concepts/nat/dcutr/) by Hole Punching. It is a protocol for establishing direct connections between nodes through hole punching, without a signaling server. DCUtR involves synchronizing and opening connections to each peer’s predicted external addresses. 

For further information see: 
- https://docs.libp2p.io/concepts/nat/overview/

### What happens during a network partition or peer disconnect?
#todo 
Each peer continues locally by writing operations as a content addressed event to the log.
This may cause participants heads to diverge.
When a peer reconnects, the heads are shared again and the Log joins new heads, deterministically sorting all events to a consistent order which is then used to recalculate the state.

## Data & Consensus
### How are conflicts resolved using Merkle-CRDTs in CO-kit?
Conflicts are resolved by the [Log](../reference/log.md) which is sorting the stream of events by the event's logical clock.

### What are the trade-offs between different consensus modes (none, PoA, manual, shared)?
#todo
[Consensus](../reference/consensus.md).

### Can I define custom consensus logic for my application?
Yes. A consensus is implemented as a core with an additional guard.

### What level of schema validation or migration support does CO-kit offer?
Schema validation is up to the developer of [Core](../reference/core.md) and can be implemented as desired.
Cores can be migrated between versions. The migration itself is just another event which can be implemented in code.

### How does CO-kit manage partial data availability across distributed nodes?
As all data is represented as a graph, more precisely as a DAG (directed acyclic graph) the data is always accessed top-down, meaning we can fetch more data as we walk down the graph.
In addition, content addressing ensures validity of the data.
Distribution happens organically, but you can always opt to fetch all the data if needed.
