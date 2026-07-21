# ADR-004: Owner Key Custody and Transfer

## Status

Proposed / P0.06

## Context

P0 requires recorded decisions for owner-key custody, transfer rules, and the genesis trust anchor.

## Decision

### Genesis trust anchor

- At vault creation, a random owner Ed25519 signing key pair is generated on the creating device.
- The public key is the genesis trust anchor.
- The private key is encrypted with a key derived from the user's vault passphrase plus Argon2id and stored in the user-held recovery package and in the creating device's secure storage.
- A genesis manifest is created containing:
  - Vault identifier (random 256-bit value, base64-encoded in the manifest)
  - Owner public key
  - Genesis epoch public key
  - Creation timestamp
  - A self-signature by the owner signing key

### Key custody

- The owner private signing key must exist in plaintext only in platform secure storage during use.
- It may be exported only as part of the recovery package, encrypted under the user's passphrase.
- No server holds, stores, or can reset the owner key or passphrase.
- Device signing and encryption keys are distinct from the owner signing key.

### Key transfer

- Transferring ownership requires a signed transfer manifest from the current owner.
- The new owner generates a new Ed25519 signing key pair on their device.
- The current owner signs a manifest that includes the new owner public key, an effective epoch, and a revocation of the old owner key.
- The transfer is atomic: from the effective epoch forward, all manifests must be signed by the new owner key; pre-transfer manifests remain valid back to the genesis trust anchor.
- After transfer, old-owner device keys are no longer authorized for new mutations and must be re-enrolled or revoked.

## Consequences

- Recovery is impossible without the passphrase or recovery package.
- Ownership transfer is a deliberate, multi-step protocol that cannot be completed by the service.
- A lost owner key without a recovery package means the vault cannot be recovered.
