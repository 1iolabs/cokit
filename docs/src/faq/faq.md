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
#todo #text
- End-to-end encryption
- End-to-end verification
	- Cryptographic integrity verification
	- Cryptographic identity verification
- Identity
- Auditable and non-repudiable history
- Zero-trust
- Decentralized
- Private as not even encrypted data is shared with unknown peers

### Can CO-kit be used offline and sync later?
#todo #text
Yes.
- All changes happen only local and are eventually distributed to other participants.
- Every piece of data is content addressed and can easily synced between peers.
- Consensus is possibly blocked when too many peers are offline. But that is nor a issue:
	- Depends on CO setup
	- Optional
	- On demand
	- Don't blocks to continue
- See [[#What happens during a network partition or peer disconnect?]]?

## Architecture & Design
### What does "file-based" mean in the context of CO-kit?
All data is stored as files without special filesystem requirements like locks, links or consistency levels.
Therefore you can store CO-kit managed files on local disk, cloud storages or object stores.
All files are stored verifiable using content addressing.

### How are COs structured and persisted?
#todo #review
COs are consists of Cores which are data models used within COs.
A CO is represented as a graph of content addressed objects called a DAG.
To root of a CO is its DAG root which model all of its data.
The content addressed objects are referenced by CIDs/Data pairs and stored into an storage backend.

For further information see:
- [CO](../reference/co.md)
- [Storage](../reference/storage.md)

### What storage backends are supported out-of-the-box?
#todo #review
A storage backend works as key/value store of content addressed blocks. Where the key is a CID and the value a binary described by this CID.
Currently following backends are supported:
- Filesystem: Basically uses CID as filename and the binary as file contents in a configurable folder.
- Memory: In memory hash map based CID/Binary structure.

For further information see:
- [Key Principles](../introduction/key-principles.md#file-based)
- [Storage](../reference/storage.md)

### How does the decentralized architecture of CO-kit ensure data integrity?
#todo #review
CO-kit uses a data graph per CO which we call the log.
The log is powered by a Merkle-CRDT which deterministically orders transactions based on a logical clock.
Each piece of data is stored as a content addressed block which allows for cryptographic verification - anytime.

For further information see:
- [Log](../reference/log.md)

### Can CO-kit be integrated with traditional centralized systems?
#todo #text
Yes.
- Library can be integrated everywhere
- Can be used as backend/data
- Usable for processes that will be fed back into traditional systems
	- COs can be stored and used as receipt (leveraging non-reputability)
- Amplify the edge

## Networking & Synchronization
### How does peer discovery work in CO-kit's networking model?
Local peer discovery is done by utilizing mDNS.
CO-kit got built in GossipSub protocol for peer discovery.
However the networking is entire optional and one could just use HTTP for transferring blocks.

### What networking mode should I use for local, LAN-only collaboration?
#todo #review
By default CO-kit uses mDNS for local device discovery.
For discovery over the internet a bootstrap peer can be configured which defaults to bootstrap.1io.com. Technically this bootstrap peer exposes a libp2p gossip-sub endpoint. You a free to choose any or none bootstrap peer(s). Or use your own device as bootstrap for your other devices.
Both methods are used to discover new peers but they can be changed or disabled using configuration as you like.

### Is it possible to run CO-kit without any network connectivity?
Yes.
It is possible to use CO-kit only with files.
We also require no strong feature set for file systems - just files.
As each file is Content addressed using its CID we just need to read and write them.
This even makes is possible to just use any cloud storage drive to share the CO-kit managed files.

### How does CO-kit handle NAT traversal and firewalls in P2P mode?
#todo #text
- Circuit Relay
- Direct Connection Upgrade through Relay
- Hole Punching
- https://docs.libp2p.io/concepts/nat/overview/

### What happens during a network partition or peer disconnect?
#todo #review
Each peer continues locally by writing operations as a content addressed event to the log.
This causes participants heads to may diverge.
When some peer reconnects the heads are shared again and the Log joins new heads, deterministically sorts all events to a consistent order which is then used to recalculate the state.

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
