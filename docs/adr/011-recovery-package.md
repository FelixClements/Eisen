# ADR-011: Recovery Package

## Status

Proposed / P0.11

## Context

P0 requires a recovery-package contract covering the user-held package boundary, versioned KDF limits, AEAD AAD, encrypted keyring/trust contents, locator, and no-server-passphrase-reset behavior.

## Decision

- The recovery package is a user-held, passphrase-encrypted file.
- Argon2id derives the wrapping key; AES-256-GCM encrypts the keyring and trust sections.
- AAD binds the ciphertext to the package version, vault ID, Argon2id parameters, and section name.
- Encrypted contents include owner key, retained epoch roots, genesis/manifest chain, and device membership.
- A non-secret locator helps the user identify the package.
- The server cannot reset or bypass the passphrase. Lost passphrase means lost vault unless another enrolled device exists.

The full contract is in `docs/specs/recovery-package.md`.

## Consequences

- User is solely responsible for passphrase custody.
- Recovery restores trust state and keys on a new device, which then generates a fresh device identity.
- Support and service processes must be designed so that they cannot reset or recover the passphrase.
