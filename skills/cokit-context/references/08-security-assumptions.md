# Security Assumptions and Cryptography

## Host Trust Assumption

**All security mechanics in COkit assume that the host machine is safe and can be trusted.**
If the host is compromised, no software-level security applies.

## Audit Status

**Cryptography in COkit has not yet been peer reviewed/audited.**
This is explicitly noted in the official docs.

## Cryptographic Primitives

Each piece of data in COkit carries an identifier for the cryptographic scheme used,
making data structures self-describing and supporting algorithm upgrades.

| Function | Algorithm |
|----------|-----------|
| Content hashing | Blake3-256 |
| Symmetric encryption | XChaCha20-Poly1305 |
| Asymmetric cryptography | Ed25519 |
| Key exchange | X25519 |
| Key derivation | Argon2 |

### Rust Crates Used
- blake3, sha2, ed25519, chacha20poly1305, aead

## Encryption in COs

### CO-Level Encryption
Each CO can be configured as encrypted or unencrypted at creation time.
- **Private COs**: Encrypted (only participants can access)
- **Public COs**: Unencrypted (anyone can read)
- **Local CO**: Always encrypted (root key in OS keychain)

### Storage Encryption Layer
The storage stack includes an encryption layer that encrypts blocks before persisting.
Default: XChaCha20-Poly1305.

### Key Management
Encryption keys are stored in the Local CO's `co-core-keystore` Core.
Keys can be versioned: when a new version is created, new data uses the new key.
This enables advanced patterns like "new participants only see data produced after they joined."

### Root Key
The Local CO's root encryption key is stored in the OS keychain.
All other keys in the Local CO are derived from this root key.

## Privacy Considerations

### PeerID Tracking
PeerIDs are unique and could theoretically be tracked by malicious peers.
Mitigation: PeerIDs can be rotated anytime (force_new_peer_id setting).

### DIDComm Sender Visibility
Per the DIDComm spec, the didcontact protocol sends the sender's DID reference in
plain text. The recipient is encrypted. This means a PeerID can be related to a DID
by observing network traffic.

### Pairwise DIDs for Privacy
Users can create many context-specific DIDs to prevent cross-context correlation.

## Verification Properties

- **Content addressing (CIDs)**: All data is tamper-evident. Any change produces a
  different CID. Data integrity is cryptographically verifiable at any time.
- **DID-signed events**: Every Log event is signed by the submitting participant's DID,
  providing non-repudiation and identity verification.
- **Deterministic WASM reducers**: All peers compute the same state transitions,
  enabling cross-peer verification.
- **Consensus finality**: When reached, checkpoints are cryptographically verifiable
  and immutable among CO participants.

## Trust Model

COkit provides a zero-trust environment:
- End-to-end encryption in transit and at rest
- End-to-end verification: cryptographic integrity AND identity verification
- Private data is not shared with unknown peers (even encrypted data)
- Auditable and non-repudiable history of state changes

## What COkit Does NOT Provide (Security Caveats)

- No protection against a compromised host machine
- Cryptography is not yet audited (single-source: docs/src/usage/security.md)
- No built-in protection against traffic analysis (PeerID correlation caveat above)
- Consensus finality depends on authority availability (PoA can be blocked if >1/3 offline)

## Source Map

- docs/src/usage/security.md (primary: trust assumption, crypto primitives, privacy caveats)
- docs/src/reference/storage.md (encryption layer, XChaCha20-Poly1305)
- docs/src/reference/co.md (CO encryption settings, key versioning)
- docs/src/reference/identity.md (DID-based signing, pairwise DIDs)
- docs/src/faq/faq.md (verification, zero-trust claims)
- docs/src/usage/configuration.md (force_new_peer_id for privacy)
