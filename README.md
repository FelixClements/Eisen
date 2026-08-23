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

A second device: sign in with the same email and password. No vault setup and no restore.
