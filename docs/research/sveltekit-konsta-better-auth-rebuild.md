# SvelteKit + Konsta UI + Better Auth Rebuild Research

Primary sources: [Konsta UI Svelte](https://konstaui.com/svelte), [Better Auth SvelteKit](https://better-auth.com/docs/integrations/svelte-kit), [Better Auth D1 adapters](https://better-auth.com/docs/adapters/other-relational-databases), [better-auth-cloudflare](https://github.com/zpg6/better-auth-cloudflare), Eisen ADRs and `clients/pwa-svelte`.

## 1. Stack integration

| Layer | Choice | Notes |
|-------|--------|-------|
| Framework | SvelteKit 2 + Svelte 5 | Runes, `{@render children()}` |
| Styling | Tailwind CSS v4 | `@import 'tailwindcss'` in `app.css`; no `tailwind.config.js` |
| UI | Konsta UI v5.3 | `npm i konsta`; import `konsta/svelte/theme.css` |
| Deploy | `@sveltejs/adapter-cloudflare` | D1/R2/KV via `platform.env` |
| PWA | `@vite-pwa/sveltekit` | Custom SW for push handlers |
| Local DB | Dexie 4 | Plaintext tasks in IndexedDB |
| Auth | Better Auth | Email/password; session cookies |
| Crypto | Web Crypto API | PBKDF2-SHA256 600k + AES-GCM |

### Konsta setup (`src/app.css`)

```css
@import 'tailwindcss';
@import 'konsta/svelte/theme.css';

@theme {
  --color-brand-primary: #0f766e;
  --color-brand-red: #ef4444;
  --color-brand-amber: #f59e0b;
  --color-brand-blue: #3b82f6;
  --color-brand-gray: #6b7280;
}
```

Roboto required for Material theme; iOS uses system font.

### Layout wrapper

```svelte
<App theme="ios" safeAreas>
  {@render children()}
</App>
```

Official SvelteKit guide still shows `<slot />` — use Svelte 5 snippets and `{@render children()}`.

## 2. Konsta UI patterns for Eisen

| Android concept | Konsta component |
|-----------------|------------------|
| ModalNavigationDrawer | `Panel` (`side="left"`, `opened`, `onBackdropClick`) |
| Priority ledger sections | `List` + `ListGroup` + sticky `groupTitle` |
| FAB add task | `Fab` with `bottom-safe-4 right-safe-4` |
| Undo snackbar | `Toast` |
| Discard confirmation | `Dialog` |
| In-app banner | `Notification` (not OS push) |

Quadrant colors: `@theme --color-brand-*` + `k-color-brand-red` etc. on section headers.

Icons: `framework7-icons/svelte` for native Konsta look; `useTheme()` switches iOS/Material icons.

## 3. Better Auth on D1

### Option A: Official (`better-auth` + `kysely-d1`)

- Community dialect: [kysely-d1](https://github.com/aidenwallis/kysely-d1)
- `database: { db: new Kysely({ dialect: new D1Dialect({ database: env.DB }) }), type: 'sqlite' }`
- Run schema via Better Auth CLI migrations

### Option B: `better-auth-cloudflare`

- `withCloudflare({ d1: { db, options } }, { emailAndPassword: { enabled: true } })`
- Drizzle adapter, built-in rate limiting, KV optional
- Context7: `/zpg6/better-auth-cloudflare` (460 snippets)

**Decision for `clients/web`:** Option A (kysely-d1) — fewer dependencies, matches Better Auth official docs.

### SvelteKit integration

```typescript
// src/hooks.server.ts
const session = await auth.api.getSession({ headers: event.request.headers });
if (session) {
  event.locals.session = session.session;
  event.locals.user = session.user;
}
return svelteKitHandler({ event, resolve, auth, building });
```

```typescript
// src/lib/auth.ts — plugins
plugins: [sveltekitCookies(getRequestEvent)]
```

Auth routes: `/api/auth/*` via `svelteKitHandler`.

## 4. Two-layer auth + vault crypto

| Layer | Purpose | Storage |
|-------|---------|---------|
| Better Auth | Identity, sessions | D1: user, session, account tables |
| Vault passphrase | Encrypt tasks | IndexedDB: vault row per userId; CryptoKey in memory or sessions table |

Flow:

1. Sign up / sign in (Better Auth)
2. First visit: set vault passphrase → derive key → encrypt validation token
3. Return visits: unlock vault (separate from account password)
4. Sync: session cookie + encrypted blobs

Crypto (from `pwa-svelte/src/lib/crypto.ts`):

- PBKDF2-SHA256, 600,000 iterations, 256-bit AES-GCM key
- IV prepended to ciphertext, base64 packed

**Onboarding copy:** "Your vault passphrase encrypts your tasks. We cannot recover it if you lose it. Your account password only controls sign-in."

## 5. D1 schema redesign

Better Auth tables created by CLI migration (`user`, `session`, `account`, `verification`).

Eisen tables (migration `0001_eisen.sql`):

```sql
CREATE TABLE vault_records (
  record_id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL,
  encrypted_blob BLOB NOT NULL,
  modified_at INTEGER NOT NULL,
  sync_version INTEGER NOT NULL,
  deleted INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE push_subscriptions (
  id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL,
  endpoint TEXT NOT NULL UNIQUE,
  p256dh TEXT NOT NULL,
  auth TEXT NOT NULL,
  created_at INTEGER NOT NULL
);

CREATE TABLE wake_schedules (
  id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL,
  device_id TEXT NOT NULL,
  wake_at INTEGER NOT NULL,
  nonce TEXT NOT NULL,
  sent INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE backups (
  package_id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL,
  r2_key TEXT NOT NULL,
  created_at INTEGER NOT NULL
);
```

Dropped from `pwa-svelte`: `accounts`, `devices`, pairing KV (Better Auth sessions replace device enrollment).

Sync API: `user_id` from `event.locals.user.id`; no `ownerId`/`deviceId` enrollment check.

## 6. Notification design (E2EE-preserving)

OS reminders when PWA closed require Web Push ([android-notifications.md](./android-notifications.md)).

1. Client: `Notification.requestPermission()` + `PushManager.subscribe({ applicationServerKey: VAPID_PUBLIC })`
2. POST `/api/push/subscribe` — store subscription in D1 (no task content)
3. Client computes next `reminderAt` → POST `/api/push/schedule` with `{ wakeAt, nonce }`
4. Cron/scheduled Worker sends generic push at `wakeAt` (VAPID private key in secrets)
5. Service worker `push` handler: read vault session from IndexedDB, decrypt due tasks, `showNotification(title)`

Server never sees task titles. Konsta `Notification` is in-app only.

## 7. Port map from `pwa-svelte`

| Source | Action |
|--------|--------|
| `lib/crypto.ts` | Copy as-is |
| `lib/db.ts` | Adapt: vault keyed by `userId`; remove owner pairing |
| `lib/vault.ts` | Adapt: require Better Auth user; separate vault setup |
| `lib/sync.ts` | Replace ownerId with session user.id |
| `lib/recovery.ts` | Scope to userId |
| `routes/api/sync` | Auth-gated; user_id from session |
| `routes/api/backup` | Auth-gated |
| `routes/api/pairing/*` | Drop |
| `routes/api/devices/enroll` | Drop |
| UI routes | Rebuild with Konsta components |

## 8. Non-goals

- Rust `eisen-core`, Leptos PWA, signed envelopes, X25519 pairing
- Android client changes
- OAuth / social login (email/password only at launch)
- Plaintext tasks on server
