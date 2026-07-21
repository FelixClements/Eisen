# State Machines

This document defines the state machines for HLC generation, task mutations, device membership, revocation, ownership transfer, and epoch cutover.

## HLC state machine

### States

- `INIT`: HLC has not been persisted.
- `READY`: HLC counter and wall-clock component are persisted.
- `SKEW`: Received HLC is too far ahead of local wall clock.

### Transitions

- `INIT → READY`: on first local mutation, set `wall = now()` and `counter = 1`.
- `READY → READY` (local mutation): `wall = max(now(), previous_wall)`, `counter = (wall == previous_wall) ? previous_counter + 1 : 0`.
- `READY → READY` (receive): `wall = max(now(), received_wall)`, `counter = (wall == received_wall) ? max(local_counter, received_counter) + 1 : received_counter`.
- `READY → SKEW`: if `received_wall - now() > skew_threshold` (default 5 minutes).
- `SKEW → READY`: after logging diagnostic input and either catching up or operator override.

### Deterministic ordering

- HLCs are ordered first by `wall`, then by `counter`.
- Tuples are `(wall, counter, device_id)`; ties in `(wall, counter)` are broken by lexicographic `device_id`.

## Task mutation state machine

### States

- `ABSENT`
- `ACTIVE`
- `COMPLETED`
- `DELETED`

### Transitions

- `ABSENT → ACTIVE`: `create` mutation.
- `ACTIVE → ACTIVE`: `update` mutation that modifies fields.
- `ACTIVE → COMPLETED`: `complete` mutation.
- `COMPLETED → ACTIVE`: `restore` mutation.
- `ACTIVE → DELETED`: `delete` mutation.
- `COMPLETED → DELETED`: `delete` mutation.
- `DELETED → ACTIVE`: `restore` mutation.
- `DELETED → DELETED` (no change): any mutation that does not win LWW for `deleted_at`.
- `PURGE` is local-only and removes the task from the materialized view.

### Retry rules

- A local mutation that fails to commit is retried by the outbox with a new nonce and HLC.
- The same operation ID must not be reused with different content.

## Membership state machine

### States

- `PENDING`: enrollment capability created but not yet accepted.
- `AUTHORIZED`: device is a member of the current epoch.
- `REVOKED`: device has been revoked by the owner.

### Transitions

- `PENDING → AUTHORIZED`: enrollment handshake completes and a membership manifest is signed.
- `AUTHORIZED → REVOKED`: owner signs a revocation manifest.
- `REVOKED → PENDING`: a new enrollment is required; the old device identity is never reused.

## Ownership-transfer state machine

### States

- `OWNER_CURRENT`
- `OWNER_TRANSFER_PENDING`
- `OWNER_TRANSFERRED`

### Transitions

- `OWNER_CURRENT → OWNER_TRANSFER_PENDING`: owner signs a transfer manifest naming a new owner and a future epoch.
- `OWNER_TRANSFER_PENDING → OWNER_TRANSFERRED`: the transfer epoch arrives and the new owner key signs the next manifest.
- `OWNER_TRANSFERRED → OWNER_CURRENT`: not allowed; transfer is irreversible without a new transfer from the new owner.

## Epoch-cutover state machine

### States

- `EPOCH_STABLE`
- `EPOCH_ROTATING`
- `EPOCH_CUTOVER`

### Transitions

- `EPOCH_STABLE → EPOCH_ROTATING`: owner initiates rotation; new epoch root is distributed.
- `EPOCH_ROTATING → EPOCH_CUTOVER`: all known devices have acknowledged the new epoch, or the cutover timeout elapses.
- `EPOCH_CUTOVER → EPOCH_STABLE`: local device begins using the new epoch and stops accepting old-epoch mutations.

### Pre/post-cutover behavior

- Before cutover, devices may author operations in the current epoch.
- During rotation, devices can receive and apply operations from both epochs.
- After cutover, new operations must use the new epoch; old-epoch operations are rejected unless already in the immutable log.
- Devices that missed the cutover must repair from a snapshot or re-enroll.

## Deterministic ordering rules

- HLC ordering is total within a single vault.
- For concurrent field updates, the LWW tuple `(hlc, device_id)` decides the winner.
- Manifest-chain ordering is by epoch number, then by owner signature sequence.
