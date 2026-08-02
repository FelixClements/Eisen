---
labels: wayfinder:grilling
status: closed
---

## Question

What is the destination for the Eisen redesign, given that the work laptop blocks `.exe` downloads and the project wants to use Leptos and Cloudflare's free tier?

## Answer

- Build a **Leptos PWA** with the Rust `eisen-core` compiled to WASM (`wasm32-unknown-unknown`).
- The PWA **replaces both native Windows and Android clients**.
- Encrypted vault data lives in the browser's **Origin Private File System (OPFS)**; the master key stays in memory while unlocked.
- Device identity is a **new per-browser-profile device** generated with **Web Crypto**; recovery package for transfer.
- Deploy on Cloudflare free tier: **Worker with Static Assets** for the PWA; **D1** and **R2** used later in P3 for metadata sync and encrypted backups.
- P2 remains **local-only/offline**; the Cloudflare backend is added in **P3**.
- Existing `clients/android` and `clients/windows` are **kept for now**; new `clients/pwa` created.
