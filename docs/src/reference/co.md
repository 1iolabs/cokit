# CO
A CO is a virtual room for collaboration.
CO stands for collaborative object and is a fundamentally new concept of distributed collaboration, because a CO is not just another group chat of sorts.
It rather serves a multitude of functionalities in a distributed network while running locally on each participants device.

## Structure

### Cores
Each CO contains at least one [Core](/reference/core.md).

### Participants
Each CO contains participants (DID).

### Network settings
Each CO may contain networking settings.

### Encryption settings
Each CO may be encrypted.

## Types

### Local CO
The Local CO is the the device local only CO which manages application local state:
- Memberships of COs
    - Memberships are normally stored inside the DIDs Personal CO.
    - But we need to store membership to the Personal COs of the users identities.
- Encrypted keys of COs
- Identities
- Device Settings
- PeerID

### Private CO
Private CO which only allows read access to participants. While setting up a Private CO, the admin decides who can access by adding the DIDs of the desired participants to the CO settings.

### Public CO
A CO that runs unencrypted on the participants devices. As with private COs the runtime defines the participant permissions (read, write, ...). The participants are responsible for possible backups and edge computing services, meaning they decide for themselves how much storage/bandwidth to buy. (if necessary) Public COs can be resolved via a Public CO Registry

### Personal CO
Stores identities and settings of one actual user (may spanning over multiple DIDs) in a Private CO so it can be synced between devices.
- Personal CO can be used to save settings that should be shared privately over all devices
- Settings are synced between all devices
