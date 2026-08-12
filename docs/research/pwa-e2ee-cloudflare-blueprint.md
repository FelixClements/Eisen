# Offline-First, End-to-End Encrypted PWA Todo App — Cloudflare Free Tier Blueprint

**Scope:** Research only. No application source files are modified. This document is a single-file architectural blueprint that evaluates the stack, storage, cryptography, and Cloudflare infrastructure for an E2EE, offline-first PWA todo application that runs entirely on the Cloudflare Free Tier.

**Target:** A progressive web app (PWA) that stores todos locally, works offline, syncs when online, and keeps all user content end-to-end encrypted so the Cloudflare side of the platform sees only opaque blobs.

---

## 1. Language & Framework Analysis

### 1.1 Evaluation criteria

For an offline-first PWA on Cloudflare Pages/Workers, the framework choice is driven by:

1. **Production bundle size impact on initial service-worker caching.** A todo PWA should precache the HTML, CSS, JS, and offline fallback. Larger bundles increase install time, cache churn, and data usage on metered networks [20].
2. **Native Cloudflare Pages/Workers integration.** A first-party or officially supported adapter reduces runtime surprises and gives access to `env`/`platform` bindings (KV, D1, R2) without custom boilerplate.
3. **Reactive state management for rapid local-first sync.** The framework needs fine-grained reactivity (signals or observable stores) and must allow cheap, live reads from an IndexedDB source so the UI re-renders immediately when local data changes.

### 1.2 SvelteKit + TypeScript

SvelteKit is the application framework for Svelte. Cloudflare is a first-class target.

* **Cloudflare adapter:** The official `@sveltejs/adapter-cloudflare` supports all SvelteKit features and builds for both Cloudflare Workers Static Assets and Cloudflare Pages. The build output directory for Pages is `.svelte-kit/cloudflare` [1].
* **Runtime bindings:** The `event.platform` object passes the Cloudflare `env`, `ctx`, `caches`, and `cf` objects into hooks and endpoints, giving direct access to D1, KV, R2, and the Cache API [1].
* **Bundle size:** Svelte is a compiler, not a runtime virtual-DOM framework. It compiles components to vanilla JavaScript, producing very small bundles. Svelte 5’s fine-grained signal-based reactivity means updates to a single value in a list do not invalidate the rest of the list, which is ideal for large todo lists rendered from IndexedDB [2][5].
* **PWA/service worker ergonomics:** SvelteKit exposes a `$service-worker` module for custom service workers and integrates cleanly with `vite-plugin-pwa` or `workbox-build` [20].
* **Reactive state:** Svelte 5 introduces *runes* — explicit reactive primitives (`$state`, `$derived`, `$effect`) that can be used in `.svelte.js`/`.svelte.ts` modules, meaning shared reactive logic can live outside components [2]. Dexie’s `liveQuery()` returns an Observable that satisfies the Svelte store contract, so it can be consumed with the `$` prefix directly [6].

**Verdict:** Strongest fit. Minimal bundle, first-party Cloudflare adapter, and a reactivity model that is purpose-built for compiler-optimized, local-first UIs.

### 1.3 Next.js App Router + TypeScript

* **Cloudflare adapter:** Cloudflare officially supports Next.js on Workers through the `@opennextjs/cloudflare` adapter. It supports App Router, RSC, Server Actions, SSR, streaming, middleware, and static generation [3].
* **Build output:** Workers build into `.open-next/worker.js` with an `assets` directory; Pages is less emphasized now that Workers Assets is the preferred path [3].
* **Bundle size/impact:** Next.js is larger by default. The React runtime, client navigation, and App Router hydration code add meaningful weight, which increases the service-worker precache payload. That matters for a mobile PWA on a free tier with limited caching budget.
* **Reactivity model:** React relies on hooks/`useState`/Context. For live IndexedDB reads, you need a third-party store such as Zustand, Valtio, or Jotai, plus a polling or `BroadcastChannel`/Dexie hook layer. This is more plumbing than SvelteKit + Svelte runes.

**Verdict:** Viable if the team already knows React, but overkill for a local-first todo app and heavier on the wire/cache than SvelteKit.

### 1.4 Remix / React Router v8 + TypeScript

