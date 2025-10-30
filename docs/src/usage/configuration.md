# Configuration

## CO
Within COs, the following parameters can be configured.

### Tags
The tags field of the CO can be used for metadata and configurations.

For further information see:
- [co-core-co: Co: tags](/crate/co_core_co/struct.Co.html#structfield.tags)

#### Invite
Invite settings when receiving a new invite.
- Scope: [Local CO](../reference/co.md#local-co)
- Settings
	- `manual`: Add as “pending” membership.
	- `disable`: Reject all Invite requests.
	- `accept`: Auto accept all Invite requests.
	- `did`: Only accept invite when DID can be verified for certain properties.

For further information see:
- [co-primitives: CoInvite](/crate/co_primitives/enum.CoInvite.html)

#### Join
Join settings of a shared CO.
- Scope: [Private CO](../reference/co.md#private-co), [Public CO](../reference/co.md#public-co)
- Settings
	- `invite`: Only accept joins when participant has been invited.
	- `accept`: Auto-accept all join requests.
	- `did`: Only accept join when DID can be verified for certain properties.
	- `manual`: Add "pending" participant.

For further information see:
- [co-primitives: CoJoin](/crate/co_primitives/enum.CoJoin.html)

### Network
Network configurations for connectivity in a CO using various protocols.

For further information see:
- [Network Configuration](../reference/network.md#network-configuration)


## Application
Application-specific configurations available to app developers leveraging CO-kit.

### Network
When starting the network, it's possible to pass startup options.

For further information see:
- [co-sdk: NetworkSettings](/crate/co_sdk/struct.NetworkSettings.html)
- [co-sdk: Application: create_network](/crate/co_sdk/struct.Application.html#method.create_network)

#### Force New Peer ID
Force to create a new PeerID before network startup.
This can be used to improve privacy so users can't be tracked across application starts.

#### Listen
Endpoint to listen to.
Defaults to `/ip4/0.0.0.0/udp/0/quic-v1`, this uses all available interfaces and a random port with QUIC connectivity.

#### Bootstrap
Bootstrap endpoints to improve connectivity.
Defaults to `dns/bootstrap.1io.com/udp/5000/quic-v1/p2p/12D3KooWCoAgVrvp9dWqk3bds1paFcrK8HuYB8yY13XWaahwfm7o`.
