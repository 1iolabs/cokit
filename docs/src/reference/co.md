# CO
A CO is a virtual room for collaboration.  
CO stands for Collaborative Object and is a fundamentally new concept of distributed collaboration.  
A CO is not just another group chat of sorts. Rather, a CO serves a multitude of functionalities in a distributed network, all while running locally on each participant's device.

## What makes a CO
A CO is like a bucket or project for digital data, which can be used for collaboration and communication.  

COs can serve a multitude of digital processes without the requirement for expensive setups.  
They are designed to be lightweight, ad-hoc-usable and cheap to create, enabling the use of thousands or even millions of them.

Each CO(ntainer😉) is unique, much like a receipt, making it ideal for granular, trackable, and disposable data operations at scale.  
Imagine it like a database that runs for every participant but locally.

## Structure

### Cores
Each CO contains at least one [Core](../reference/core.md).  
Cores are the data model of which COs are composed and are explained in the following chapter.

### Participants
Each CO can contain participants.  
They are identified through their DID - whether they are humans or machines (like an IoT device, or a piece of infrastructure).  
It is important to notice that a CO can contain zero participants as well as millions of participants.

### Network settings
Each CO may contain networking settings.  
With these settings the connectivity of the CO can be configured.  
By default, the [didcontact](../glossary/glossary.md#didcontact) protocol is used for the participants.

For further information see:
- [Network](../reference/network.md#network-configuration)

### Encryption settings
Each CO can be encrypted.  
The encryption can be set while creating a CO, making it either unencrypted or secured by a specific encryption algorithm.  

Encryption keys are stored in the [Local COs](../reference/co.md#local-co) [`keystore`](../reference/core.md#co-core-keystore) core.  
These encryption keys can be versioned.  
When a new version is created, this new version is used for new data.  
This allows advanced sharing patterns, such as allowing new participants to only see data produced _after_ they joined the CO.

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
It manages the device's or application's local state, which forms the root entry point.  
It exists once for each unique storage (memory or filesystem path), and is always encrypted.  
The root encryption key is stored in the OS keychain.  
From this key all other keys used in the Local CO will be derived.

The application's local state includes:
- Identities
- Memberships of COs
- Encryption keys of COs
- Device Settings
- Network PeerID

### Private CO
A CO that can only be accessed by participants.  
While setting up a Private CO, the initiator decides on permissions, cores, and who can access it by adding the DIDs of the desired participants.  
All participants with the requisite permissions can also do this at a later stage.  
Think of it like a private chat between participants.

### Public CO
A CO that is unencrypted and open to be read by anyone.  
While setting up a Public CO, the initiator decides on permissions and cores.  
All participants with the requisite permissions can also do this at a later stage.  
Think of it like a public group chat.

### Personal CO
Acts like a Private CO, but is typically only accessible by the creator.  
Stores creators, identities and settings, so that these can be synced between devices[^issue-82].  
Similar to a wallet.

[^issue-82]: [Personal CO (#82)](https://gitlab.1io.com/1io/co-sdk/-/issues/82)
