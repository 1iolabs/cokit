# FAQ

<!-- toc -->

## General
### What is CoKit and what are its primary use cases?
#todo
- Collaborative Applications with focus on usability and data-ownership for the users.
- 
### Is CoKit open-source? Under what license is it distributed?
#todo
- Yes, we use APLGv3 [see](/license/legal-notice.md)

### What platforms does CoKit support? (e.g., Linux, Windows, macOS, etc.)


#todo

### How does CoKit differ from other CRDT or P2P-based frameworks?
#todo
- 

### Can CoKit be used offline and sync later?
Yes.
#todo


## Architecture & Design
### What does "file-based" mean in the context of CoKit?
All data is stored as files without special filesystem requirements like locks, links or consistency.
Therefore you can store co-kit managed files on local disk, cloud storages or object stores.
All files are stored verifiyable using content addressing.

### How are COs structured and persisted?
A CO is represented as a graph of content addressed objects.
#todo

### What storage backends are supported out-of-the-box?
Filesystem and Memory.
#todo

### How does the decentralized architecture of CoKit ensure data integrity?
CO-KIT uses a data graph per CO which we call the Log. The log is powered by an Merkle-CRDT which deterministically orders transactions based on a logical clock.
#todo

### Can CoKit be integrated with traditional centralized systems?
Yes.
#todo


## Networking & Synchronization
### How does peer discovery work in CoKit's networking model?
#todo
Local peer discovery is done by utilising mDNS.
Co-Kit got built in GossipSub protocol for peer discovery.
However the networking is entire optional and one could just use HTTP for transferring blocks.


### What networking mode should I use for LAN-only collaboration?
#todo

### Is it possible to run CoKit without any network connectivity?
Yes. 
It is possible to use CO-KIT only with files.
We also require no strong feature set for file systems - just files.
As each file is Content addressed using its CID we just need to read and write them.
This even makes is possible to just use any cloud storage drive to share the co-kit managed files.

### How does CoKit handle NAT traversal and firewalls in P2P mode?
#todo

### What happens during a network partition or peer disconnect?
#todo


## Data & Consensus
### How are conflicts resolved using Merkle-CRDTs in CoKit?
Conflicts are resolved by sorting by the event's logical clock.
See more at the [Log](/reference/sdk-components/log.md).

### What are the trade-offs between different consensus modes (none, PoA, manual, shared)?
#todo
See more at [Consensus](/reference/sdk-components/consensus.md).

### Can I define custom consensus logic for my application?
Yes. A consensus is implemented as a core with an additional guard.

### What level of schema validation or migration support does CoKit offer?
Schema validation is up to the developer of core and can be implemented as desired.
Cores can be migrated between versions. The migration itself is just another event which can be implemented in code.
See more at [Core](/reference/core.md).

### How does CoKit manage partial data availability across distributed nodes?
As all data is represented as a graph more precisely as a DAG (directed acyclic graph) the data is always accessed, top-down meaning we could fetch more data as we walk the graph down.
In addition, content, addressing ensure validity of the data.
So distribution happens organically, but you can always, for example just fetch all the data.