* **Cloudflare adapter:** Remix has been superseded by React Router v8. Cloudflare recommends creating a React Router v8 project with the Cloudflare Vite plugin, which runs server code in the Workers runtime during development [4].
* **Build output:** Default `workers/app.ts` is the Worker entry; assets are in `build/client` [4].
* **Bundle size:** React-based, similar runtime overhead to Next.js.
* **Reactive state:** Remix/React Router uses loaders/actions. The data layer is server-centric. For client-side IndexedDB reactivity, additional libraries are required.

**Verdict:** Good for full-stack React apps, but not the leanest path for an offline-first, client-encrypted PWA. The official guidance explicitly recommends React Router for new projects instead of Remix [4].

### 1.5 Other strong candidates

* **SolidStart / SolidJS:** Uses `createSignal` and `createStore` for extremely fine-grained, tiny-bundle reactivity. Cloudflare is a supported preset. A strong alternative if the team prefers explicit signals, but the ecosystem is smaller and there is less documentation for Pages/Workers than SvelteKit.
* **Qwik:** Resumability gives a tiny first interaction bundle, but the Cloudflare adapter is newer and the reactivity model is different enough that it may slow down local-first IndexedDB integration.
* **Astro with React/Svelte islands:** Excellent for static sites; less natural for a highly interactive, offline-first todo app where the client bundle must contain the entire data and crypto pipeline.

### 1.6 Signals vs. stores for local-first IndexedDB

For syncing from IndexedDB, the most important property is **fine-grained, observable reactivity**:

* **Svelte 5 runes (`$state`, `$derived`):** Compiler-level reactivity. Because the reactivity is explicit, logic can be extracted into `.svelte.ts` files and shared between components and the sync layer [2].
* **Svelte stores (`writable`, `readable`, `derived`):** Still valid; `liveQuery()` from Dexie is a Svelte-compatible store, so an entire `todo` list can be bound with `$todos` [6].
* **Solid signals/stores:** `createSignal` is the reactive primitive; `createStore` gives fine-grained, proxy-based object reactivity. Reads track at the property level, which is efficient for large lists, but the API is getter-function based [30].
* **Preact signals:** Lightweight signals that can be used with React or standalone. Good for integration, but less framework-specific support on Cloudflare than Svelte.

**Conclusion for this app:** SvelteKit 5 with Svelte runes, optionally layering Dexie `liveQuery()` as a Svelte store, gives the smallest bundle, the cleanest local-first reactivity, and the best Pages/Workers integration.

---

## 2. Local Storage & Client-Side Sync Specification

### 2.1 Storage options comparison

The browser stores todos in an offline-first database. We compare the most relevant options.

#### Raw IndexedDB

* **Pros:** Built-in, asynchronous, transactional, handles large objects and binary data.
* **Cons:** Verbose event-driven API, no native observability, no query engine beyond key ranges, manual schema migrations, and easy to misuse in multi-tab or worker contexts [7].

#### Dexie.js

Dexie is a minimal, Promise-based wrapper over IndexedDB.

* **API:** Declarative schema, fluent `where()` queries, transactions, and built-in live queries [6].
* **Reactivity:** `liveQuery()` returns an Observable that is a valid Svelte/Angular Observable and can be consumed as a Svelte store [6]. `useLiveQuery()` is available for React.
* **Bundle:** Small relative to full databases (≈ 50 KB minified, per community benchmarks [7]).
* **Sync primitives:** Dexie’s middleware/hooks let you observe `CREATE`/`UPDATE`/`DELETE` operations and build your own sync engine on top of IndexedDB [6]. Dexie Cloud exists but is a paid hosted service; this blueprint avoids it.

#### RxDB

RxDB is a local-first NoSQL database with a generic replication protocol.

* **Pros:** Storage-agnostic (IndexedDB, OPFS, SQLite), JSON Schema validation, Mango queries, built-in replication, encryption and compression plugins, multi-tab coordination.
* **Cons:** Much larger bundle (≈ 200 KB minified [7]), dependency on RxJS patterns, and unnecessary complexity for a single-user todo app.

#### WatermelonDB

WatermelonDB is a reactive database built on SQLite (React Native/Node) or LokiJS (web).

