# ADR-006: Task Schema and Merge Semantics

## Status

Proposed / P0.07

## Context

P0 requires a frozen task schema with field-level LWW rules, tombstone/restore semantics, mutation kinds, field bounds, and a merge-history UX requirement.

## Decision

- The task schema is defined in `docs/specs/task-schema.md`.
- Each task field carries an LWW tuple of `(hlc, device_id, value)`.
- The winner for each field is the tuple with the greater HLC; ties are broken by lexicographic `device_id`.
- Mutations are `create`, `update`, `complete`, `restore`, `delete`, and local-only `purge`.
- `deleted_at` and `completed_at` are timestamp tombstones; `restore` clears them.
- Default bounds: 100,000 tasks, 256-byte titles, 4,096-byte notes.
- Merge history in the UI shows which field was overwritten and by which HLC/device, without exposing secrets or peer plaintext.

## Consequences

- Merge is deterministic and commutative at the field level.
- Deleted tasks remain recoverable until a local purge.
- The UI must surface merge conflicts without exposing the content of competing concurrent edits.
