# Cloud API Contract

## Transport

- All endpoints use HTTPS with TLS 1.3.
- Authentication is via short-lived service bearer tokens tied to the account. Authorization for vault operations is via device Ed25519 signatures on request bodies.
- Service tokens do not authorize vault operations; they only authenticate the account/session.

## Endpoints

### `POST /v1/append`

Append one or more encrypted envelopes to the vault.

**Request body:**

```json
{
  "ver": 65536,
  "vault_id": "<base64>",
  "items": [
    {
      "op_id": "<base64>",
      "digest": "<base64>",
      "envelope": "<base64>"
    }
  ]
}
```

**Response body:**

```json
{
  "results": [
    {
      "op_id": "<base64>",
      "status": "accepted | conflict | rejected",
      "receipt": "<base64>",
      "error": "<optional stable error code>"
    }
  ]
}
```

**Semantics:**

- Append is idempotent: submitting the same `op_id` and `digest` returns the original `receipt`.
- If `op_id` matches but `digest` differs, the item is rejected as a conflict.
- If `op_id` is new and `digest` is unique, the item is accepted and a durable receipt is returned.
- The receipt is a digest-bound token that includes the operation ID, vault ID, and accepted timestamp.
- The service may reject oversized requests or rate-limited clients with explicit limit responses.

### `POST /v1/read`

Read encrypted envelopes from the vault.

**Request body:**

```json
{
  "ver": 65536,
  "vault_id": "<base64>",
  "cursor": "<opaque cursor>",
  "limit": 100
}
```

**Response body:**

```json
{
  "items": [
    {
      "op_id": "<base64>",
      "digest": "<base64>",
      "envelope": "<base64>"
    }
  ],
  "next_cursor": "<opaque cursor>",
  "expiry": "<ISO-8601>"
}
```

**Semantics:**

- Cursors are opaque to the client and expire after a server-defined period.
- Items are returned in an order consistent with the server's internal log. The cursor does not define task ordering.
- If the cursor is expired or invalid, the server returns an error that instructs the client to bootstrap from a snapshot.
- `limit` is capped by the server; requests with higher limits are clamped or rejected.

### `POST /v1/snapshot/advertise` and `POST /v1/snapshot/download`

Advertise or download an encrypted snapshot.

- Snapshots are opaque to the service.
- The service validates the owner signature and stores the encrypted blob.
- Download returns the blob and its advertised checkpoint.

### `POST /v1/account/token`

Refresh the short-lived service token.

- Requires account credentials.
- Returns a token valid for a bounded time (default 15 minutes).

## Retry outcome semantics

- **2xx with receipt:** the operation is durable; the client may mark the outbox item delivered.
- **4xx (client error):** the request is not retried as-is; the client must fix the request or surface the error.
- **5xx (server error):** the client may retry with exponential backoff. A later 2xx with the same `op_id` and `digest` confirms durability.
- **Timeout / network failure:** the client may retry. It must accept possible duplicate processing and rely on idempotent `op_id`/`digest` checks.

## Bounded requests

- Maximum request body size: 1 MiB.
- Maximum `items` per append: 100.
- Maximum `limit` per read: 1,000.
- Default cursor expiry: 5 minutes.
- Per-account rate limits are enforced and surfaced through standard limit responses.

## Retention

- Envelopes are retained for the account's retention period.
- Deleted envelopes are not immediately purged; they are marked and removed after the retention window.
- Snapshot blobs are retained for at least 90 days.

## Error taxonomy

Errors use stable, non-secret codes:

- `invalid_version`
- `invalid_cursor`
- `cursor_expired`
- `conflict`
- `rate_limited`
- `payload_too_large`
- `auth_required`
- `forbidden`
- `service_unavailable`