* **Pros:** Lazy loading, proven offline-first, sync primitives, RxJS-based reactivity.
* **Cons:** On the web it uses `LokiJSAdapter`, which is no longer actively maintained; you must bring your own backend and implement the sync protocol manually [8]. More suited to React Native than a modern browser PWA.

#### Other wrappers

* **idb (Jake Archibald):** Tiny promise wrapper, but provides no queries or reactivity [7].
* **localForage:** Key-value with IndexedDB fallback; no queries [7].
* **PouchDB:** Full CouchDB-compatible document store; heavy and slow on large IndexedDB datasets [7].

### 2.2 Recommended option: Dexie.js

For this PWA, **Dexie.js** is the best choice:

* It stays close to IndexedDB (the fastest available browser storage).
* It has first-class live queries that integrate with Svelte stores.
* The bundle is small enough not to dominate the service-worker precache payload.
* It gives us enough hooks to implement the custom sync engine we need, while the encryption happens outside the database layer, before values reach Dexie.

Schema declaration for the local IndexedDB could look conceptually like:

```ts
const db = new Dexie('EisenVault') as Dexie & {
  todos: EntityTable<TodoDoc, 'id'>;
};

db.version(1).stores({
  todos: 'id, modifiedAt, *collection, [collection+modifiedAt]'
});
```

Note: locally we still store plaintext fields (`title`, `completed`, `dueDate`) because the device is trusted after the user has unlocked the app. The encrypted payload is what leaves the device.

### 2.3 Sync conflict resolution for encrypted strings

End-to-end encryption makes the server-side record an opaque blob. The server cannot see `title`, `completed`, `dueDate`, or any other user content. Therefore:

* **CRDTs are not appropriate for the ciphertext.** A CRDT/field-union merge requires the server to understand the document structure. With E2EE, the server never has the key, so it cannot merge concurrent edits of the same todo [15].
* **Last-Write-Wins (LWW) with monotonic logical timestamps is the pragmatic choice.** Each record carries a `modifiedAt` (or `syncVersion`) generated by the client. When a conflict occurs — two devices update the same `id` with different content — the server keeps the record with the highest `modifiedAt` [15].
* **Tombstones must win.** A deleted todo should be represented as an encrypted tombstone and, in case of a concurrent edit, the deletion should take precedence. We recommend a `deleted` integer flag (or `deletedAt` timestamp) that acts as the tiebreaker [15].
* **Hybrid Logical Clock (HLC) or server-seen timestamp?** Because clients can have clock skew, use a **server-assigned `syncVersion` monotonic integer** returned by the API. The client stores the last `syncVersion` it received and only uploads changes since that version. Each successful write returns the new `syncVersion`. If the client needs offline resolution, it can fall back to a local HLC (`wallClock + counter`) that is eventually dominated by the server version.
* **Conflict UX:** When the client pulls a newer `modifiedAt` record that overwrites local pending edits, the app detects the fork and can either auto-apply the server record or present a manual merge UI using the locally decrypted plaintext copies. Because we cannot merge encrypted content on the server, any user-visible merge must happen on a single client.

---

## 3. End-to-End Encryption (E2EE) Cryptographic Specification

### 3.1 Design goals

* **Zero-knowledge server:** Cloudflare receives only opaque, authenticated ciphertext blobs. It cannot read, sort, or filter todo content.
* **No heavy crypto dependencies:** Use the browser’s native `crypto.subtle` Web Crypto API.
* **Keys never extractable:** Derived keys are marked `extractable: false` so raw key bytes cannot be exported from the browser’s key store [9].
* **Password-derived master key:** The user’s password (or better, a passphrase) derives an AES-256-GCM key via PBKDF2.

### 3.2 Key derivation

```ts
async function deriveMasterKey(password: string, salt: Uint8Array) {
  const encoder = new TextEncoder();
  const imported = await crypto.subtle.importKey(
    'raw',
    encoder.encode(password),
    'PBKDF2',
    false,
    ['deriveKey']
  );

  return crypto.subtle.deriveKey(
    {
      name: 'PBKDF2',
      salt,
      iterations: 600_000,
      hash: 'SHA-256'
    },
    imported,
    { name: 'AES-GCM', length: 256 },
    false,                 // extractable = false
    ['encrypt', 'decrypt']
  );
}
```

