---
labels: wayfinder:prototype
status: open
---

## Question

How should we implement `SecureStorage` and `ClockStorage` for the browser using OPFS, so that `eisen-core` can persist encrypted data in the PWA?

## Context

- The core's storage is abstracted behind `SecureStorage` and `ClockStorage` traits.
- The browser backend should use the **Origin Private File System (OPFS)**.
- All writes must be encrypted; keys live in memory only.
- OPFS is async (via `web-sys` and `wasm-bindgen-futures`); the core traits are currently synchronous.

## Options to evaluate

- Wrap OPFS in an async layer and adapt the core to async storage.
- Use OPFS synchronous access handles where available.
- Buffer state in memory and flush to OPFS at transaction boundaries.
