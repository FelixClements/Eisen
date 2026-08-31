# Eisen Web

SvelteKit + Konsta UI + Better Auth. One account password signs you in and encrypts tasks in the browser. The server stores opaque blobs only.

## Development

```bash
npm install
npx wrangler d1 migrations apply eisen-web-db --local
npm run test
npm run dev
```

Production schema (after deploy CI also runs this):

```bash
npx wrangler d1 migrations apply eisen-web-db --remote
```

Copy `.env.example` to `.env` and set `BETTER_AUTH_SECRET`, `BETTER_AUTH_URL`, and optionally `VITE_VAPID_PUBLIC_KEY`. When D1 is bound (`npm run preview` / Pages), both auth env vars are required — there is no in-code fallback secret.

Use `npm run preview` (Wrangler Pages dev) to test push APIs with D1 bindings. Plain `npm run dev` does not provide Cloudflare env bindings for wake dispatch.

Preview deployments use a separate D1 database (`eisen-web-db-preview`) and R2 bucket (`eisen-attachments-preview`). Do not point `preview_database_id` at production.

## Push notifications

Generate VAPID keys:

```bash
npx @pushforge/builder vapid
```

Configure:

| Variable | Where |
|----------|-------|
| `VITE_VAPID_PUBLIC_KEY` | `.env` locally; GitHub Actions secret `VAPID_PUBLIC_KEY` for CI builds |
| `VAPID_PRIVATE_KEY` | Cloudflare Pages secret (JWK JSON string from command above) |
| `VAPID_SUBJECT` | Cloudflare Pages env (`mailto:you@example.com`) |
| `CRON_SECRET` | Cloudflare Pages secret **and** `eisen-push-cron` Worker secret (same value; protects `/api/push/cron`) |

Pages does not support cron triggers. Deploy the standalone cron Worker (runs every minute, calls `/api/push/cron`):

```bash
# Set CRON_SECRET on the cron Worker (must match Pages)
cd workers/push-cron
npx wrangler secret put CRON_SECRET
# Edit TARGET_ORIGIN in workers/push-cron/wrangler.toml if not using eisen-web.pages.dev
cd ../..
npm run deploy:push-cron
```

After deploy, enable push in **Settings → Enable push reminders**, then create a task with a reminder time.

A second device: sign in with the same email and password. No vault setup and no restore.
