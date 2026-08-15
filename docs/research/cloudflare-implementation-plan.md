# Cloudflare Deployment Implementation Plan

This is the actionable, step-by-step implementation plan derived from `cloudflare-deployment-plan.md`. It is ordered so that each step can be executed and verified before moving to the next.

**Status:** Phases 1–3 have been completed in the current environment. Phase 4 items are optional future work.

---

## Phase 0: Pre-conditions

| # | Step | Verification |
|---|------|--------------|
| 1 | Confirm you have a Cloudflare account with Workers & Pages enabled. | You can open https://dash.cloudflare.com/ and see a zone/account. |
| 2 | Confirm the PWA builds locally with `npm run build`. | `npm run build` exits 0 and `.svelte-kit/cloudflare` is created. |
| 3 | Confirm the Playwright suite passes locally. | `npx playwright test` exits 0. |
| 4 | Confirm `wrangler` is installed and authenticated. | `npx wrangler --version` prints a version; `npx wrangler whoami` shows your account. |

---

## Phase 1: One-Time Resource Provisioning

Goal: create the Cloudflare resources the PWA needs and capture their IDs.

| # | Step | Command | Verification |
|---|------|---------|--------------|
| 1 | Authenticate Wrangler with Cloudflare. | `npx wrangler login` | Browser OAuth completes; `npx wrangler whoami` shows account. |
| 2 | Create the D1 database. | `npx wrangler d1 create eisen-db` | Output shows `database_id`. |
| 3 | Create the KV namespace. | `npx wrangler kv namespace create KV` | Output shows `id`. |
| 4 | Enable R2 in the Cloudflare dashboard and create the bucket. | `npx wrangler r2 bucket create eisen-attachments` | Output shows `Created bucket 'eisen-attachments'`. |
| 5 | Create the Pages project. | `npx wrangler pages project create eisen-svelte-pwa --production-branch=main` | Output shows `Successfully created the 'eisen-svelte-pwa' project`. |

**Status:** Completed. Created IDs:

- D1: `7cf88082-4cb8-4ba8-ad03-c26db5f710c4`
- KV: `70281fa565714be7b7c26af569556071`
- R2 bucket: `eisen-attachments`
- Pages project: `eisen-svelte-pwa`

---

## Phase 2: Local Configuration

Goal: update `wrangler.toml` so the local build and preview use the correct bindings.

| # | Step | Command / Action | Verification |
|---|------|------------------|--------------|
| 1 | Open `clients/pwa-svelte/wrangler.toml`. | — | File exists at `clients/pwa-svelte/wrangler.toml`. |
| 2 | Replace the placeholder D1 `database_id`. | Edit `[[d1_databases]]` `database_id` to the real UUID. | `wrangler.toml` contains the real `database_id`. |
| 3 | Add a `preview_database_id` for local development. | Add `preview_database_id = "local-eisen-db"` under the D1 binding. | `preview_database_id` is present. |
| 4 | Replace the placeholder KV `id`. | Edit `[[kv_namespaces]]` `id` to the real ID. | `wrangler.toml` contains the real KV `id`. |
| 5 | Confirm the R2 `bucket_name` is `eisen-attachments` and the binding is `ATTACHMENTS`. | Edit `[[r2_buckets]]` if needed. | `bucket_name = "eisen-attachments"` and `binding = "ATTACHMENTS"`. |
| 6 | Confirm `[vars]` include `APP_NAME` and `RECORD_SCHEMA_VERSION`. | Verify or add the block. | `APP_NAME = "Eisen"` and `RECORD_SCHEMA_VERSION = "1"` are present. |

**Status:** Completed in commit `1a9c421`.

---

## Phase 3: Database Migrations and First Deploy

Goal: prepare the production database, build the PWA, and deploy.

| # | Step | Command | Verification |
|---|------|---------|--------------|
| 1 | Apply migrations to the production D1 database. | `npx wrangler d1 migrations apply eisen-db --remote` | Tables `vault_records`, `accounts`, `devices`, `backups` exist. Verify with `npx wrangler d1 execute eisen-db --remote --command="SELECT name FROM sqlite_master WHERE type='table'"`. |
| 2 | (Optional) Apply local migrations for local preview. | `npx wrangler d1 migrations apply eisen-db --local` | Local `.wrangler/state/v3/d1` has the schema. |
| 3 | Build the application for Cloudflare Pages. | `npm run build` | `npm run build` exits 0 and `.svelte-kit/cloudflare` is created. |
| 4 | Deploy to production. | `npx wrangler pages deploy .svelte-kit/cloudflare --branch=main` | Output shows `Deployment complete!` with a URL. |
| 5 | Verify the site loads. | `curl -I https://eisen-svelte-pwa.pages.dev/` | HTTP 200. |
| 6 | Verify an API endpoint exists. | `curl -X POST https://eisen-svelte-pwa.pages.dev/api/sync -H 'Content-Type: application/json' -d '{}'` | HTTP 400 or 403 (not 500 or 404), meaning the Worker and D1 binding are reachable. |