* **Algorithm:** PBKDF2 is the Web Crypto API key derivation function designed for low-entropy inputs such as passwords [9].
* **Hash:** SHA-256. SHA-1 is still safe for PBKDF2 but should be avoided in new code [10].
* **Salt:** A random or pseudo-random value of at least 16 bytes (128 bits). It does not need to be kept secret, but it must be unique per user and stored alongside the ciphertext so the same key can be re-derived [10].
* **Iterations:** 600,000 is consistent with OWASP guidance for PBKDF2-HMAC-SHA-256 when FIPS-140/NIST-grade protection is desired [13]. On low-end devices this may be slow, so the derivation should be run in a Web Worker or the app should show a progress indicator.
* **Key extractability:** The final `CryptoKey` is returned with `extractable: false`, which is a core Web Crypto property [9]. Raw key material is never available to JavaScript as bytes.

### 3.3 Data encapsulation

Each todo record is serialized, then encrypted with the derived master key.

```ts
async function encryptRecord(key: CryptoKey, plaintext: object): Promise<EncryptedRecord> {
  const iv = crypto.getRandomValues(new Uint8Array(12)); // 96-bit IV recommended
  const encoder = new TextEncoder();
  const data = encoder.encode(JSON.stringify(plaintext));

  const ciphertext = await crypto.subtle.encrypt(
    { name: 'AES-GCM', iv, tagLength: 128 },
    key,
    data
  );

  return {
    iv: arrayBufferToBase64(iv),
    payload: arrayBufferToBase64(ciphertext),
    version: 1
  };
}
```

* **Cipher:** AES-256-GCM. OWASP recommends AES with at least a 128-bit key (ideally 256-bit) and an authenticated mode such as GCM or CCM whenever available [13].
* **IV:** 96 bits (12 bytes), generated with `crypto.getRandomValues()` for every encryption operation. The IV must never be reused with the same key; it does not need to be secret and is safe to transmit in the clear [11].
* **Authentication tag:** 128 bits by default. GCM provides confidentiality, integrity, and authenticity in one operation [11].
* **Plaintext serialization:** JSON. For attachments, binary data should be wrapped and encrypted as an `ArrayBuffer`.
* **Wire encoding:** base64 or base64url. The server stores these as `TEXT` or `BLOB` fields. base64url is slightly more URL-safe and avoids the `+`/`/` padding issues in JSON [14].

### 3.4 Security trade-offs

* **No server-side sorting/filtering:** Because the server sees only ciphertext, it cannot query by `title`, `completed`, `dueDate`, or any user content. All listing, sorting, filtering, and searching must happen in the client against the decrypted local Dexie database. This is the fundamental cost of zero-knowledge architecture.
* **Irreversible password loss:** If the user forgets the password, the master key cannot be derived, and no password reset is possible without a recovery key. The server has no knowledge of the password or the key. The app should offer an optional recovery mechanism (e.g., a recovery key encrypted with a second key derived from a recovery passphrase and stored offline), but this is out of scope for the minimal PWA.
* **No deterministic encryption for search:** Never use deterministic encryption to allow server-side searching; it leaks patterns and can be attacked with frequency analysis. Local decryption plus a local search index is the safe alternative.
* **Local storage is trusted:** The plaintext in IndexedDB is only safe while the device is trusted. On a shared device, the app should purge the `CryptoKey` on page hide/timeout and re-derive it on unlock.
* **Browser CryptoKey lifetime:** `extractable: false` keys reside in the browser’s secure key store. They are not exportable, but an attacker with full device compromise could still misuse them through the running app. The app should never store the raw passphrase in memory longer than the derivation call.

### 3.5 Cryptographic workflow summary

1. User enters passphrase.
2. App derives an AES-256-GCM master key with PBKDF2, SHA-256, 600,000 iterations, and a per-user random salt.
3. Master key is `CryptoKey` with `extractable: false`.
4. On save: record → JSON → `crypto.subtle.encrypt({ name: 'AES-GCM', iv, tagLength: 128 }, key, plaintext)`.
5. IV + ciphertext + version are uploaded as an opaque payload.
6. On sync: pull encrypted records; decrypt locally; write to Dexie; local reactive UI updates.

---

## 4. Cloudflare Infrastructure Schema

### 4.1 Free Tier resource model

