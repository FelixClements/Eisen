# Retention Policy

## Data categories

### Encrypted envelopes and snapshots

- Stored on the cloud service for 90 days after the vault is active.
- Extended retention is user-configurable up to the account limit.
- Deleted envelopes are marked for removal and purged after the retention window.

### Recovery packages

- Held by the user. The service never holds or backs up recovery packages.
- Users are responsible for retaining their own recovery packages.

### Epoch keys

- Old epoch root keys are retained on devices for 90 days.
- After 90 days, old epoch keys are securely erased from all devices and are no longer included in newly created recovery packages.
- Recovery packages are static, user-held files (see `recovery-package.md`); the system cannot reach into an already-exported package to erase keys. Any recovery package created before the window can still decrypt the old-epoch data it captured, and retaining or discarding it is the user's responsibility.

### Local device artifacts

- WAL, journal, caches, and temporary files are retained only as needed for operation.
- Encrypted backups of local state follow the cloud retention window.

### Logs and diagnostics

- Operational logs may retain non-content metadata (timestamps, request IDs, error codes) for 30 days.
- Crash reports and diagnostics must not contain keys, passphrases, or decrypted task data.

## Deletion

- When a user deletes a task, the tombstone operation is appended. The deleted task data remains in encrypted envelopes for the retention window.
- Account deletion triggers removal of all cloud data after the retention window.
- Local deletion is device-local and does not affect other devices' copies until the tombstone is synchronized.

## Availability limitations

- The service does not guarantee availability. Users can continue to operate offline.
- Relay is volatile and does not persist data; availability depends on peer connectivity.
