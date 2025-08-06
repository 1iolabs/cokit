# CO
A CO is a virtual room for collaboration.
CO stands for collaborative object and is a fundamentally new concept of distributed collaboration, because a CO is not just another group chat of sorts.
It rather serves a multitude of functionalities in a distributed network while running locally on each participants device.

## Structure

### Cores
Each CO contains at least one [Core](/reference/core.md).
Cores are the data model of which COs are composed.

### Participants
Each CO contains participants.
They are identified through their DID.
It is important to notice that a CO can contain zero participants as well as millions of participants.
The CO initiator is by default, equipped with all permissions

### Network settings
Each CO may contain networking settings.
With there settings the connectivity of the CO can be configured.
As a default [didcontact](/glossary/glossary.md#didcontact) with any of the participants are used.

### Encryption settings
Each CO can be encrypted.
The encryption can be set while creating a CO, making it either unencrypted or secure it using a specific encryption algorithm.
Encryption keys are stored in the Local COs Key-Store core.
The encryption keys can be versioned.
Once a new version is created that key is used for new data.
This allows advanced sharing patterns, like, allowing new participants to only see data produced after they joined the CO.

## Types

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
Like a private chat between two participants.

### Public CO
#todo
A CO that runs unencrypted on the participants devices. As with private COs the runtime defines the participant permissions (read, write, ...). The participants are responsible for possible backups and edge computing services, meaning they decide for themselves how much storage/bandwidth to buy. (if necessary) Public COs can be resolved via a Public CO Registry

### Personal CO
#todo
Stores identities and settings of one actual user (may spanning over multiple DIDs) in a Private CO so it can be synced between devices.
- Personal CO can be used to save settings that should be shared privately over all devices
- Settings are synced between all devices
