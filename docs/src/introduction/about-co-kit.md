# COkit
A software development kit for building decentralized applications. COkit is written in Rust.
COkit enables you to build data rooms called COs using data models called COres.
## COkit is like ...
...a database that combines blockchain's zero trust functionalities and (optional) consensus with git, CRDT, and then integrates with verifiable identity.

## COkit for ...

### ... Frontend Developers
- Like Backend as a Service (BaaS) but for free
- Realtime database and collaboration
- Data used/produced on a device is locally-available automatically
- Built-in identity management through [DID](../reference/identity.md#what-is-a-did)
- No special handling for files – they are just pieces of data which can be stored using [content addressing](../glossary/glossary.md#cid)
- Existing [Cores](../reference/core.md) can be used in your app without a single line of code

### ... Backend Developers
- [Cores](../reference/core.md) work like containers that contain the business logic and data, but shareable over networks
- Efficient automatic caching of data through [content addressing](../glossary/glossary.md#cid)
- Works offline without special handling
- No more managing database clusters
- Move latency off the critical path - make the client instant
- You still design the data model, indexing, and business rules - just in a distributed way as a Core
- Your COkit-built app can be integrated with current backends and APIs (REST, GraphQL etc.)

### ... Database Nerds
- [Cores](../reference/core.md) are like databases with built-in atomicity, consistency, isolation and durability
- The [Log](../reference/log.md) with its resulting states, is a Multiversion concurrency control
 (MVCC)
- COkit is like a database with Master-Master replication, where your business logic can decide what happens on conflicts.
  - With transactions only the database can make decisions upon conflicts.
	- With conventional databases only transactions/operations can be reapplied/reordered, but without knowing the business logic behind. Cores reapply the whole business logic when reordering a transaction.
- No need for an interface or API between data and business logic because it's one thing: a Core

### ... Poets
- If your brain is really sore, start using our Core! 🤓