**Status:** Completed. Live URL: `https://eisen-svelte-pwa.pages.dev/`

---

## Phase 4: Optional Future Work

These are not required for the first deployment, but are the natural next steps.

**Status:** Items 1–2 are in progress. A `pwa-check` job and a `pwa-deploy` job have been added to `.github/workflows/ci.yml`. The `pwa-deploy` job requires a `CLOUDFLARE_API_TOKEN` secret in the repository.

| # | Step | Status | Why | Notes |
|---|------|--------|-----|-------|
| 1 | Add a `preview` branch deploy workflow. | Not started | Let every feature branch get its own staging URL. | Use `npx wrangler pages deploy .svelte-kit/cloudflare --branch=<branch-name>` in GitHub Actions. |
| 2 | Add a GitHub Actions CI/CD pipeline. | In progress | Build, test, and deploy on push. | `pwa-check` runs `npm run check` and `npm run build`. `pwa-deploy` deploys the artifact to `eisen-svelte-pwa` on push to `main` using `CLOUDFLARE_API_TOKEN`. |
| 3 | Add a custom domain. | Replace `eisen-svelte-pwa.pages.dev` with your own domain. | Configure in the Pages dashboard under `eisen-svelte-pwa > Custom domains`. |
| 4 | Protect preview deployments with Cloudflare Access. | Prevent public preview URLs. | https://developers.cloudflare.com/pages/configuration/preview-deployments/ |
| 5 | Set up a secret if needed. | Any API keys the Worker needs. | `npx wrangler pages secret put API_KEY`. |
| 6 | Enable real-time log tailing for debugging. | Watch production Worker logs. | `npx wrangler pages deployment tail <url> --project-name=eisen-svelte-pwa` |
| 7 | Verify PWA installation and service worker in production. | Confirm `manifest.webmanifest` and `sw.js` are served. | Check the Network tab for `registerSW.js` and install prompt on mobile. |
| 8 | Monitor analytics and real user metrics. | Track performance and errors. | Use Cloudflare Analytics and the `Real-time logs` tab in the dashboard. |

---

## Rollback Plan

If a deployment breaks the site:

1. Roll back in the Cloudflare Dashboard under **Workers & Pages > eisen-svelte-pwa > Deployments**.
2. Select the previous working deployment and click **Activate**.
3. If the database schema is broken, do **not** roll back D1 migrations manually without a backup; instead fix forward with a new migration file.
4. For bad `wrangler.toml` changes, revert the file, rebuild, and redeploy.

---

## Common Issues Checklist

| Symptom | Likely Cause | Fix |
|---------|--------------|-----|
| `D1 binding not configured` (500) | `wrangler.toml` missing real `database_id` or `preview_database_id`. | Check and fix `wrangler.toml`, then rebuild and redeploy. |
| `KV binding not configured` (500) | `wrangler.toml` missing real KV `id`. | Update KV `id` and redeploy. |
| `R2 binding not configured` (500) | R2 not enabled in the dashboard or wrong `bucket_name`. | Enable R2 in the dashboard and ensure `bucket_name` matches. |
| Sync/backup returns 403 | Device is not enrolled in the `devices` table. | Generate a pairing code in Settings (initiates the device) or use the pairing claim flow on a new device. |
| `Incorrect passphrase` after pairing | `claimPairingCode` overwrote the local account with empty salt/validation. | Clear site data and create a new account, or import a recovery package. |

---

## Required GitHub Secrets (for CI/CD deploy)

If you want GitHub Actions to deploy automatically, add this secret at `https://github.com/FelixClements/Eisen/settings/secrets/actions`:

- `CLOUDFLARE_API_TOKEN` — create at https://dash.cloudflare.com/profile/api-tokens with the **Cloudflare Pages** and **Cloudflare Workers** templates.

`wrangler.toml` already contains `name = "eisen-svelte-pwa"`, so Wrangler will find the project using the token.

## Verification Commands

```bash
# Confirm local build
npm run build

# Confirm tables exist in production
npx wrangler d1 execute eisen-db --remote --command="SELECT name FROM sqlite_master WHERE type='table'"

# Tail production logs
npx wrangler pages deployment list --project-name=eisen-svelte-pwa
npx wrangler pages deployment tail "https://<deployment-url>" --project-name=eisen-svelte-pwa

# Smoke-test the live site
curl -s -o /dev/null -w "%{http_code}" https://eisen-svelte-pwa.pages.dev/
curl -s -X POST https://eisen-svelte-pwa.pages.dev/api/sync -H 'Content-Type: application/json' -d '{}'
```
