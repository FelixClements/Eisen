# ADR-003: Cryptographic Primitives and Platform Capability Matrix

## Status

Proposed / P0.05

## Context

P0 requires a recorded decision on the cryptographic primitives used for encryption, key derivation, digests, signing, and key exchange, plus a platform capability matrix and Argon2id parameters.

## Decision

All cryptographic operations live in the shared Rust core. Platform-specific code is responsible only for secure key storage, random-number sourcing, and lifecycle handling.

## Approved primitives

| Purpose | Primitive | Library / crate | Notes |
|---|---|---|---|
| Symmetric encryption | AES-256-GCM | `aes-gcm` | 96-bit IV, 128-bit tag, no padding. |
| Key derivation | HKDF-SHA-256 | `hkdf` | Extract-and-expand for sub-key derivation. |
| Password hashing | Argon2id | `argon2` | Memory-hard KDF for recovery-package passphrase. |
| Digest | SHA-256 | `sha2` | Default digest for canonical bytes, manifest chains, and operation IDs. |
| Signing | Ed25519 | `ed25519-dalek` | Device and owner signing keys. |
| Key exchange | X25519 | `x25519-dalek` | Ephemeral key exchange during enrollment. |
| Secure random | OS CSPRNG | `getrandom` | Delegates to Android Keystore RNG, Windows CNG, or `/dev/urandom`. |

## Platform capability matrix

| Capability | Android | Windows | Rust core |
|---|---|---|---|
| AES-GCM | Rust core | Rust core | `aes-gcm` |
| HKDF | Rust core | Rust core | `hkdf` |
| Argon2id | Rust core | Rust core | `argon2` |
| SHA-256 | Rust core | Rust core | `sha2` |
| Ed25519 signing | Rust core | Rust core | `ed25519-dalek` |
| X25519 key exchange | Rust core | Rust core | `x25519-dalek` |
| Secure random source | `SecureRandom` + `getrandom` | Windows CNG + `getrandom` | `getrandom` |
| Key material storage | Android Keystore | Credential Locker / DPAPI | Encrypted at rest by host |
| Biometric / device-lock binding | StrongBox / TEE where available | Windows Hello / TPM where available | N/A |

## Argon2id parameters

The default parameters balance resistance against offline cracking with mobile device constraints. Two profiles are defined:

| Profile | Memory (KiB) | Iterations | Parallelism | Salt length | Tag length | Target use |
|---|---|---|---|---|---|---|
| Mobile | 19,456 (19 MiB) | 2 | 1 | 16 bytes | 32 bytes | Phones, low-end Android, battery-sensitive devices |
| Desktop / Server | 65,536 (64 MiB) | 3 | 4 | 16 bytes | 32 bytes | Windows desktops, cloud build agents, high-end devices |

The mobile profile aligns with the OWASP Password Storage Cheat Sheet minimum recommendation. The desktop profile provides a higher memory hardness for devices where it is acceptable.

### Benchmarked selection process

1. Run `argon2` benchmark harness on representative Android and Windows devices at install time or first unlock.
2. Measure time to derive a key with the desktop profile; if it exceeds 500 ms on the target device class, fall back to the mobile profile.
3. Record the selected profile in the local device manifest. Do not change the profile after vault creation without a migration.
4. Re-run benchmarks after major OS or hardware changes and update the default profile if warranted.

## Review plan

- Dependency audit before each release: verify crate versions, review `cargo audit` output, and confirm no yanked or vulnerable crypto dependencies.
- Quarterly review of primitive choices against current standards (NIST, OWASP, IETF) and published cryptanalysis.
- Independent crypto review before cloud-sync release (G5).
- Maintain a capability matrix diff in the ADR when platform APIs or crate versions change.

## Consequences

- Centralizing crypto in the Rust core reduces the attack surface and ensures identical behavior on Android and Windows.
- Platform key-storage APIs are abstracted behind a small interface; device binding is limited to what each platform can provide.
- Argon2id parameters are tunable per device class, but the recovery-package format must record which profile was used.
