# ADR-010: Cloud API Contract

## Status

Proposed / P0.10

## Context

P0 requires a cloud API contract covering append idempotency, operation-ID/digest conflict behavior, durable receipts, cursor ordering/expiry, retention, bounded requests, and retry outcome semantics.

## Decision

- The cloud service exposes a `/v1/append`, `/v1/read`, `/v1/snapshot/*`, and `/v1/account/token` REST API over HTTPS with TLS 1.3.
- Service authentication is via short-lived bearer tokens. Vault-operation authorization is via device Ed25519 signatures.
- Append is idempotent by `(op_id, digest)`. Matching `op_id` with different `digest` is a conflict.
- Durable receipts are digest-bound and returned for accepted operations.
- Read cursors are opaque, ordered by server log, and expire after 5 minutes.
- Request sizes and rates are bounded with explicit limit responses.
- Retention aligns with the 90-day epoch-key and snapshot window.
- Errors use a stable, non-secret taxonomy.

The full contract is in `docs/specs/cloud-api.md`.

## Consequences

- The service cannot decrypt task content; it only validates signatures and stores opaque envelopes and snapshots.
- Outbox retry logic on the client must be robust to timeouts and 5xx errors.
- Cursor expiry forces clients to use snapshots for long-lived catch-up.
