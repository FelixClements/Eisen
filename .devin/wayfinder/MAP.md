---
labels: wayfinder:map
skills: domain-modeling, grilling, research, prototype
---

## Destination

Eisen is redesigned as a **local-first, installable PWA built with Leptos**, compiled from the Rust `eisen-core` compiled to WASM. The PWA replaces both the native Windows and Android clients, stores encrypted vault data in the browser's **Origin Private File System (OPFS)**, and uses **Web Crypto** for a per-browser-profile device identity. It is deployed on Cloudflare's free tier as a **Worker with Static Assets**. P2 remains local/offline; the Cloudflare backend (**D1** for sync metadata, **R2** for encrypted backups/snapshots) is added in P3.

## Notes

- Skills to consult during this map: `domain-modeling`, `grilling`, `research`, `prototype`.
- Build language: Rust (Leptos UI + `eisen-core` in WASM); deployment with `trunk`/`cargo-leptos` and `wrangler`.
- Browser APIs: `web-sys`, `wasm-bindgen`, Web Crypto, OPFS, service worker, PWA manifest.
- Keep existing `clients/android` and `clients/windows` directories for now; create `clients/pwa` or `clients/web`.
- Update `phasing-plan.md` and ADRs as decisions close, not before.

## Decisions so far

- [TICKET-01 Destination and scope for the Eisen redesign](tickets/TICKET-01-destination.md) — local-first Leptos PWA, Cloudflare Workers + Static Assets, P2 local, P3 cloud, existing clients kept for now.

## Open tickets (frontier)

- [TICKET-02 Choose the Leptos build and deployment toolchain](tickets/TICKET-02-build-toolchain.md)
- [TICKET-03 Prototype `eisen-core` compiled to WASM](tickets/TICKET-03-core-wasm.md)
- [TICKET-04 Design OPFS-backed `SecureStorage` and `ClockStorage`](tickets/TICKET-04-opfs-storage.md)
- [TICKET-05 Design browser-profile device identity and key lifecycle](tickets/TICKET-05-device-identity.md)
- [TICKET-06 PWA manifest, service worker, and install UX](tickets/TICKET-06-pwa-shell.md)
- [TICKET-07 Update `phasing-plan.md` and ADRs for the new stack](tickets/TICKET-07-plan-adr-updates.md)
- [TICKET-08 Cloudflare Worker + Static Assets project layout](tickets/TICKET-08-cloudflare-deploy.md)

## Not yet specified

- P3 Cloudflare sync architecture (what exactly lives in D1 vs R2, account/auth model, sync protocol).
- PWA install behavior on a locked-down work laptop if the browser policy blocks installation.
- Recovery-package UX and cross-browser-profile device enrollment.
- Performance and OPFS storage limits for large vaults.
- Free-tier quota guardrails (D1, R2, Workers request limits).
- Data retention and deletion on Cloudflare.

## Out of scope

- Native Windows client beyond the PWA.
- Native Android client beyond the PWA (existing code is kept for reference, not extended).
- Paid-only Cloudflare products (e.g., Durable Objects, Workers Paid tier).
