# ADR-007: Local-At-Rest Coverage

## Status

Proposed / P0.08

## Context

P0 requires a decision on local-at-rest encryption coverage for all disk artifacts created by the client.

## Decision

All local disk artifacts that can contain vault data must be encrypted under keys that never leave platform secure storage or the user-held recovery package.

| Artifact | Coverage | Notes |
|---|---|---|
| Database (Room/SQLite) | Encrypted | Use SQLCipher or encrypted Room with a key derived from the epoch root and stored in Keystore/Credential Locker. |
| WAL / journal | Encrypted | Covered by database encryption; ensure WAL and journal files are included in the encrypted page size. |
| Caches | Encrypted or non-secret | In-memory caches may hold decrypted plaintext; on-disk caches must be encrypted or contain only non-secret IDs/cursors. |
| Exports | Encrypted | Explicit encrypted export flow; never write plaintext vault content. |
| Backups | Encrypted | Device backups must include only encrypted artifacts and the recovery package. |
| Crash artifacts | Non-secret only | Crash reports and diagnostic logs must not contain keys, passphrases, decrypted task data, or recovery material. |
| Temporary files | Cleared | Use encrypted temp storage or securely erase plaintext temp files after use. |

## Consequences

- Any plaintext leak of task content to disk is a security bug.
- Backup and restore tooling must be configured to exclude decrypted caches and logs.
- Diagnostics and crash reporting must be reviewed to ensure no secret material is logged.
