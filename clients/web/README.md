# Eisen Web (`clients/web`)

SvelteKit + Konsta UI + Better Auth + encrypted local-first tasks.

## Development

```bash
cd clients/web
npm install
npx wrangler d1 migrations apply eisen-web-db --local
npm run dev
```

Copy `.env.example` to `.env` and set `BETTER_AUTH_SECRET`, `BETTER_AUTH_URL`, and optionally `VITE_VAPID_PUBLIC_KEY`.

## Architecture

- **Better Auth** — account sign-in (D1)
- **Vault passphrase** — encrypts tasks (IndexedDB + sync blobs)
- **Web Push** — wake-clock reminders without server-side plaintext
