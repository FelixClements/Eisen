# Web-only repo prune

Research for flattening `clients/web` to the repo root and deleting every other client, the Rust protocol track, and protocol-era docs.

## Product decision

ADR-013 names the SvelteKit web client as the active product. It uses one Account password, Better Auth, Dexie ciphertext, and record last-write-wins sync. It does not implement the frozen Rust/HLC/envelope protocol from ADRs 001-012.

Source: `docs/adr/013-web-one-password-e2ee.md`, `CONTEXT.md`

## What was removed

| Path | Reason |
|------|--------|
| `clients/android`, `clients/pwa`, `clients/pwa-svelte`, `clients/windows` | Not the product |
| `core/`, `protocol/`, `storage/`, `servers/`, `tests/` | Rust/protocol track; web does not use them |
| `cloudflare-deploy/`, `.devin/` | Legacy scratch and Wayfinder tickets |
| `docs/adr/001` through `012`, `docs/specs/`, `docs/threat-model/` | Superseded by ADR-013 |
| Obsolete `docs/research/*` | android/pwa-svelte rebuild notes |
| Five Rust/Android workflow files | No longer applicable |

## What remains

| Path | Role |
|------|------|
| `src/`, `static/`, `migrations/` | SvelteKit app |
| `package.json`, `wrangler.toml` | npm project and Cloudflare bindings |
| `CONTEXT.md` | Domain glossary |
| `docs/adr/013-web-one-password-e2ee.md` | Web track ADR |
| `docs/agents/` | Agent guidance |
| `docs/CI.md` | Web-only pipeline |
| `.cursor/agents/web-repo-maintainer.md` | Repo maintenance subagent |

## CI after prune

Jobs in `.github/workflows/ci.yml`:

- `web-check`, `web-test`, `web-build` at repo root
- `web-deploy` to Cloudflare Pages project `eisen-web` on `main` push
- `structure` via `tools/verify-structure.sh` (checks root app layout)

Local parity: `./ops/run-local-checks.sh` runs structure check plus `npm ci`, `check`, `test`, `build`.

## Build notes

`@vite-pwa/sveltekit` with SvelteKit 2.70 emits `service-worker.mjs` while the PWA plugin expects `.js` during `injectManifest`. The build uses `injectionPoint: false` in `vite.config.ts` to skip the broken inject step. Push handling in `src/service-worker.ts` still ships; precache manifest injection may need a follow-up.

`VaultParamsExistError` lives in `src/lib/workspace/types.ts` so `memory-cloud.ts` does not import server code into the client bundle.

## Risks

- Confirm `CLOUDFLARE_API_TOKEN` and Pages project `eisen-web` exist before relying on `web-deploy`.
- GitHub issues may still link to deleted paths.
