# PWA shell prototype

This is a throwaway `clients/pwa` scaffold for ticket #14.

## Run locally

Install `trunk` and `wasm32-unknown-unknown` once:

```bash
cargo install trunk
cd clients/pwa
```

Development server:

```bash
trunk serve
```

Production build:

```bash
trunk build --release
npx wrangler deploy
```

## What is here

- `Cargo.toml` — Leptos 0.8 CSR with `eisen-pwa` crate.
- `Trunk.toml` — builds `index.html` to `dist/`.
- `index.html` — root page with Trunk asset links and service worker registration.
- `public/manifest.json` — PWA install manifest.
- `public/service-worker.js` — runtime cache for offline use.
- `public/style.css` + `public/icon.svg` — minimal shell.
- `src/lib.rs` / `src/main.rs` — tiny Leptos app.
- `wrangler.toml` — Cloudflare Workers + Static Assets deployment.

## Design decisions this prototype validates

- Trunk is the build tool (from #10).
- CSR-only Leptos app with `mount_to_body`.
- PWA manifest + service worker for install and offline.
- Static assets deployed with `wrangler` + `[[assets]]`.
- No `.exe` download; the URL is the app.