For a todo PWA, the free tier is more than enough for a moderately sized user base:

* **Cloudflare Pages:** 100 projects per account, 20,000 static files, builds limited to one at a time on the free plan [19].
* **Workers / Pages Functions:** 100,000 requests per day, 10 ms CPU time per request, 50 subrequests per invocation, 1 MB environment variables [19].
* **D1 (SQLite):** 10 databases on the free plan, 500 MB per database, 5 GB total storage, 50 queries per Worker invocation, 2 MB max string/BLOB/row size [16].
* **Workers KV:** 1 GB total storage, 100,000 reads/day, 1,000 writes/day, 25 MiB value size [19].
* **R2:** 10 GB-month free storage, plus 1 million Class A and 10 million Class B operations/month [19].

### 4.2 D1 SQL schema for encrypted data

The server stores only opaque metadata plus ciphertext. We intentionally avoid any column that could leak user context (`title`, `completed`, `priority`). Instead, `collection` is an unencrypted category name (e.g., `todos`) used only for sync routing; all actual content is in `enc_payload`.

D1 uses SQLite semantics. Supported JavaScript-to-D1 type conversions include `TEXT`, `INTEGER`, `REAL`, and `BLOB`; `ArrayBuffer` and typed-array values are written as `BLOB` and returned as an `Array` by the D1 binding API [17].

```sql
-- Users table: only login metadata. The server never has the user's password or key.
CREATE TABLE IF NOT EXISTS users (
  id              TEXT PRIMARY KEY,              -- opaque user UUID
  email_hash      TEXT UNIQUE,                  -- deterministic hash, not the email itself
  created_at      INTEGER NOT NULL,             -- millisecond timestamp
  auth_pub_key    BLOB                          -- optional public key for device auth
);

-- Vault records: opaque, encrypted todo documents.
CREATE TABLE IF NOT EXISTS vault_records (
  id              TEXT PRIMARY KEY,              -- record UUID (opaque to server)
  user_id         TEXT NOT NULL,
  collection      TEXT NOT NULL DEFAULT 'todos', -- sync routing only (e.g. 'todos', 'archive')
  enc_payload     BLOB NOT NULL,                -- AES-GCM ciphertext (base64 -> BLOB or TEXT)
  iv              BLOB NOT NULL,                -- AES-GCM IV (12 bytes)
  salt            BLOB NOT NULL,                -- per-user PBKDF2 salt (stored once per record or user)
  schema_version  INTEGER NOT NULL DEFAULT 1,   -- record format version
  modified_at     INTEGER NOT NULL,             -- HLC or wall clock (for LWW)
  sync_version    INTEGER NOT NULL,             -- server-assigned monotonic sync token
  deleted         INTEGER NOT NULL DEFAULT 0,   -- 0 = active, 1 = tombstone

  FOREIGN KEY (user_id) REFERENCES users(id)
);

-- Index for efficient "get all changed records since syncVersion X for this user".
CREATE INDEX IF NOT EXISTS idx_vault_sync
  ON vault_records(user_id, collection, sync_version);

-- Index for LWW conflict resolution.
CREATE INDEX IF NOT EXISTS idx_vault_modified
  ON vault_records(user_id, id, modified_at);
```

* **`enc_payload`:** Contains the base64-encoded ciphertext produced by `crypto.subtle.encrypt(...)`. On the server it is an opaque `BLOB` or `TEXT`. When using D1, base64 strings are most portable; for compactness you could store the raw `ArrayBuffer` as `BLOB`, but the D1 binding reads `BLOB` back as an `Array`, so the client must convert it back to `Uint8Array` [17].
* **`iv`:** 12-byte random IV stored as `BLOB`.
* **`salt`:** PBKDF2 salt. It may be stored once per user in the `users` table or duplicated in each record for self-contained recovery; for E2EE, it does not need to be secret.
* **`deleted`:** Plaintext tombstone flag. We deliberately keep it unencrypted so the server can garbage-collect old tombstones and clients can know a record was removed without decrypting it. If hiding deletion patterns is a threat, this can be encrypted as well, but then a separate metadata token must reveal the deletion for sync.
* **Booleans:** D1/SQLite has no native `BOOLEAN` type; booleans are stored as `INTEGER` 0/1 [17].
* **Row-size limit:** A single D1 row plus any `TEXT`/`BLOB` value cannot exceed 2 MB [16]. Todo text is tiny, but if you support file attachments, keep the binary in R2 and store an `r2_key` reference in the encrypted record.

