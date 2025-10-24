# Configuration

## CO
Within COs, the following parameters can be configured.

### Tags
The tags field of the CO can be used for metadata and configurations.

For further information see:
- [co-core-co: Co: tags](/crate/co_core_co/struct.Co.html#structfield.tags)

#### Invite
Invite settings when receive a new invite.
- Scope: Local CO
- Settings
	- `manual`: Add as “pending” membership.
	- `disable`: Reject all Invite requests.
	- `accept`: Auto accept all Invite requests.
	- `did`: Only accept invite when DID can be verified for certain properties.

For further information see:
- [co-primitives: CoInvite](/crate/co_primitives/enum.CoInvite.html)

#### Join
Join settings of a shared CO.
- Scope: CO
- Settings
	- `invite`: Only accept joins when participant has been invited.
	- `accept`: Auto Accept all Join requests.
	- `did`: Only accept join when DID can be verified for certain properties.
	- `manual`: Add "pending" participant.

For further information see:
- [co-primitives: CoJoin](/crate/co_primitives/enum.CoJoin.html)

### Network
Network configurations for connectivity in a CO using various protocols.

For further information see:
- [Network Configuration](../reference/network.md#network-configuration)

