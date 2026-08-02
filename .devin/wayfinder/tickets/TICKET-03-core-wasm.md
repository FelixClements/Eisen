---
labels: wayfinder:prototype
status: open
---

## Question

Can `eisen-core` compile to `wasm32-unknown-unknown` for direct use by Leptos, and what changes are needed to make it WASM-compatible?

## Context

- `clock.rs` uses `std::fs`/`std::io` for `FileStorage`.
- `vector_runner.rs` uses `std::fs` for input/output paths.
- `std::sync::Mutex` and `Send`/`Sync` may need single-threaded WASM handling.
- The core uses `SecureStorage` and `ClockStorage` traits, but the default file-backed implementations are not WASM-compatible.

## Deliverable

A throwaway branch or a `core-wasm-prototype` workspace that lists compile errors and a recommended fix for each.
