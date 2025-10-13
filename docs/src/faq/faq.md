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
- We use APLGv3 as the license of choice [see](../license/legal-notice.md)

### What platforms does CO-kit support? (e.g., Linux, Windows, macOS, etc.)
- CO-kit is fully platform agnostic and can be supported by any OS

### How does CO-kit differ from other CRDT or P2P-based frameworks?
#todo #tech

### Can CO-kit be used offline and sync later?
Yes. ENTER EXPLANATION
#todo #tech


## Architecture & Design
### What does "file-based" mean in the context of CO-kit?
All data is stored as files without special filesystem requirements like locks, links or consistency levels.
Therefore you can store CO-kit managed files on local disk, cloud storages or object stores.
All files are stored verifiable using content addressing.

### How are COs structured and persisted?
A CO is represented as a graph of content addressed objects.
#todo #tech

### What storage backends are supported out-of-the-box?
Filesystem and Memory.
#todo #tech

### How does the decentralized architecture of CO-kit ensure data integrity?
CO-kit uses a data graph per CO which we call the log. The log is powered by a Merkle-CRDT which deterministically orders transactions based on a logical clock.
#todo

### Can CO-kit be integrated with traditional centralized systems?
Yes.
#todo


## Networking & Synchronization
### How does peer discovery work in CO-kit's networking model?
Local peer discovery is done by utilizing mDNS.
CO-kit got built in GossipSub protocol for peer discovery.
However the networking is entire optional and one could just use HTTP for transferring blocks.

### What networking mode should I use for LAN-only collaboration?
#todo #tech

### Is it possible to run CO-kit without any network connectivity?
Yes.
It is possible to use CO-kit only with files.
We also require no strong feature set for file systems - just files.
As each file is Content addressed using its CID we just need to read and write them.
This even makes is possible to just use any cloud storage drive to share the CO-kit managed files.

### How does CO-kit handle NAT traversal and firewalls in P2P mode?
#todo #tech

### What happens during a network partition or peer disconnect?
#todo #tech


## Data & Consensus
### How are conflicts resolved using Merkle-CRDTs in CO-kit?
Conflicts are resolved by the [Log](../reference/log.md) which is sorting by the event's logical clock.

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
