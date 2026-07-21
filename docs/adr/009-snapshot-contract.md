# ADR-009: Snapshot Contract

## Status

Proposed / P0.09

## Context

P0 requires a decision on snapshot semantics: acceptance, per-origin coverage, monotonic checkpoint, retention, stale/incomplete/replay rejection, and repair rules.

## Decision

### Signed snapshot acceptance

- A snapshot is a signed, encrypted bundle containing the materialized state and the operation log coverage.
- Snapshots are signed by the owner key of the epoch in which they are created.
- A device accepts a snapshot only after verifying the owner signature, the epoch manifest chain, and the encrypted payload integrity.

### Per-origin coverage

- A snapshot declares the contiguous operation range it covers for each origin device.
- Coverage is represented as `(origin_device_id, start_seq, end_seq)` tuples.
- A device must not install a snapshot unless it either has no local state or the snapshot is a strict superset of the local coverage.

### Monotonic checkpoint

- Each snapshot carries a monotonic checkpoint HLC that is greater than or equal to all operation HLCs it contains.
- Devices track the highest checkpoint they have accepted and reject snapshots with a lower or equal checkpoint.

### Retention

- Snapshots are retained by the owner and may be stored on the cloud service as opaque encrypted blobs.
- Only the most recent compatible snapshot per device is required to be retained locally.
- Older snapshots are retained for the same 90-day window as old epoch keys.

### Stale / incomplete / replay rejection

- Stale snapshots: checkpoint older than the device's current checkpoint are rejected.
- Incomplete snapshots: missing operation ranges or non-contiguous coverage are rejected.
- Replay: a snapshot with a checkpoint already seen is rejected.

### Repair rules

- If local state is inconsistent with accepted coverage, the device enters repair mode.
- Repair installs the verified snapshot, marks missing local operations as repaired, and then replays subsequent operations from the log or cloud.
- Local operations not covered by the snapshot are preserved and reconciled; they are never silently discarded.

## Consequences

- Snapshots provide a bounded recovery point and a way to bootstrap new devices.
- Coverage tracking prevents silent data loss during repair.
- Replay and stale rejection keep the immutable log append-only and monotonic.
