# Task Schema

## Task object

A task is a map of fields. Each field has an associated **LWW tuple** `(hlc, device_id, value)`.

| Field | Type | Required | Bounds | Notes |
|---|---|---|---|---|
| `id` | byte[16] | yes | 16 bytes | Stable random task identifier. |
| `title` | text | yes | 1–256 chars UTF-8 | Display title. |
| `notes` | text | no | 0–4,096 chars UTF-8 | Free-form notes. |
| `quadrant` | uint | yes | 0–3 | 0=Do, 1=Schedule, 2=Delegate, 3=Eliminate. |
| `due_date` | uint | no | POSIX ms | Optional due date. |
| `completed_at` | uint | no | POSIX ms | Set when task is completed. |
| `deleted_at` | uint | no | POSIX ms | Tombstone marker. |
| `created_at` | uint | yes | POSIX ms | HLC of first create mutation. |
| `updated_at` | uint | yes | POSIX ms | HLC of last mutation. |

## Mutation kinds

| Mutation | Effect |
|---|---|
| `create` | Creates `id`, `title`, `notes`, `quadrant`, `due_date`, `created_at`, `updated_at`. Fails if `id` already exists. |
| `update` | Updates one or more of `title`, `notes`, `quadrant`, `due_date`. Updates `updated_at`. |
| `complete` | Sets `completed_at` to mutation HLC. Updates `updated_at`. |
| `restore` | Clears `completed_at` and `deleted_at`. Updates `updated_at`. |
| `delete` | Sets `deleted_at` to mutation HLC. Updates `updated_at`. |
| `purge` | Local-only request to remove a task from the local materialized view and log. Requires the task to be deleted. |

## Field-level LWW tuple comparator

- For each field, the winner is the tuple with the greater HLC.
- If HLCs are equal, the lexicographically greater `device_id` (UUID bytes) wins to ensure determinism.
- A field value of `null` in an `update` mutation clears the field if it wins.

## Tombstone and restore semantics

- `deleted_at` is a tombstone. A deleted task is hidden from normal views but retained in the log.
- A `restore` mutation clears `deleted_at` and `completed_at`, returning the task to an active state.
- A `complete` mutation on a deleted task is allowed and sets `completed_at`.
- A `delete` mutation on a completed task sets `deleted_at` and retains `completed_at`.
- A `purge` is not a merge-visible mutation; it is a local-only cleanup request.

## Merge result

- The merged state of a task is the per-field LWW winner across all seen mutations.
- A task is considered deleted in the materialized view if `deleted_at` is non-null and `deleted_at` wins over `restore` mutations that clear it.
- A task is considered completed if `completed_at` is non-null and is not cleared by a winning `restore`.

## Field bounds

- Maximum tasks per vault: 100,000 (default; configurable by protocol profile).
- Maximum title length: 256 UTF-8 bytes.
- Maximum notes length: 4,096 UTF-8 bytes.
- Maximum attachments / per-task metadata: protocol profile defines; default is zero in the local-only phase.

## Merge-history UX requirement

- The UI must indicate when a field's current value was overwritten by a concurrent edit from another device.
- The indication must not expose the other device's plaintext content or secret state.
- Details shown: which field changed, the HLC of the winning mutation, and a non-secret device label if available.
