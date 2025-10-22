# CO-kit
A software development kit for building decentralized applications. It is written in Rust. 
CO-kit enables you to build data rooms called COs and integrate data models called COres. #review 

## CO-kit is like ...
CO-kit is like a database that combines blockchain's zero trust functionalities and (optional) consensus with git, CRDT, and then integrates with verifiable identity.

## CO-kit for ...

### ... Frontend developers 
- Like BaaS but for free
- Realtime database and collaboration
- Data which are used/produced on a device is available local automatically
- Built-in identity management through [DID](../reference/identity.md#What-is-a-DID)
- No special handling for files – they are just pieces of data which can be stored using [content addressing](../glossary/glossary.md#CID)
- Existing cores can be used in your app without a single line of code

### ... Backend developers
- [Cores](../reference/core.md) work like containers which contain the business logic and data but shareable over network
- Efficient automatic caching of data through content addressing
- Works offline without special handling
- No more managing database clusters
- Move latency off the critical path - make the client instant
- You still design the data model, indexing, and business rules - just in a distributed way as a core
- It can be integrated with current backends and APIs (REST, GraphQL etc.)

### ... Database nerds
- Cores are like databases with built-in atomicity, consistency, isolation and durability
- The Log with its resulting states are a MVCC
- CO-kit is like a database with Master-Master replication, where your business logic can decide what happens on conflicts, whereas with transactions only the database can.
	- With conventional databases only transactions/operations can be reapplied/reordered, but without knowing the business logic behind. Cores reapply the whole business logic when reordering a transaction.
- No need for an interface or API between data and business logic because it's one thing: a core

### ... Poets
- If your brain is really sore, start using our core. 🤓

