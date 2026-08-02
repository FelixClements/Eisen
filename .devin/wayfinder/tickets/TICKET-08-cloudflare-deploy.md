---
labels: wayfinder:prototype
status: open
---

## Question

What is the minimal Cloudflare project layout for a Leptos PWA using Workers + Static Assets on the free tier?

## Context

- The PWA static assets (HTML, WASM, JS, CSS, manifest, service worker) are served by a Cloudflare Worker.
- Workers Static Assets is the chosen product.
- Free tier limits: 100,000 Worker requests/day, 1 GB R2, D1 5M queries/day / 500 MB storage.
- `wrangler` is the deployment tool.

## Deliverable

A throwaway `wrangler` project that deploys a "Hello world" Leptos PWA to a `*.workers.dev` or custom domain on the free tier, proving the toolchain.
