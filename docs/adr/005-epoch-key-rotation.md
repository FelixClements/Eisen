# ADR-005: Epoch Key Rotation and Old-Epoch Cutover

## Status

Proposed / P0.06

## Context

P0 requires recorded decisions on epoch-root key rotation, per-recipient distribution, retention boundaries, and old-epoch cutover behavior.

## Decision

### Epoch-root generation

- Epoch 0 root key is a random 256-bit value generated at vault creation.
- Each rotation produces a new random 256-bit epoch root.
- Epoch root keys are never transmitted in plaintext; they are encrypted for each authorized device using that device's encryption key.

### Scoped key derivation

- From an epoch root, HKDF derives distinct keys for each purpose and origin device:
  - `epoch-<n>-owner-sign` — owner-level signing key for manifests
  - `epoch-<n>-device-sign-<device-id>` — per-device signing key
  - `epoch-<n>-encrypt-<device-id>` — per-device encryption key
  - `epoch-<n>-snapshot` — snapshot encryption key
- Context strings include the vault identifier, epoch number, purpose, and device identifier to prevent cross-purpose or cross-vault key reuse.

### Rotation and distribution

- Rotation is initiated by the owner.
- A new epoch is created and the new root is distributed in encrypted envelopes to every authorized device.
- Each device acknowledges receipt in a durable local commit and publishes a membership proof signed with the new epoch key.
- Rotation is staged with an `effective_epoch` value; devices continue using the current epoch until the cutover.

### Retention boundary

- Old epoch root keys and device keys are retained for a bounded recovery window of 90 days.
- After the retention window, old epoch keys are securely erased from all devices.
- Backups and snapshots from an old epoch remain encrypted and are not re-encrypted; access requires the old epoch key if still retained.

### Old-epoch cutover behavior

- After the cutover epoch number, devices reject new mutations signed with an old epoch key.
- Operations that were created before the cutover and already in the immutable log are accepted if their epoch was valid at creation time.
- A device that has not received the new epoch key cannot author new operations and must repair from a snapshot or re-enroll.
- The UI must show an advisory state until all devices have acknowledged the new epoch.

## Consequences

- Forward secrecy is limited to epoch boundaries; compromise of an old epoch key allows decryption of that epoch's data until the retention window expires.
- Recovery packages include all retained epoch roots so restores remain possible for the retention window.
- The cutover is a hard boundary for new operations but does not rewrite history.
