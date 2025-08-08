# CO
A CO is a virtual room for collaboration.
CO stands for collaborative object and is a fundamentally new concept of distributed collaboration, because a CO is not just another group chat of sorts.
It rather serves a multitude of functionalities in a distributed network while running locally on each participants device.

## What makes a CO
A CO is like a bucket or project for digital data, which can be used for collaboration and communication.
COs can serve for various digital processes without the requirement for expensive setups.
COs are designed to be lightweight, ad-hoc usable and cheap to create, enabling the use of thousands or even millions of them.
Each container is unique, much like a receipt, making it ideal for granular, trackable, and disposable data operations at scale.
Like a database that runs for every participant but locally.

## Structure

### Cores
Each CO contains at least one [Core](../reference/core.md).
Cores are the data model of which COs are composed.

### Participants
Each CO contains participants.
They are identified through their DID.
It is important to notice that a CO can contain zero participants as well as millions of participants.

### Network settings
Each CO may contain networking settings.
With there settings the connectivity of the CO can be configured.
As a default [didcontact](../glossary/glossary.md#didcontact) with any of the participants are used.

### Encryption settings
Each CO can be encrypted.
The encryption can be set while creating a CO, making it either unencrypted or secure it using a specific encryption algorithm.
Encryption keys are stored in the Local COs Key-Store core.
The encryption keys can be versioned.
Once a new version is created that key is used for new data.
This allows advanced sharing patterns, like, allowing new participants to only see data produced after they joined the CO.

## Types

Overview:

| CO           | Local                             | Private                           | Public                            | Personal                          |
| ------------ | --------------------------------- | --------------------------------- | --------------------------------- | --------------------------------- |
| Encryption   | <input type="checkbox" style="pointer-events: none;" checked /> | <input type="checkbox" style="pointer-events: none;" checked /> | <input type="checkbox" style="pointer-events: none;" />         | <input type="checkbox" style="pointer-events: none;" checked /> |
| Networking   | <input type="checkbox" style="pointer-events: none;" />         | <input type="checkbox" style="pointer-events: none;" checked /> | <input type="checkbox" style="pointer-events: none;" checked /> | <input type="checkbox" style="pointer-events: none;" checked /> |
| Syncing      | <input type="checkbox" style="pointer-events: none;" />         | <input type="checkbox" style="pointer-events: none;" checked /> | <input type="checkbox" style="pointer-events: none;" checked /> | <input type="checkbox" style="pointer-events: none;" checked /> |
| Participants | <input type="checkbox" style="pointer-events: none;" />         | <input type="checkbox" style="pointer-events: none;" checked /> | <input type="checkbox" style="pointer-events: none;" checked /> | <input type="checkbox" style="pointer-events: none;" />         |

### Local CO
The Local CO is the the device's local-only CO.
It manages the device's or application's local state which forms the root entry point.
It exists once for each unique storage (memory or filesystem path), and is always encrypted.
The root encryption key is stored in the OS keychain from this key all other keys used in the Local CO will be derived.
The application's local state include:
- Identities
- Memberships of COs
- Encryption keys of COs
- Device Settings
- Network PeerID

### Private CO
A CO which can only accessed by participants.
While setting up a Private CO, the initiator decides about permissions, cores and who can access it by adding the DIDs of the desired participants.
All participants with the permissions can do that at a later stage as well.
Like a private chat between two participants.

### Public CO
A CO that is unencrypted and open to be read by anyone.
While setting up a Public CO, the initiator decides about permissions and cores.
All participants with the permissions can do that at a later stage as well.
Like a public group chat.

### Personal CO
Acts like a private CO but usually once accessible by the creator.
Stores creators, identities and settings, so those can be synced between devices[^issue-82].


[^issue-82]: https://gitlab.1io.com/1io/co-sdk/-/issues/82
