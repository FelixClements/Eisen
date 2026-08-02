---
labels: wayfinder:prototype
status: open
---

## Question

What should the PWA shell look like (manifest, service worker, offline caching) to support install-first use on a work laptop without any `.exe` download?

## Context

- The app should be installable as a PWA.
- Work laptops may block installations; the app must still be reachable as a URL.
- Service worker must cache the static app (HTML/WASM/JS/CSS) for offline use.
- OPFS is the source of truth for data; service worker does not access vault data.

## Options to evaluate

- Minimal `manifest.json` + hand-written service worker.
- Use a service-worker crate/tool (`workbox` via `trunk`, `sw-rs`, or Leptos built-in PWA support).
- Treat install as optional; ensure URL-first fallback.