### 4.3 Sync API (Pages Functions)

The API is deliberately thin because the server cannot read content.

```ts
// /functions/sync.ts — conceptual Cloudflare Pages Function
interface Env {
  DB: D1Database;
}

export const onRequestPost: PagesFunction<Env> = async (context) => {
  const { request, env } = context;
  // Authenticate user (e.g., signed JWT or session token)...
  const userId = await authenticate(request);
  const body = await request.json<SyncRequest>();

  // 1. Insert/update incoming encrypted records, assigning sync_version.
  for (const record of body.updates) {
    await env.DB.prepare(
      `INSERT INTO vault_records
         (id, user_id, collection, enc_payload, iv, salt, schema_version, modified_at, sync_version, deleted)
       VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8,
         (SELECT COALESCE(MAX(sync_version), 0) + 1 FROM vault_records WHERE user_id = ?2 AND collection = ?3), ?9)
       ON CONFLICT(id) DO UPDATE SET
         enc_payload = excluded.enc_payload,
         iv = excluded.iv,
         modified_at = excluded.modified_at,
         sync_version = (SELECT COALESCE(MAX(sync_version), 0) + 1 FROM vault_records WHERE user_id = ?2 AND collection = ?3),
         deleted = excluded.deleted
       WHERE excluded.modified_at > vault_records.modified_at;`
    )
    .bind(record.id, userId, record.collection, record.encPayload, record.iv,
          record.salt, record.schemaVersion, record.modifiedAt, record.deleted ? 1 : 0)
    .run();
  }

  // 2. Return any records newer than the client's last sync_version.
  const incoming = await env.DB.prepare(
    `SELECT id, collection, enc_payload, iv, salt, schema_version, modified_at, sync_version, deleted
     FROM vault_records
     WHERE user_id = ?1 AND collection = ?2 AND sync_version > ?3
     ORDER BY sync_version ASC`
  )
  .bind(userId, body.collection, body.lastSyncVersion)
  .all();

  return Response.json({
    records: incoming.results,
    newSyncVersion: incoming.results.length
      ? Math.max(...incoming.results.map(r => r.sync_version))
      : body.lastSyncVersion
  });
};
```

This is a research sketch, not production code. It demonstrates:

* D1 `?N` positional parameter binding (max 100 bindings per query [16]).
* `INSERT ... ON CONFLICT` as the server-side LWW write.
* The use of `sync_version` to deliver incremental changes since a client’s last known version.

### 4.4 Attachments (R2)

For binary attachments (images, audio notes) that may exceed D1’s 2 MB row size [16], store the ciphertext in **R2** and store the object key inside the encrypted record:

```ts
// Inside the encrypted record plaintext (before AES-GCM)
interface TodoRecord {
  id: string;
  title: string;
  completed: boolean;
  dueDate?: number;
  attachments?: { r2Key: string; iv: string; tag: string }[];
  modifiedAt: number;
}
```

R2 provides zero-egress-cost object storage and is available on the free tier [19].

---

## 5. Final Recommendation & Getting Started Script

### 5.1 Definitive stack

| Layer | Choice | Rationale |
|-------|--------|-----------|
| Framework | **SvelteKit 5 + TypeScript** | First-party Cloudflare adapter, compiler-based small bundles, fine-grained rune reactivity, Dexie live queries as Svelte stores [1][2][6]. |
| PWA / service worker | **SvelteKit `$service-worker` + `vite-plugin-pwa`** | Keeps the app installable offline; avoid precaching huge payloads [20]. |
| Local DB | **Dexie.js v4** | Minimal wrapper, live queries, TypeScript-first, hooks for custom sync, small bundle [6][7]. |
| Sync conflict | **Last-Write-Wins with server-assigned `sync_version`** | Encrypted server cannot merge content; LWW is the only safe, serverless approach [15]. |
| Crypto | **Web Crypto API (`crypto.subtle`)** | PBKDF2 → AES-256-GCM, `extractable: false`, no npm crypto packages [9][11][13]. |
| Server / API | **Cloudflare Pages + Pages Functions (Workers runtime)** | Handles auth, sync, and device registration; close to the edge. |
| Database | **D1 (SQLite)** | Stores opaque encrypted records and sync metadata; free tier supports 500 MB per DB [16][17]. |
| Session tokens / rate limits | **Workers KV** | Fast edge reads for short-lived session data; 100,000 reads/day on the free tier [19]. |
| Attachments | **R2** | Encrypted files > 2 MB; 10 GB/month free [19]. |

