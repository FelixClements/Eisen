# Merge-History UX Behavior Note

## Scope

This note defines how the Eisen PWA exposes merge/history evidence when a task field is overwritten by a concurrent edit from another device. It is tied to the frozen field-level LWW merge contract in `docs/specs/task-schema.md` and ADR-006.

## What the user sees

- When the materialized state of a task field differs from the locally-generated edit the user made, the UI shows a **merge-evidence indicator**.
- The indicator is shown once per affected field, at the point the winning mutation is applied to the materialized view.
- The user can open a **History panel** on the task detail screen to see the list of fields for which merge evidence exists.

## Evidence shown

For each overwritten field the UI displays:

- **Field name** (e.g., "Title", "Notes", "Quadrant", "Due date").
- **Winning mutation time** as a human-readable wall-clock timestamp derived from the winning HLC's `wall` value. The raw HLC `counter` and `device_id` are not shown by default.
- **Device label** of the device that produced the winning mutation. This is a stable, non-secret label derived from the public `device_id` (e.g., a user-supplied name or "Device 8f4a..."). No secret key material, recovery data, or plaintext from the competing edit is shown.
- **Outcome** in plain language:
  - "Updated from another device" for `Update`
  - "Marked complete from another device" for `Complete`
  - "Deleted from another device" for `Delete`
  - "Restored from another device" for `Restore`

## Evidence not shown

- The losing value (the local or peer value that was overwritten) is never displayed, and no diff is shown.
- The full other device `device_id` is not displayed. Only a short, stable, non-reversible label is used.
- The winning HLC `counter` and raw `wall` ms are not shown directly; they are only used to derive the timestamp and to determine the winning order.

## Display rules

- Merge evidence is shown only when the field value currently visible is the result of a winning concurrent edit from a different `device_id` than the local edit the user made.
- If the user later makes a local change that wins, the merge-evidence indicator for that field is cleared (or replaced if another concurrent edit wins).
- `deleted_at` and `completed_at` tombstones follow the same rule: the UI shows who won the delete/complete/restore race, not the competing state.
- `purge` is never shown because it is a local-only cleanup request and not merge-visible.

## Example scenarios

1. **Concurrent title edit** — Two devices update the same task title. The device with the higher HLC wins. The user on the losing device sees "Title: updated from another device at 14:32" and the winning title.
2. **Delete vs complete** — A task is deleted on device A and completed on device B. The higher-HLC mutation wins; the UI shows either "Deleted from another device" or "Marked complete from another device".
3. **Restore vs delete** — A restore on device A competes with a delete on device B. The higher HLC wins and the UI reflects the winning action with the appropriate evidence.

## Privacy and security constraints

- No secret content from the winning or losing edit is exposed.
- The timestamp is derived from `wall` only; it does not encode key lifecycle or epoch information.
- Device labels are computed from public `device_id` bytes and are not linked to owner or recovery material.
- Merge evidence is stored only in the encrypted local materialized view; it is not logged, reported, or synced independently.

## Out of scope

- A full linear edit history is not exposed (the log is immutable but not shown).
- Per-field diff or side-by-side comparison of competing values is not shown.
- Conflicting values are not exposed to external analytics, crash reporting, or the service worker.
