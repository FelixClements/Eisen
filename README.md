# Eisen Web

SvelteKit + Konsta UI + Better Auth. One account password signs you in and encrypts tasks in the browser. The server stores opaque blobs only.

## Development

```bash
npm install
npx wrangler d1 migrations apply eisen-web-db --local
npm run test
npm run dev
```

Copy `.env.example` to `.env` and set `BETTER_AUTH_SECRET`, `BETTER_AUTH_URL`, and optionally `VITE_VAPID_PUBLIC_KEY`.

Use `npm run preview` (Wrangler Pages dev) to test push APIs with D1 bindings. Plain `npm run dev` does not provide Cloudflare env bindings for wake dispatch.

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
| `CRON_SECRET` | Cloudflare Pages secret (random string; protects `/api/push/cron`) |

After deploy, enable push in **Settings → Enable push reminders**, then create a task with a reminder time.

A second device: sign in with the same email and password. No vault setup and no restore.
