# COkit Overview and Positioning

## What COkit Is

COkit is an SDK written in Rust that enables developers to build decentralized, local-first,
peer-to-peer collaborative applications. It provides the full stack needed for distributed
apps: data modeling, conflict-free synchronization, identity, encryption, permissions,
and optional consensus -- all without requiring backend servers or cloud infrastructure.

## Key Value Proposition

COkit eliminates the traditional backend stack. Instead of: database + backend + API +
cloud deployment, developers define a Core (data model + business logic) and build
their frontend. Data syncs automatically between peers.

## How COkit Describes Itself

COkit positions itself as combining:
- Blockchain-style zero-trust verification and optional consensus
- Git-like branching/merging of data history
- CRDT-based automatic conflict resolution
- Verifiable decentralized identity (DIDs)

All in a single SDK that runs locally on each participant's device.

## Target Audiences

**Frontend developers:** Like a Backend-as-a-Service but free. Realtime database,
local-first data, built-in identity management via DID. No special file handling needed.

**Backend developers:** Cores work like containers with business logic and data.
Efficient content-addressed caching. Offline-first without special handling.

**Database-oriented developers:** Cores provide atomicity, consistency, isolation, and
durability. The Log functions as MVCC. Master-Master replication where business logic
decides conflict outcomes.

## Use Cases Highlighted in Sources

- Collaborative applications with data-ownership focus
- Communication platforms and messengers (privacy/security focus)
- Defense sector dual-use: battlefield communication and catastrophe management
- Receipts and verifiable proofs of transactions between parties
- Client-server architecture (define server as authority)
- Multi-region cloud services (PoA across regions)
- Edge caching with occasional server sync

## Company Context

COkit is developed by 1iO (BRANDGUARDIAN GmbH), a company with 20+ years of enterprise
collaboration experience. Their manifesto emphasizes data sovereignty, direct communication,
transparency (open source), interoperability, and accessibility.

## Source Map

- docs/src/introduction/welcome.md - positioning, value proposition
- docs/src/introduction/about-co-kit.md - audience-specific descriptions
- docs/src/introduction/why.md - philosophy, comparison to classical app development
- docs/src/introduction/key-principles.md - local-first, P2P, file-based, etc.
- docs/src/introduction/features.md - feature list
- docs/src/faq/faq.md - use cases, offline support, integration questions
- https://1io.com/en/COkit - public product page (double-source: confirms local-first, P2P, Rust, cross-platform)
- https://1io.com/en/manifest - company philosophy (double-source: confirms data sovereignty principles)
