# Security

```admonish note
All security mechanics in COKIT assume that the host machine it is running on is safe and can be trusted.  

If this is not the case, you have much bigger problems to worry about, because no software will be secure.
```

```admonish warning
Cryptography in COKIT has not yet been peer reviewed/audited.
```

## Cryptography
Each piece of data in COKIT carries an identifier for the cryptographic scheme used.  
This ensures interoperability, supports multiple algorithms, allows upgrading, and makes data structures self-describing.

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
Privacy is dependent on the networking strategy used.

PeerIDs are unique, and may tracked by malicious peers.  
However, they can be rotated anytime to improve privacy.

As specified by the [didcomm](../reference/network.md#protocol-didcomm) spec, the built-in [didcontact](../reference/network.md#protocol-didcontact) protocol sends the sender's DID reference in plain text.  
The recipient is encrypted, however, so this can only be used to relate a PeerID to a DID.

## Related
- [Security (#24)](https://gitlab.1io.com/1io/cokit/-/issues/24)
