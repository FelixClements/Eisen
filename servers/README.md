# Servers

Cloud-sync service, relay service, account/session APIs, and operational tooling.

Servers must never hold decryption keys or plaintext task content. They store and serve opaque encrypted envelopes and snapshots, validate device signatures, and manage account authentication, quotas, cursors, and retention.

The P3 cloud backend is implemented on **Cloudflare Workers** with **D1** for metadata and **R2** for encrypted blobs. The existing Go server code in `servers/` is from the previous stack and is not in scope until/unless the relay server (P5) or a self-hosted backend is explicitly re-adopted.

Owner: project owner / backend

## Contents

- `cloudflare/` — Cloudflare Worker(s) for cloud sync (`/v1/append`, `/v1/read`, `/v1/snapshot/*`, `/v1/account/token`).
- `cloud/` — Go cloud-sync HTTP API from the previous stack. Kept for reference only.
- `relay/` — volatile peer-to-peer relay from the previous stack. Kept for reference only.
- `internal/` — shared server packages from the previous stack.

See `docs/adr/012-server-stack.md` for the stack decision and rationale.
