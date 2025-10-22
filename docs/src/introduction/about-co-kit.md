# CO-kit
A software development kit to build decentralized applications with, written in Rust.

## CO-kit is like ...
CO-kit is like a database that combines blockchain's zero trust functionalities and (optional) consensus with git, CRDT, and then integrates with verifiable identity.

## CO-kit for ...

### ... Frontend developers 
#todo #review 
- Like BaaS but for free
- Realtime database and collaboration
- Data which are used/produced on a device is automatically available locally
- Builtin identity management using DID
- No special handling for files as they are just data which can be stored using content addressing
- Existing cores can be used in your app without a single line of code

### ... Backend developers
#todo #review 
- Cores work like containers which contain the business logic and data but shareable over network
- Efficient automatic caching of data through content addressing
- Works offline without special handling
- No more managing database clusters
- Move latency off the critical path - make the client instant
- You still design the data model, indexing, and business rules - just in a distributed way as a core
- It can be integrated with current backends and APIs (REST, ...)

### ... Database nerds
#todo #review 
- Cores are like databases with builtin atomicity, consistency, isolation and durability
- The Log with its resulting states are a MVCC
- CO-kit is like a database with Master-Master replication, where your business logic can decide what happens on conflicts, whereas with transactions only the database can.
	- With classical databases only transactions/operations can be reapplied/reordered, but without knowing the business logic behind. Cores reapply the whole business logic when reordering a transaction.
- No need for an interface or API between data and business logic because it's one thing: a core

### ... Poets
#todo #review 
- If your brain is really sore, start using our core. 🤓

