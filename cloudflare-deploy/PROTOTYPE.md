# Cloudflare Workers + Static Assets deployment prototype

This is a throwaway branch for ticket #16. It contains the smallest possible
layout that proves the Cloudflare toolchain.

## Layout

```
cloudflare-deploy/
  wrangler.toml   # Wrangler configuration
  dist/           # static assets (index.html, manifest.json)
    index.html
    manifest.json
```

For the real PWA, `dist/` will be `clients/pwa/dist/` and built by Trunk.

## Verified command

```bash
cd cloudflare-deploy
npx wrangler deploy --dry-run
```

Output:

```
✨ Read 2 files from the assets directory .../cloudflare-deploy/dist
Total Upload: 0.34 KiB / gzip: 0.24 KiB
No bindings found.
--dry-run: exiting now.
```

## Correct `wrangler.toml` for P2 (no worker, static assets only)

```toml
name = "eisen-pwa"
compatibility_date = "2026-08-02"

[assets]
directory = "dist"
not_found_handling = "single-page-application"
```

## P3 extension

When an API worker is added, add `main` and the `ASSETS` binding:

```toml
name = "eisen-pwa"
main = "src/worker.ts"
compatibility_date = "2026-08-02"

[assets]
directory = "dist"
binding = "ASSETS"
not_found_handling = "single-page-application"
```

## To actually deploy

```bash
npx wrangler login   # one-time
npx wrangler deploy
```
