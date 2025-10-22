# Key Principles

## Local First
Data is stored locally on the users end device and is only shared with the group of people with whom they are currently collaborating, encapsulated from all other users, creating their own private platform. The number of private platforms that can be created is not limited.

## Bring your own infrastructure
Peer-to-Peer is the go to mechanism used for networking, but there is no lock-in to a single networking strategy and you can configure it to your needs.

### File-based
All data is stored as files without special filesystem requirements like locks, links or consistency levels.
Therefore you can store CO-kit-managed files on local disk, cloud storages or object stores.
All files are stored verifiable using content addressing.

### Peer-to-Peer
Only the necessary data is exchanged in end-to-end encrypted form using peer-to-peer technology. There are only users, and no central servers as a middleman.

## Instant updates
#todo
#question Is this the same as Partial Real Time Sync?

## Multiplayer
#todo

## Real time sync
#todo
Changes are distributed in real time.
They are pushed directly to all or configured participants.
Only the Log heads are distributed and participants can fetch the referenced data on demand and in parallel.

## Partial data
#todo
No full replica of all data needed.
Content addressing allows for validated on demand fetching.

## Interoperability
Tools that build on Co-kit should be compatible with each other, even if the builders are not partners. Interoperability between cool solutions is already built in.

## Accessibility
We want everyone to be able to take full advantage of the platform and derive value from it, not just developers. Not only shall the applications built with CO-kit be easy to use, but we also hope for the best developer experience possible.

## Self-Sovereign
#todo
No need to rely on big corporates to provide storage space or cloud solutions. Co-kit enables you to build without dependencies and lets you choose (if needed) providers independently. 

## Open & Democratic
No one should ever own a platform and exploit others through their data. We see CO-kit as public property. Everything is open-source. Go and build a better digital future with it. 

See more in the chapter [How to contribute](./development/how-to-contribute.md)
See more in the chapter [Licensing](./license/lega-notice.md).
