# Servers

Cloud-sync service, relay service, account/session APIs, and operational tooling. Implemented in Go.

Servers must never hold decryption keys or plaintext task content. They store and serve opaque encrypted envelopes and snapshots, validate device signatures, and manage account authentication, quotas, cursors, and retention.

Owner: project owner / backend

## Contents

- `cloud/` — cloud-sync HTTP API (`/v1/append`, `/v1/read`, `/v1/snapshot/*`, `/v1/account/token`).
- `relay/` — volatile peer-to-peer WebSocket relay (`eisen-relay-v1`).
- `internal/` — shared server packages (auth, quotas, storage, cursors, logging).

See `docs/adr/012-server-stack.md` for the stack decision and rationale.
