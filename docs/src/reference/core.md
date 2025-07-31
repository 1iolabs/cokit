# Core
A core (**CO** **re**ducer) is a piece of data that acts like a state. Cores can be directly added to COs and they work like an in-code database. They implement a reducer function that take actions which have been pushed to a CO. The reducer then changes the cores data accordingly.

### Technical Notes
- A core can be easily serialized (we use cbor) and saved
- Serialization yields a CID which can then be used to reference that data (via a log or in other states for example)
- The serialized data can then be stored on the hard drive or sent to other participants

## Migrations


### Builtin cores
We provide a set of cores. They are the following:

#### co-core-co
Root core which manages the COs core instances, guards and participants.

#### co-core-keystore
Stores credientials.
Used internally to store DID and PeerID private keys.

#### co-core-membership
Stores memberships informations of COs.
Used internally in the Local CO to track which COs one of out identities is a member of.

#### co-core-board
Kanban board core.
Used internally to coordinate pending network requests.

#### co-core-storage
Stores reference informations about storage blocks which exist on disk.
Used internally to free blocks from disk which are not referenced anymore.

#### co-core-poa
Proof-of-authority core.
Provides POA [consensus](./sdk-components/consensus.md) for a CO.

#### co-core-room
Messaging core. Stores messages in matrix compatible format.

#### co-core-file
Stores hierarchical file structures like a file system.

#### co-core-data-series
Stores data series, counters and aggregations on the data.

#### co-core-role
#todo

#### co-core-pin
#todo
