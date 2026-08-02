# ADR-012: Cloud and relay server stack

## Status

Accepted / Redesigned for Cloudflare Workers — 2026-08-02

## Context

The Eisen server boundary provides cloud-sync service, account/session APIs, encrypted snapshot storage, and (later) optional volatile relay service. The server must never hold decryption keys or plaintext task content. It stores and serves opaque encrypted envelopes and snapshots, validates device signatures, and manages account metadata, quotas, cursors, and retention.

The server stack was not fixed during P0 because the protocol contracts (D-010, D-011, and the transport specs) are independent of implementation language. After the client stack moved to a Leptos PWA, the cloud deployment is consolidated onto Cloudflare's free tier.

## Decision

### P2 (local-only)

- There is **no server**.
- Vault data stays in the browser's Origin Private File System.
- Recovery and cross-device transfer use a user-held encrypted recovery package.

### P3 (cloud-sync beta)

- **Cloud platform:** Cloudflare Workers (free tier) with Static Assets.
- **Metadata store:** Cloudflare D1 for account metadata, device manifests, cursors, and envelope references.
- **Blob store:** Cloudflare R2 for encrypted envelopes, signed snapshots, and recovery backups.
- **Worker language:** TypeScript (`wrangler`/`workerd`) for P3.01–P3.17; Rust (`workers-rs`) is an option if a single-language boundary is desired later. The choice is fixed by P3.01.
- **Build and deployment:** `wrangler deploy` from `clients/pwa/` for the PWA assets, and a separate worker directory (`servers/cloudflare/`) for the worker script.

### P5 (volatile relay)

- **Not decided here.** The volatile relay may use Cloudflare Durable Objects, a separate Go relay, or WebRTC/data channel. The decision is deferred until P5 and recorded separately.

### Security invariants

- The worker never decrypts task content; it only stores opaque encrypted bytes and validates signatures/manifests against public keys.
- The worker does not hold vault keys or passphrases.
- D1 contains only routing/operational metadata; R2 contains opaque blobs.

## Consequences

- No self-hosted Go server in P3.
- Global edge caching and low-latency reads from R2.
- Free-tier limits apply:
  - Workers requests: 100,000/day.
  - D1: 500,000 rows/day, 5,000 queries/day.
  - R2: 10M reads/month, 1M writes/month.
- The worker can be extended later with `main = "src/worker.ts"` and `assets.binding = "ASSETS"` to fetch static assets from worker code.
- Crypto/protocol validation can still call the shared Rust core (compiled to WASM or a native sidecar) if needed, but the worker primarily uses canonical validation and signature verification.

## Evidence

### Wrangler dry run (P2, static assets only)

```bash
cd cloudflare-deploy
npx wrangler deploy --dry-run
```

Result:

```
✨ Read 2 files from the assets directory .../dist
Total Upload: 0.34 KiB / gzip: 0.24 KiB
No bindings found.
--dry-run: exiting now.
```

### P3 worker deploy (to be verified in P3.01)

```bash
cd servers/cloudflare
npx wrangler deploy
```

This will be verified when the P3 worker skeleton is implemented.

## Relationship to other ADRs

- D-001 now fixes the Leptos PWA and the shared Rust/WASM core.
- D-003 fixes that all cryptographic operations live in the shared Rust core; the worker validates signatures/manifests but does not decrypt content.
- D-010 and the cloud API spec define the contract the Cloudflare worker must implement.
- D-011 and the recovery package spec define how a device restores without a server passphrase reset.
