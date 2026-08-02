---
labels: wayfinder:map
skills: domain-modeling, grilling, research, prototype
github_issue: https://github.com/FelixClements/Eisen/issues/8
---

> This map is now tracked as [GitHub issue #8](https://github.com/FelixClements/Eisen/issues/8). The local markdown is a mirror; issue numbers below point to GitHub.

## Destination

Eisen is redesigned as a **local-first, installable PWA built with Leptos**, compiled from the Rust `eisen-core` compiled to WASM. The PWA replaces both the native Windows and Android clients, stores encrypted vault data in the browser's **Origin Private File System (OPFS)**, and uses **Web Crypto** for a per-browser-profile device identity. It is deployed on Cloudflare's free tier as a **Worker with Static Assets**. P2 remains local/offline; the Cloudflare backend (**D1** for sync metadata, **R2** for encrypted backups/snapshots) is added in P3.

## Notes

- Skills to consult during this map: `domain-modeling`, `grilling`, `research`, `prototype`.
- Build language: Rust (Leptos UI + `eisen-core` in WASM); deployment with `trunk`/`cargo-leptos` and `wrangler`.
- Browser APIs: `web-sys`, `wasm-bindgen`, Web Crypto, OPFS, service worker, PWA manifest.
- Keep existing `clients/android` and `clients/windows` directories for now; create `clients/pwa` or `clients/web`.
- Update `phasing-plan.md` and ADRs as decisions close, not before.

## Decisions so far

- [#9 Destination and scope for the Eisen redesign](https://github.com/FelixClements/Eisen/issues/9) — local-first Leptos PWA, Cloudflare Workers + Static Assets, P2 local, P3 cloud, existing clients kept for now.
- [#10 Choose the Leptos build and deployment toolchain](https://github.com/FelixClements/Eisen/issues/10) — use **Trunk** for the CSR build; deploy with **Wrangler + Cloudflare Workers Static Assets**.
- [#11 Prototype `eisen-core` compiled to WASM](https://github.com/FelixClements/Eisen/issues/11) — the core **compiles** to `wasm32-unknown-unknown` with one `getrandom/js` target dependency. Browser storage is the remaining work.
- [#12 Design OPFS-backed `SecureStorage` and `ClockStorage`](https://github.com/FelixClements/Eisen/issues/12) — use **OPFS + Web Worker**; `OpfsSecureStorage` and `OpfsClockStorage` implement the synchronous core traits inside a worker.
- [#13 Design browser-profile device identity and key lifecycle](https://github.com/FelixClements/Eisen/issues/13) — device keys in Rust/WASM; `OpfsSecureStorage` encrypted under vault passphrase; each browser profile is a new device; recovery package contains owner trust + epoch roots, not the old device key.
- [#14 PWA manifest, service worker, and install UX](https://github.com/FelixClements/Eisen/issues/14) — `clients/pwa` is a **Trunk + Leptos 0.8 CSR** app with PWA manifest, runtime-caching service worker, and `wrangler.toml` for Static Assets.

## Open tickets (frontier)

- [#15 Update `phasing-plan.md` and ADRs for the new stack](https://github.com/FelixClements/Eisen/issues/15)
- [#14 PWA manifest, service worker, and install UX](https://github.com/FelixClements/Eisen/issues/14)
- [#15 Update `phasing-plan.md` and ADRs for the new stack](https://github.com/FelixClements/Eisen/issues/15)
- [#16 Cloudflare Worker + Static Assets project layout](https://github.com/FelixClements/Eisen/issues/16)

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
