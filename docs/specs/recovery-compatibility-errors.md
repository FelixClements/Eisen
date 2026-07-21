# Recovery, Compatibility, and Error Contracts

## Recovery transaction

1. The user selects a recovery package and enters the passphrase.
2. The device derives the wrapping key with Argon2id using the parameters stored in the package header.
3. The device decrypts the keyring and trust contents.
4. A new device identity and nonce domain are generated; the package does not reuse an old device identity.
5. The device replays the manifest chain and rebuilds the materialized view from a snapshot or from cloud/relay peers.
6. The recovery transaction is committed locally before any acknowledgement is sent.

## Wrong-passphrase behavior

- AEAD tag verification failure is the only signal of a wrong passphrase.
- No distinction is made between a corrupt package and a wrong passphrase.
- The UI must not reveal whether the package exists or is valid until the passphrase succeeds.
- A small delay is applied after repeated failures.

## Supported-version policy

- A decoder supports its own major version and all prior major versions that are not deprecated.
- A decoder rejects messages with a higher major version.
- A decoder accepts higher minor versions within the same major version, ignoring unknown optional fields.
- Deprecated major versions are removed only after all known clients have upgraded.

## Deprecation policy

- A major version is deprecated by publishing a new manifest with a `min_supported_version` field.
- Devices on a deprecated version can read but cannot author new operations.
- After a transition window, deprecated versions are rejected.

## Downgrade policy

- Downgrade to an older client version is allowed only if the older version supports the current message major version.
- A downgrade that loses data or security features is blocked by the local store compatibility check.
- Database migrations are forward-only; downgrade requires restoring from a recovery package.

## Rollback policy

- Rollback of the application or OS is treated as a counter/nonce uncertainty event.
- The device enters repair mode and reconciles with a trusted snapshot or peer before authoring new operations.
- Rollback to a version with known vulnerabilities is blocked by the protocol version check.

## Non-secret stable error taxonomy

All errors returned to the user or logged operationally use stable codes. Error messages must not include keys, passphrases, decrypted content, or recovery material.

| Code | Meaning |
|---|---|
| `auth_required` | Authentication needed. |
| `forbidden` | Authenticated but not authorized. |
| `invalid_version` | Message version not supported. |
| `invalid_envelope` | Envelope structure or canonical encoding invalid. |
| `signature_invalid` | Signature verification failed. |
| `aead_decryption_failed` | AEAD tag mismatch or wrong passphrase. |
| `nonce_reused` | Nonce/counter replay detected. |
| `unknown_epoch` | Epoch not in manifest chain. |
| `unauthorized_device` | Device not authorized in current epoch. |
| `cursor_expired` | Cloud/relay cursor is stale. |
| `rate_limited` | Request rate exceeded. |
| `payload_too_large` | Message exceeds size limit. |
| `service_unavailable` | Service temporarily unavailable. |
| `repair_required` | Local state inconsistent; repair from snapshot. |
| `wrong_passphrase` | Recovery passphrase incorrect (indistinguishable from corrupt package). |
