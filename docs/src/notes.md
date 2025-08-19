- Guards/Permissions redundant?
- Next steps -> more examples?
- kleines Messenger beispiel? - beispiel, was man sofort benutzen kann?
- minimal coding -> dioxus
- minimal coding verlinken mit source code
- screens einbauen von applications (figma)
- next steps: von der liste ausgehend eine beispielapp bauen, gemeinsam, step by step?
- Tauri -> tutorial/beispielapp generieren?
- step by step guide wie man eine app baut
- use cases in introduction
- side by side view: coding conservatively + coding with cokit -> no backend, no bs
	- how/dev benifits
	- cargo add co-sdk genauer erklären
- mehr zu tauri erklären?
- cbor

## Glossary
#### Terms
- cbor
- cokit
- co
- tauri
- dioxus
- guards
- core
	- state
	- schema
	- actions
- co-api
- CLA
- APLGv3
- merkle dags
- crdt
- DID
- CID
- logs (heads)
- ipld
- Consensus
- PoA
- peerID
- storage
- mDNS
- Discovery
- Wasm


# Structure
- **Introduction** 
	- about co-sdk
	- why
	- key principles
		- local-first
		- p2p
		- interoperability
		- accessibility
		- open & democratic
- **Getting Started**
	- Requirements
	- OS-specifics 
	- Installation
		- Rust
		- cargo-binstall
		- optional Dioxus CLI
	- Minimal Coding Example
	- Next Steps -> Build your first app!
- **Reference**
	- Co-Terms
		- cokit
		- core
		- CO (all of them)
		- Guards
	- SDK Components
		- Storage
		- Log (CRDT: Conflict free Replicated Data Type)
		- Network
		- Identity
		- Permissions
	- Architecture & Internals (graphic)
		- Core concepts
		- Data flows
		- Important design decisions
		- Dependency overview
- **Usage**
	- Best practices 
	- API Overview core (co-api) -> rust doc
	- API Overview apps (co-sdk) -> rust doc
	- Configuration
		- config files
		- environment variables
- **Development and Contributing**
	- Clone & build
	- Tests
	- How to contribute (CLA)
	- Bug report
	- Feature request -> cokit maintainer
- **FAQ & Troubleshooting**
- **Licensing & Legal**




# Notes

- **Intro**
	- About co-sdk + why
		Collaboration is the core of the internet's value generation. 
		Everytime collaborate, we have no option but to trust platforms with our most private data to respect and protect our privacy, in exchange for convenience and cool features. We are exploited through the data we generate.
		Cokit is turning the tables. Now you can give power back to your users. We want you to build apps that allow users to physically own their digital life, and not just in a sense of static files, but in full fledged collab workflows. 
	- COOL:
		- No Backend
		- No Databases 
		- No Queries

		- Key Principles
			- Local-first
				Data is stored locally on their end device and is only shared with the group of people with whom they are currently collaborating, encapsulated from all other users, creating their own private platform. The user keeps full ownership and stays in control the whole time.
			- p2p
				Only the necessary data is exchanged in end-to-end encrypted form using peer-to-peer technology. There are only users, no central servers as a middleman.
			- interoperability
				Different Workflows and tools that build on our foundation should be compatible with each other, even if the providers are not partners. This should not be something the app developer has to think about, but should be built in from the start.
			- accessibility
				We want everyone to be able to take full advantage of the platform and derive value from it, not just devs. this is why we're opting for easy to use applications built with cokit. Not only shall the application be easy to use, but we hope for the best dx possible.
			- open & democratic
				No one should ever own a platform and exploit others and their data. We see cokit and public property. Everything is open-source. Go and build a better digital future with it.
	- getting started
		- Voraussetzungen: Rust, Cargo, Rust Toolchain
		- Installation via `cargo add co-sdk`
		- Quickstart



- **Licensing**
	- AGPLv3 w/ linking exception

- **Reference**
	- 1io terms
		- cokit
			- co
			- log
			- heads
			- 
		- core
			- state
			- schema
			- actions
			- metadata
			- etc
	- Terminology (general)
		- merkle dags
		- dht
		- DID

- additional learning materials -> rust docs
- 


### Unsure
- System Requirements
- matrix 
- merkle dags
- mDNS
- 


### More Notes
- Rust documentation structure
	- short sentence explaining what it is
	- more detailed explanation, if necessary
	- at least one code example that users can copy/paste to try it
	- even more advanced explanations if necessary
	  
	  


# co-sdk

## Introduction

### About
Collaboration is the core of the internet's value generation. 
Everytime collaborate, we have no option but to trust platforms with our most private data to respect and protect our privacy, in exchange for convenience and cool features. We are exploited through the data we generate.
Cokit is turning the tables. Now you can give power back to your users. We want you to build apps that allow users to physically own their digital life, and not just in a sense of static files, but in full fledged collab workflows. 

### Key Principles
#### Local First
Data is stored locally on the users end device and is only shared with the group of people with whom they are currently collaborating, encapsulated from all other users, creating their own private platform. 

#### p2p
Only the necessary data is exchanged in end-to-end encrypted form using peer-to-peer technology. There are only users, no central servers as a middleman.

#### Interoperability
Tools that build on co-sdk should be compatible with each other, even if the builders are not partners. Interoperability between cool solutions is already built in.

#### Accessibility
We want everyone to be able to take full advantage of the platform and derive value from it, not just devs. Not only shall the applications built with cokit be easy to use, but we also hope for the best dx possible.

#### Open & Democratic
No one should ever own a platform and exploit others and their data. We see cokit and public property. Everything is open-source. Go and build a better digital future with it. -> See more in the chapter [Licensing]

### How
https://gitlab.1io.com/1io/ecosystem/-/issues?show=eyJpaWQiOiIzNzMiLCJmdWxsX3BhdGgiOiIxaW8vZWNvc3lzdGVtIiwiaWQiOjQ1NH0%3D







## Schnellstart
- Voraussetzungen
- Installation
- Minimalbeispiel

## Funktionen
- Feature 1
- Feature 2

## Installation & Konfiguration
1. Schritt 1
2. Schritt 2

## API
### Modul XY
- Funktion abc()

## Tests
- `cargo test`

## Deployment & Doku
- `cargo doc`

## Mitmachen
Wie man beitragen kann.

## Lizenz
MIT License © 2025 Max Mustermann

## Links
- [Website](https://example.com)
- [Dokumentation](https://example.com/docs)
