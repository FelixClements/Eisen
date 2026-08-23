# CI and local checks

The pipeline lives in `.github/workflows/ci.yml` and runs on every push and pull request to `main`.

## Jobs

| Job | Purpose | Blocking |
|-----|---------|----------|
| `structure` | Root has `src/`, `package.json`, `wrangler.toml`, `CONTEXT.md`, ADR-013 | yes |
| `markdown-lint` | Lint Markdown files | yes |
| `secret-scan` | TruffleHog secret scan | yes |
| `license-check` | `LICENSE` exists | yes |
| `web-check` | `npm ci && npm run check` | yes |
| `web-test` | `npm run test` | yes |
| `web-build` | `npm run build` | yes |
| `web-deploy` | Deploy to Cloudflare Pages `eisen-web` on `main` push | yes when secrets exist |
| `dependency-scan` | OSV scanner on lockfiles | yes when lockfiles exist |
| `sbom` | SPDX SBOM artifact | yes |

## Local commands

Run all checks:

```bash
./ops/run-local-checks.sh
```

Run individually:

```bash
./tools/verify-structure.sh
npm ci && npm run check
npm run test
npm run build
```

Deploy requires `CLOUDFLARE_API_TOKEN` and a Pages project named `eisen-web` (see `wrangler.toml`).
