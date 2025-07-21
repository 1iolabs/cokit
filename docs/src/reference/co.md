# CO
## Abstract 
A CO is a virtual room for collaboration.

CO (virtual data room) is a distributed database whose data is encrypted and is only available to the participants (unique via DID) of the data room. The CO stores references (unique via CID) of the data. The data itself is stored on the participants' devices. The DIDs, permissions and the identities of the participants (PrivateKeys) are stored in a data structure (data structure for states) “COre”. Each CO contains at least one COre. They act as “in-code databases” that store details such as the DIDs of the participants in a CO, their roles (admin, reader, etc.), permissions and status information (states) of systems such as chat rooms.

## Features
- Stores references
- Stores identities
- Stores participants
- Stores permissions

## Encryption
Each Storage data item gets an own encryption key which is encrypted with the versioned CO encryption key. This mechanism is used to share files across other CO (or versions) and prevents for the need to re-encrypt large files by only have to re-encrypt single data item keys.

## Types

#### Local CO
The Local CO is the the device local only CO which manages application local state:

- Memberships of COs
    - Memberships are normally stored inside the DIDs Personal CO.
    - But we need to store membership to the Personal COs of the users identities.
- Encrypted keys of COs
- Identities
- Device Settings
- PeerId (Public and private)

#### Personal CO
Stores identities and settings of one actual user (may spanning over multiple DIDs) in a Private CO so it can be synced between devices.

- Personal CO can be used to save settings that should be shared privately over all devices
- Settings are synced between all devices

#### Public CO
A CO that runs unencrypted on the participants devices. As with private COs the runtime defines the participant permissions (read, write, ...). The participants are responsible for possible backups and edge computing services, meaning they decide for themselves how much storage/bandwidth to buy. (if necessary) Public COs can be resolved via a Public CO Registry

#### Private CO
Private CO which only allows read access to participants. While setting up a Private CO, the admin decides who can access by adding the DIDs of the desired participants to the CO settings.