# ADR-008: Fail-Closed Counter and Rollback/Clone Policy

## Status

Proposed / P0.08

## Context

P0 requires a decision on how the local mutation counter, nonce, and secure storage behave in the face of rollback, clone, or recovery events.

## Decision

### Counter / nonce policy

- The local nonce counter is persisted in a transaction-bound way.
- On every local mutation, a new nonce is reserved and the counter is advanced before the mutation commits.
- If the transaction fails, the reserved nonce is marked as consumed and not reused.
- On startup, the last persisted counter value is loaded. If secure storage is unavailable, the counter is considered uncertain and no new nonces are issued until storage is restored.

### Rollback detection

- A rollback is detected when the persisted counter is less than a previously acknowledged value, or when a stored operation ID is missing from the local log.
- On detected rollback, the device enters a repair state. New mutations are paused until the log and counter state are reconciled with a trusted snapshot or another device.

### Clone policy

- Vault data must not be cloned to another device by copying database files.
- Each device has a unique device identity and nonce domain. A clone would result in duplicate device IDs or nonce reuse and is rejected by the membership and nonce checks.
- Restoring from a recovery package creates a new device identity and a fresh nonce domain.

## Consequences

- Nonce reuse is prevented even across crashes, rollbacks, and clones.
- A device that loses secure storage or is rolled back cannot silently resume mutating; it must repair.
- Recovery-package restore is treated as a new device, preventing clone attacks.
