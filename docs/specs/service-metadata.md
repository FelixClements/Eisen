# Service Metadata Limitations

## What the cloud service can see

- Vault identifier (opaque 256-bit value).
- Operation IDs and digests (opaque hashes).
- Encrypted envelope bytes.
- Snapshot blob digests and sizes.
- Cursor, checkpoint, and sequence numbers.
- Account and device authentication tokens (not keys).
- Non-content operational signals: request rate, payload size, error codes, timestamps.

## What the cloud service cannot see

- Task plaintext: titles, notes, due dates, completion status, or quadrant.
- User passphrase.
- Owner or device private keys.
- Epoch keys.
- Recovery package contents.

## What the relay service can see

- Volatile routing state: which capabilities are connected and which peer IDs are reachable.
- Encrypted frame bytes in transit. Payloads are not decrypted, persisted, or logged.
- Frame types and sizes for rate-limiting and abuse detection.

## What is not logged

- Keys, passphrases, decrypted task data, recovery material, enrollment capabilities, or full encrypted payloads.
- Logs contain only non-content metadata and stable error codes.

## Privacy boundaries

- Service operators cannot read task content.
- Service operators cannot reset passphrases or recover vaults.
- Aggregate operational metrics do not reveal individual task counts or content.
