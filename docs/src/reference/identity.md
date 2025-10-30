# Identity
CO-kit uses a DID (Decentralized Identifier) as the fundamental identifier for identities. A DID is structured to align with the [W3C DID Core specification](https://www.w3.org/TR/did-1.0/).

## What is a DID
A DID is a globally unique identifier. Using DID ensures that each participant is cryptographically verifiable and self-sovereign.
They provide a foundation of trust as an entirely new layer of decentralized digital identity and PKI (public key infrastructure) for the Internet.

Depending on the method, it doesn't rely on a centralized authority and can be enriched with verifiable credentials like a government-issued ID.

DIDs are typically of the form: `did:<method>:<method-specific-identifier>`

For example: `did:example:alice123`.

## Why CO-kit uses DID
In CO-kit, every interaction is signed by a DID to make everything cryptographically verifiable.

Because a DID is decentralized, self-sovereign, flexible, cryptographically verifiable and self-administered, we don't need any other identity mechanism.
When building an application, developers can extend supported DID methods.

DIDs can be created on a per-relationship or per-service basis, giving you potentially thousands of pairwise-unique identifiers that cannot be correlated without your consent.
This approach makes it harder for services and participants to track you across contexts, since each one sees a different identifier.
So rather than a participant having only a single DID (like a cell number, or national ID number), participants may have thousands of DIDs, yet can still manage them easily using CO-kit.

## How CO-kit uses DID
DIDs appear in almost every part of CO-kit since every interaction is signed by a DID. Here's how DIDs are handled in all of CO-kit.

### CO Participants
Every participant has at least one DID. A participant can be human or machine (service, device, IOT-device, AI-agent, ...).
Within a CO, participants are represented by their DID.
This relation can be extended with attributes for permission-related settings or to distinguish between participants.

### Wallet
You can save your DIDs in a wallet.
This is implemented by the [`co-core-keystore`](/crate/co_core_keystore/index.html) [Core](../reference/core.md), which is normally used in the [Local CO](../reference/co.md#local-co).
You are free to add the core to another CO, for example to share identities between your devices[^issue-82].

### Networking
To discover, identify and communicate with participants within a network, DID's are used and resolved in using the [didcomm](../reference/network.md#protocol-didcomm) and [didcontact](../reference/network.md#protocol-didcontact) protocols.
By network, by default participants of a CO are meant, but this can be [configured](../reference/co.md#network-settings) on a by-CO basis.

### Log
In the [Log](../reference/log.md), each event in the conflict-free replicated event stream is signed by the DID of CO participants.

## Usage
This example shows how to create a new `did:key:` identity using CO-kit:

```rust
use co_sdk::{Application, DidKeyIdentity, DidKeyProvider, CO_CORE_NAME_KEYSTORE};

async fn create_identity(application: Application, name: String) -> Result<DidKeyIdentity, anyhow::Error> {
    let identity = DidKeyIdentity::generate(Default::default());
    let local_co = application.local_co_reducer().await?;
    let provider = DidKeyProvider::new(local_co, CO_CORE_NAME_KEYSTORE);
    provider.store(&identity, Some(name)).await?;
    Ok(identity)
}
```

## A brief history of addresses
Here is a table borrowed from [ssimeetup.org](https://ssimeetup.org/)[^1] that showcases where DID fits in:

| Origin      | Address Type             | Network                                    |
| ----------- | ------------------------ | ------------------------------------------ |
| Pre-history | Human name               | Human networks (family, clan, tribe, etc.) |
| ~1750       | Postal address           | Postal mail network                        |
| 1879        | Telephone number         | Telephone network                          |
| 1950        | Credit card number       | Payment network                            |
| 1964        | Fax number               | Fax (facsimile) network                    |
| 1971        | Email address            | Email network                              |
| 1974        | IP address               | Internet (machine-friendly)                |
| 1983        | Domain name              | Internet (human-friendly)                  |
| 1994        | Persistent address (URN) | World Wide Web (machine-friendly)          |
| 1994        | Web address (URL)        | World Wide Web (human-friendly)            |
| 2003        | Social network address   | Social network                             |
| 2009        | Blockchain address       | Blockchain or distributed ledger network   |
| 2016        | DID                      | DID network (aka trust network)            |

## A brief history and comparision of identity systems
With the evolution of networks, we have seen two major identity systems in use.
DID is serves as the solution to amplify future identity management needs.

|  | Centralized Identity  | Federated Identity | Decentralized Identity |
| - | - | - | - |
| Security | <input type="checkbox" style="pointer-events: none;" /> | <input type="checkbox" style="pointer-events: none;" /> | <input type="checkbox" style="pointer-events: none;" checked /> |
| Privacy | <input type="checkbox" style="pointer-events: none;" /> | <input type="checkbox" style="pointer-events: none;" /> | <input type="checkbox" style="pointer-events: none;" checked /> |
| Self-sovereign | <input type="checkbox" style="pointer-events: none;" /> | <input type="checkbox" style="pointer-events: none;" /> | <input type="checkbox" style="pointer-events: none;" checked /> |
| Portability | <input type="checkbox" style="pointer-events: none;" /> | <input type="checkbox" style="pointer-events: none;" /> | <input type="checkbox" style="pointer-events: none;" checked /> |
| Usability | <input type="checkbox" style="pointer-events: none;" /> | <input type="checkbox" style="pointer-events: none;" checked /> | <input type="checkbox" style="pointer-events: none;" checked /> |

### Security
DID security is based on public/private key cryptography controlled by the user.
With centralized or federated identities, security depends on provider’s authentication systems and centralized storage.

### Privacy
With DIDs, minimal disclosure is possible (share only what’s needed), often combined with verifiable credentials for selective information sharing.
Centralized identity typically requires full disclosure of stored attributes to the identity provider and sometimes to connected services.
Federated identites allow for more selective information sharing, but also enables tracking between services - especially for the provider of them.

### Self-sovereign
DIDs can be created and controlled directly by the individual.
Centralized and federated identity is issued and managed by a central authority or service provider and not owned by the participant.
However when using the DIDs in a corporate context, issuance can also be controlled by a single entity, e.g. HR, or IT.

### Portability
DID Identities are portable and can be moved or used across systems without losing control.
This is not the case with centralized or federated identites as they are tied to the issuing platform - you can lose access if account is suspended or the provider shuts down.

### Usability
DIDs are convienient to use like federated identites.
Centralized identities however are limited to certain systems and often provide a very different user experience which is prone to errors.

## References
- [W3C DID Core specification](https://www.w3.org/TR/did-1.0/)
- [A Primer for Decentralized Identifiers](https://w3c-ccg.github.io/did-primer/)
- [DIDComm](https://didcomm.org/)


[^1]: [Webinar 46 DIDs fundamentals - IdentityBook](https://docs.google.com/presentation/d/1KGLw6WThb3Q2UUOD2tZiarb_2Q_cpUZ1jzEzWCZSGII/edit?slide=id.g7d45b6a65b_4_1294#slide=id.g7d45b6a65b_4_1294)
[^issue-82]: [Personal CO (#82)](https://gitlab.1io.com/1io/co-sdk/-/issues/82)
