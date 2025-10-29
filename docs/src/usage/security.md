# Security

```admonish note
All security mechanics in CO-kit assume that the host machine it is running on is safe and can be trusted.
If that is not the case you will have much bigger problems to worry about as no piece of software will be secure.
```

```admonish warning
Cryptography in CO-kit has currently not been peer reviewed / audited.
```

## Cryptography
Each piece of data in CO-kit carries an identifier for the cryptographic scheme used.
This ensures interoperability, supports multiple algorithms, allows upgrading and makes data structures self-describing.

Cryptography used:
- Content hashing: Blake3-256
- Symmetric cryptography: XChaCha20-Poly1305
- Asymmetric cryptography: Ed25519
- Key exchange: X25519
- Key derivation: Argon2

### Packages
Cryptography crates used:
- [blake3](https://crates.io/crates/blake3)
- [sha2](https://crates.io/crates/sha2)
- [ed25519](https://crates.io/crates/ed25519)
- [chacha20poly1305](https://crates.io/crates/chacha20poly1305)
- [aead](https://crates.io/crates/aead)

## Privacy
Privacy depends on the used networking strategy.

PeerID are unique any may tracked by malicious peers.
However, they can be rotated anytime to improve privacy.

The builtin didcontact protocol notably sends, specifed by the didcomm spec, the senders DID reference in plain text.
The recipent is encrypted so this only allows to relate a PeerID to a DID.

## Related
- [Security (#24)](https://gitlab.1io.com/1io/co-sdk/-/issues/24)