### 5.2 Initial setup script

The following files are a realistic starting point for a Cloudflare Pages project. They are illustrative configuration files, not application source code.

#### `package.json`

```json
{
  "name": "eisen-pwa",
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite dev",
    "build": "vite build",
    "preview": "wrangler pages dev .svelte-kit/cloudflare",
    "deploy": "wrangler pages deploy .svelte-kit/cloudflare",
    "check": "svelte-kit sync && svelte-check --tsconfig ./tsconfig.json"
  },
  "devDependencies": {
    "@sveltejs/adapter-cloudflare": "^4.0.0",
    "@sveltejs/kit": "^2.0.0",
    "@sveltejs/vite-plugin-svelte": "^4.0.0",
    "svelte": "^5.0.0",
    "svelte-check": "^4.0.0",
    "typescript": "^5.5.0",
    "vite": "^5.4.0",
    "vite-plugin-pwa": "^0.20.0",
    "wrangler": "^3.91.0"
  },
  "dependencies": {
    "dexie": "^4.0.0"
  }
}
```

#### `svelte.config.js`

```js
import adapter from '@sveltejs/adapter-cloudflare';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
  preprocess: vitePreprocess(),
  kit: {
    adapter: adapter({
      // Optional: path to a custom wrangler file; default works with wrangler.toml
      config: undefined,
      routes: {
        include: ['/*'],
        exclude: ['<all>']
      }
    })
  }
};

export default config;
```

#### `wrangler.toml`

```toml
# Cloudflare Pages configuration
"$schema" = "./node_modules/wrangler/config-schema.json"
name = "eisen-pwa"
compatibility_date = "2026-08-12"
compatibility_flags = ["nodejs_als"]
pages_build_output_dir = ".svelte-kit/cloudflare"

[[kv_namespaces]]
binding = "KV"
id = "<KV_NAMESPACE_ID>"

[[d1_databases]]
binding = "DB"
database_name = "eisen-db"
database_id = "<D1_DATABASE_ID>"

[[r2_buckets]]
binding = "ATTACHMENTS"
bucket_name = "eisen-attachments"

[vars]
APP_NAME = "Eisen"
RECORD_SCHEMA_VERSION = "1"
```

* `pages_build_output_dir` is the Pages-specific key that tells Wrangler where the SvelteKit Cloudflare adapter emits the build. It is required to treat the Wrangler file as the source of truth for a Pages project [18].
* `compatibility_flags = ["nodejs_als"]` enables `AsyncLocalStorage`, which SvelteKit uses for request context on Cloudflare [1].

#### `vite.config.ts`

```ts
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import { SvelteKitPWA } from 'vite-plugin-pwa';

export default defineConfig({
  plugins: [
    sveltekit(),
    SvelteKitPWA({
      manifest: {
        name: 'Eisen',
        short_name: 'Eisen',
        start_url: '/',
        display: 'standalone',
        background_color: '#ffffff',
        theme_color: '#000000',
        icons: [
          { src: '/icon-192.png', sizes: '192x192', type: 'image/png' },
          { src: '/icon-512.png', sizes: '512x512', type: 'image/png' }
        ]
      },
      workbox: {
        globPatterns: ['**/*.{js,css,html,ico,png,svg,webp,woff2}'],
        maximumFileSizeToCacheInBytes: 2 * 1024 * 1024
      }
    })
  ]
});
```

#### `tsconfig.json`

```json
{
  "extends": "./.svelte-kit/tsconfig.json",
  "compilerOptions": {
    "target": "ESNext",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "strict": true,
    "allowJs": true,
    "checkJs": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "types": ["@sveltejs/kit", "./worker-configuration"]
  }
}
```

#### Bootstrap commands

