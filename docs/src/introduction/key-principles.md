# Key Principles

## Local First
Data is stored locally on the user's end device, and is only shared among those people with whom the user is currently collaborating. The data from these collaborating users is encapsulated within this group, creating a private platform. The number of private platforms that can be created is not limited. Private platforms can also be connected to one another, and we envision a whole ecosystem of platforms at a later stage.

## BYO infrastructure
Peer-to-Peer is the go-to mechanism used for networking with COkit, but there is no lock-in to a single networking strategy. You are free to configure the networking to your own infrastructure requirements.

### File-based
All data is stored as files without special filesystem requirements like locks, links or consistency levels.
Therefore you can store COkit-managed files on local disks, cloud storages or object stores.
All stored files are verifiable using [content addressing](../glossary/glossary.md#cid).

### Peer-to-Peer
Only the necessary data is exchanged, and it is end-to-end encrypted.
By default there are only users, and no central servers as middlemen.

## Instant updates
Changes are distributed in real time.
They are pushed directly to all users or to configured participants.
Only the Log heads are distributed – participants can fetch the referenced data on demand and in parallel.

## Partial data
No full replica of all data needed.
[Content addressing](../glossary/glossary.md#cid) allows for validated, on-demand fetching.

## Interoperability
Interoperability is built-in. Tools built with COkit are inherently compatible with each other, even if the builders are not partners.

## Accessibility
We want to empower everyone, not just developers, to take full advantage of the platform and derive value from it. Not only shall the applications built with COkit be easy to use, but we are striving to provide best developer experience possible.
This is why we're also working on a low/no code version of COkit. (WIP)

## Self-Sovereign
No need to rely on big corporates to provide storage space or cloud solutions.
COkit enables you to build without cloud-dependencies and lets you choose providers independently (if you even need any).

## Open & Democratic
No one should ever own a platform and exploit others through their data.
We see COkit as public property.
Everything is open-source. Go and build a better digital future with it.

For further information see:
- [How to contribute](../development/how-to-contribute.md)
- [Legal Notice](../license/legal-notice.md)
