# Recovery Package Contract

## Scope

This document defines the user-held recovery package: the boundary, format, KDF, AEAD, encrypted contents, locator, and the no-server-passphrase-reset guarantee.

## User-held boundary

- The recovery package is a single file held by the user. The service cannot create, read, or reset it.
- It contains everything needed to recover the vault on a new device, except the user's passphrase.
- The package does not contain plaintext task content; it contains the encrypted keyring and trust material.

## Format

| Section | Contents |
|---|---|
| Header | `EISEN-RECOVERY` magic, version uint32, Argon2id parameters |
| KDF salt | 16 random bytes |
| Encrypted keyring | AEAD-encrypted blob containing owner signing key, retained epoch roots, and genesis manifest |
| Trust contents | AEAD-encrypted blob containing device membership and manifest chain |
| Locator | Non-secret hint (e.g., vault ID prefix) to identify which recovery package belongs to which vault |
| Checksum | SHA-256 over the canonical bytes of the preceding sections |

## KDF

- Argon2id is used to derive the wrapping key from the user's passphrase and the salt.
- Parameters are stored in the header so the package is versioned and future parameters can be used.
- Default profile: mobile (19 MiB, t=2, p=1) or desktop (64 MiB, t=3, p=4), selected at package creation time.

## AEAD

- AES-256-GCM is used for the encrypted sections.
- The nonce is 96 bits and derived from the section index and a package-scoped counter.
- Additional Authenticated Data (AAD) includes:
  - The recovery-package version
  - The vault identifier
  - The Argon2id parameters
  - The section name (`keyring` or `trust`)

## Encrypted keyring

- Owner Ed25519 signing key
- Retained epoch root keys (current + old-epoch recovery window)
- Genesis manifest and owner trust anchor

## Trust contents

- Current manifest chain (genesis to current epoch)
- Device membership list with public keys and epoch authorizations
- Recovery locator: a user-provided optional label or the vault ID prefix

## Locator

- The locator is non-secret and may be displayed to help the user identify the correct recovery package.
- It must not contain the passphrase, keys, or any plaintext task data.

## No-server-passphrase-reset

- The server has no capability to derive, reset, or bypass the recovery-package passphrase.
- A forgotten passphrase results in permanent vault loss unless the user has another enrolled device or a separate unencrypted backup.
- Support must never request the passphrase or recovery package from the user.

## Creation and restore

- Creation: the user chooses a passphrase; the device runs Argon2id, encrypts the keyring and trust contents, and writes the package.
- Restore: the new device runs Argon2id with the stored parameters, decrypts the package, generates a new device identity and nonce domain, and re-enrolls.

## Wrong-passphrase behavior

- Decryption failure (AEAD tag mismatch) is treated as a wrong passphrase.
- The implementation must not leak whether the package is corrupted or the passphrase is wrong beyond the AEAD failure.
- A small delay and failure counter may be applied to slow brute-force attempts.