```bash
# 1. Create the SvelteKit project for Cloudflare Pages
npm create cloudflare@latest -- eisen-pwa --framework=svelte --platform=pages

# 2. Install Dexie for offline-first IndexedDB storage
npm install dexie

# 3. Add the PWA plugin
npm install -D vite-plugin-pwa

# 4. Provision D1 and KV in your Cloudflare account
npx wrangler d1 create eisen-db
npx wrangler kv namespace create "KV"
npx wrangler r2 bucket create eisen-attachments

# 5. Update wrangler.toml with the IDs printed above, then run migrations
npx wrangler d1 migrations apply eisen-db --local

# 6. Develop locally with Pages Functions
npm run preview

# 7. Build and deploy
npm run build
npx wrangler pages deploy .svelte-kit/cloudflare
```

### 5.3 Threat and cost notes

* **Zero-knowledge server:** The server never sees todo titles, completion states, due dates, or attachment contents. It only routes opaque blobs.
* **Free-tier headroom:** A 500 MB D1 database [16] and 10 GB R2 [19] comfortably support tens of thousands of todos plus moderate attachments for a single user or small early-adopter base.
* **CPU limit:** PBKDF2 at 600,000 iterations is intentionally slow. Test on low-end hardware and consider lowering to 100,000–310,000 iterations if UX is poor, but never below OWASP’s 10,000-floor [13]. Run derivation in a Web Worker to avoid blocking the main thread.
* **Subrequest budget:** Each sync invocation performs at most a few D1 queries. The 50-subrequest free limit is generous for a simple sync endpoint [19].

---

## References

1. SvelteKit — `adapter-cloudflare` documentation. https://svelte.dev/docs/kit/adapter-cloudflare
2. Svelte — "What are runes?" (Svelte 5 reactivity). https://svelte.dev/docs/svelte/what-are-runes
3. Cloudflare Workers — Next.js framework guide. https://developers.cloudflare.com/workers/framework-guides/web-apps/nextjs/
4. Cloudflare Workers — React Router (formerly Remix) framework guide. https://developers.cloudflare.com/workers/framework-guides/web-apps/react-router/
5. Cloudflare Pages — build configuration and framework presets. https://developers.cloudflare.com/pages/configuration/build-configuration/
6. Dexie.js — main documentation. https://dexie.org/docs/Dexie.js
7. RxDB — "Best IndexedDB Wrapper" comparison. https://rxdb.info/articles/indexeddb/best-indexeddb-wrapper.html
8. WatermelonDB — Synchronization introduction. https://watermelondb.dev/docs/Sync/Intro
9. MDN — `SubtleCrypto.deriveKey()`. https://developer.mozilla.org/en-US/docs/Web/API/SubtleCrypto/deriveKey
10. MDN — `Pbkdf2Params`. https://developer.mozilla.org/en-US/docs/Web/API/Pbkdf2Params
11. MDN — `AesGcmParams`. https://developer.mozilla.org/en-US/docs/Web/API/AesGcmParams
12. W3C — Web Cryptography API (Level 2). https://www.w3.org/TR/webcrypto/
13. OWASP — Cryptographic Storage Cheat Sheet. https://cheatsheetseries.owasp.org/cheatsheets/Cryptographic_Storage_Cheat_Sheet.html
14. GitHub — `sealedkeys/crypto` (Web Crypto reference implementation). https://github.com/sealedkeys/crypto
15. Replication Conflict Resolution — "Choosing Between LWW, Field-Union, and CRDT". https://www.replication-conflict-resolution.org/conflict-detection-automated-resolution-strategies/algorithm-selection-for-merge/choosing-between-lww-field-union-and-crdt/
16. Cloudflare — D1 limits. https://developers.cloudflare.com/d1/platform/limits/
17. Cloudflare — D1 worker API (JavaScript to D1 type conversion). https://developers.cloudflare.com/d1/worker-api/
18. Cloudflare Pages — Functions Wrangler configuration. https://developers.cloudflare.com/pages/functions/wrangler-configuration/
19. Cloudflare — Workers & Pages pricing and limits (free tier). https://developers.cloudflare.com/workers/platform/pricing/ and https://developers.cloudflare.com/workers/platform/limits/
20. Chrome for Developers — Workbox precaching "dos and don'ts". https://developer.chrome.com/docs/workbox/precaching-dos-and-donts
