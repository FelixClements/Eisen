---
labels: wayfinder:research
status: open
---

## Question

Which Leptos build and deployment toolchain should we use for the Eisen PWA, and how does it integrate with the Rust `eisen-core` WASM build and Cloudflare `wrangler`?

## Context

- The PWA is a Leptos app.
- `eisen-core` must compile to `wasm32-unknown-unknown` and be consumed by Leptos directly (no JS glue).
- Deployment target is Cloudflare Worker with Static Assets.
- Free-tier constraints apply.

## Options to evaluate

- `trunk` with `wasm-bindgen`.
- `cargo-leptos`.
- A custom pipeline (`wasm-pack`/`wasm-bindgen-cli` + Vite/JS).
