# FAQ

<!-- toc -->

## General
### What is CoKit and what are its primary use cases?
#todo

### Is CoKit open-source? Under what license is it distributed?
#todo

### What platforms does CoKit support? (e.g., Linux, Windows, macOS, etc.)
#todo

### How does CoKit differ from other CRDT or P2P-based frameworks?
#todo

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

### What networking mode should I use for LAN-only collaboration?
#todo

### Is it possible to run CoKit without any network connectivity?
Yes.
#todo

### How does CoKit handle NAT traversal and firewalls in P2P mode?
#todo

### What happens during a network partition or peer disconnect?
#todo


## Data & Consensus
### How are conflicts resolved using Merkle-CRDTs in CoKit?
#todo

### What are the trade-offs between different consensus modes (none, PoA, manual, shared)?
#todo

### Can I define custom consensus logic for my application?
#todo

### What level of schema validation or migration support does CoKit offer?
#todo

### How does CoKit manage partial data availability across distributed nodes?
#todo
