# Local PWA Storage vs. Cloudflare D1 Data Split

This note maps what the Eisen `clients/pwa-svelte` PWA keeps in the browser, what it encrypts before leaving the device, what it sends to the Cloudflare Worker, and where the server ultimately stores it.

The implementation currently in `clients/pwa-svelte` uses **Dexie/IndexedDB** for local state. The broader architecture documents (`docs/architecture-data-flow.md`, `phasing-plan.md`) describe a planned Rust/WASM + OPFS design, but that is not what `pwa-svelte` currently runs.

---

## Summary table

| Data kind | Browser store | Encrypted on device? | Sent to worker? | Cloud location |
|---|---|---|---|---|
| Tasks | IndexedDB `tasks` | Encrypted before transport/sync | Yes (`/api/sync`) | D1 `vault_records` |
| Account | IndexedDB `accounts` | `validationValue` is encrypted; salt is not | `ownerId`, `vaultId` | D1 `accounts` |
| Device state | IndexedDB `deviceState` | No | `deviceId` | D1 `devices` |
| Session key | IndexedDB `sessions` | CryptoKey itself is non-extractable | No | Local only |
| Sync cursor | `localStorage` (`eisen-last-version`) | No | `lastVersion` | Not stored as account-wide cursor; server versions live in `vault_records.sync_version` |
| Recovery package | Exported as user-held `Blob` | Yes, PBKDF2/AES-GCM | Yes (`/api/backup`) | D1 `backups` + R2 `backups/{ownerId}/{packageId}` |
| Pairing code | None (ephemeral in memory) | No code to encrypt | Yes (`/api/pairing/*`) | KV `pairing:{code}` (transient) + D1 `accounts`/`devices` |

---

## 1. Tasks

### Local store

Tasks live in the Dexie/IndexedDB database `eisen-pwa`, table `tasks`.

From `clients/pwa-svelte/src/lib/db.ts`:

```ts
export interface Task {
  id: string;
  title: string;
  description: string;
  isImportant: boolean;
  isUrgent: boolean;
  dueDate: number | null;
  reminderAt: number | null;
  isCompleted: boolean;
  isArchived: boolean;
  isPinned: boolean;
  category: string;
  createdAt: number;
  updatedAt: number;
  sync_version: number | null;
  deleted: number;
  encrypted_blob?: string;
}

this.version(3).stores({
  tasks:
    'id, isCompleted, isArchived, isPinned, dueDate, updatedAt, createdAt, deleted, sync_version',
  ...
});
```

The local `tasks` table keeps the **decrypted** task fields so the UI can sort, filter, and search them. The research blueprint confirms this choice: “locally we still store plaintext fields … because the device is trusted after the user has unlocked the app. The encrypted payload is what leaves the device” (`docs/research/pwa-e2ee-cloudflare-blueprint.md`).

### Encryption before sending

On sync, each task is serialized into a payload and encrypted with the master `CryptoKey` using AES-256-GCM.

From `clients/pwa-svelte/src/lib/sync.ts`:

```ts
const payload: EncryptedTask = {
  title: task.title,
  description: task.description,
  ...
};
return {
  recordId: task.id,
  encryptedBlob: await encrypt(JSON.stringify(payload), masterKey),
  modifiedAt: task.updatedAt,
  deleted: task.deleted
};
```

`encrypt()` is defined in `clients/pwa-svelte/src/lib/crypto.ts`:

```ts
export async function encrypt(plaintext: string, key: CryptoKey): Promise<CipherString> {
  const encoder = new TextEncoder();
  const iv = new Uint8Array(crypto.getRandomValues(new Uint8Array(12)));
  const ciphertext = new Uint8Array(
    await crypto.subtle.encrypt({ name: 'AES-GCM', iv }, key, encoder.encode(plaintext))
  );
  ...
  return toBase64(combined);
}
```

### What reaches the worker and where it lands

The client `POST`s to `/api/sync` with `ownerId`, `deviceId`, `lastVersion`, and the `changes` array.

The Worker in `clients/pwa-svelte/src/routes/api/sync/+server.ts` writes each change to the D1 table `vault_records`:

```sql
INSERT INTO vault_records (record_id, owner_id, encrypted_blob, modified_at, sync_version, deleted)
VALUES (?, ?, ?, ?, ?, ?)
ON CONFLICT(record_id) DO UPDATE SET
  owner_id = excluded.owner_id,
  encrypted_blob = excluded.encrypted_blob,
  modified_at = excluded.modified_at,
  sync_version = excluded.sync_version,
  deleted = excluded.deleted
```

The task content inside `encrypted_blob` is opaque to the server. The server only assigns `sync_version`, stores the ciphertext, and returns newer records ordered by `sync_version`.

The D1 schema for `vault_records` is:

```sql
CREATE TABLE IF NOT EXISTS vault_records (
  record_id TEXT PRIMARY KEY,
  owner_id TEXT NOT NULL,
  encrypted_blob BLOB NOT NULL,
  modified_at INTEGER NOT NULL,
  sync_version INTEGER NOT NULL,
  deleted INTEGER NOT NULL DEFAULT 0
);
```

(`clients/pwa-svelte/migrations/0001_init.sql`)

---

## 2. Account

### Local store

Account data is in the IndexedDB `accounts` table.

From `clients/pwa-svelte/src/lib/db.ts`:

```ts
export interface Account {
  ownerId: string;
  vaultId: string;
  deviceSalt: string;
  validationValue: string;
  createdAt: number;
}

this.version(3).stores({
  accounts: 'ownerId',
  ...
});
```

`validationValue` is an encrypted known string used to verify the passphrase; `deviceSalt` is the per-device PBKDF2 salt.

From `clients/pwa-svelte/src/lib/vault.ts`:

```ts
const VALIDATION_VALUE = 'eisen-validation-value';

export async function createAccount(password: string, keepSignedIn = false): Promise<void> {
  const salt = new Uint8Array(crypto.getRandomValues(new Uint8Array(16)));
  const key = await deriveMasterKey(password, salt);
  const encryptedValidation = await encrypt(VALIDATION_VALUE, key);
  const account = await createAccountRecord(encryptedValidation, salt);
  ...
}
```

`createAccountRecord` stores `deviceSalt` as base64 and `validationValue` as the encrypted string in `accounts` (`clients/pwa-svelte/src/lib/db.ts`):

```ts
const account: Account = {
  ownerId: crypto.randomUUID(),
  vaultId: crypto.randomUUID(),
  deviceSalt: btoa(String.fromCharCode(...deviceSalt)),
  validationValue,
  createdAt: Date.now()
};
```

### Encryption

The **passphrase verification value** is encrypted with the master key. The salt and `ownerId`/`vaultId` are not encrypted at rest; they are operational metadata.

### What reaches the worker and where it lands

The PWA sends `ownerId` and `vaultId` for device enrollment and pairing. The Worker writes or updates the D1 `accounts` table.

From `clients/pwa-svelte/src/routes/api/devices/enroll/+server.ts`:

```sql
INSERT INTO accounts (owner_id, vault_id, created_at, last_sync_at, device_count)
VALUES (?, ?, ?, ?, ?)
ON CONFLICT(owner_id) DO UPDATE SET
  vault_id = excluded.vault_id,
  last_sync_at = excluded.last_sync_at,
  device_count = excluded.device_count
```

The D1 `accounts` schema is:

```sql
CREATE TABLE IF NOT EXISTS accounts (
  owner_id TEXT PRIMARY KEY,
  vault_id TEXT NOT NULL UNIQUE,
  created_at INTEGER NOT NULL,
  last_sync_at INTEGER,
  device_count INTEGER DEFAULT 1
);
```

(`clients/pwa-svelte/migrations/0002_add_accounts_and_devices.sql`)

So the **cloud only holds the account ID, vault ID, and device metadata** — never the passphrase, salt, or master key.

---

## 3. Device state

### Local store

Device state is in the IndexedDB `deviceState` table:

```ts
export interface DeviceState {
  deviceId: string;
  ownerId: string;
  lastSyncAt: number | null;
}

this.version(3).stores({
  deviceState: 'deviceId, ownerId',
  ...
});
```

It is created alongside the account in `createAccountRecord`:

```ts
await db.deviceState.add({
  deviceId: crypto.randomUUID(),
  ownerId: account.ownerId,
  lastSyncAt: null
});
```

### Encryption

Device state is unencrypted in IndexedDB. `deviceId` is a public-ish routing identifier.

### What reaches the worker and where it lands

`deviceId` is sent with every sync, backup, and pairing request. The Worker stores it in the D1 `devices` table:

```sql
INSERT OR IGNORE INTO devices (device_id, owner_id, enrolled_at, last_seen_at, revoked_at)
VALUES (?, ?, ?, ?, ?)
```

(`clients/pwa-svelte/migrations/0002_add_accounts_and_devices.sql`)

The Worker also updates `last_seen_at` on each sync:

```sql
UPDATE devices SET last_seen_at = ? WHERE device_id = ?
```

(`clients/pwa-svelte/src/routes/api/sync/+server.ts`)

---

## 4. Session

### Local store

A non-extractable `CryptoKey` can be persisted in the IndexedDB `sessions` table when the user selects “Keep me signed in”:

```ts
export interface Session {
  id: 'current';
  key: CryptoKey;
  createdAt: number;
}

this.version(3).stores({
  sessions: 'id, createdAt',
  ...
});
```

From `clients/pwa-svelte/src/lib/vault.ts`:

```ts
export async function persistSession(key: CryptoKey): Promise<void> {
  await db.sessions.put({ id: 'current', key, createdAt: Date.now() });
}
```

### Encryption

The `CryptoKey` itself is stored as a Web Crypto `CryptoKey` object. It is created with `extractable: false` (`clients/pwa-svelte/src/lib/crypto.ts`), so the raw key bytes are not exposed to JavaScript. It never leaves the device and is never sent over the network.

---

## 5. Sync state

### Local store

The client keeps a local sync cursor in `localStorage`:

```ts
const LAST_SYNC_KEY = 'eisen-last-version';

export function getLastSyncVersion(): number {
  const raw = localStorage.getItem(LAST_SYNC_KEY);
  return raw ? parseInt(raw, 10) : 0;
}

export function setLastSyncVersion(version: number): void {
  localStorage.setItem(LAST_SYNC_KEY, String(version));
}
```

(`clients/pwa-svelte/src/lib/sync.ts`)

### What reaches the worker

Only the `lastVersion` value is sent in the sync request body so the Worker can return records with a higher `sync_version`. The Worker does not persist a per-device cursor.

The server does maintain `last_seen_at` on the `devices` table and `last_sync_at` on `accounts`, but these are timestamps, not sync cursors.

---

## 6. Recovery packages

### Local generation

`exportRecoveryPackage` exports the entire local account and task state to a user-held file. The package is encrypted with a fresh PBKDF2 salt and the user’s passphrase.

From `clients/pwa-svelte/src/lib/recovery.ts`:

```ts
export interface RecoveryPackage {
  version: number;
  ownerId: string;
  vaultId: string;
  kdfSalt: string;
  ciphertext: string;
  createdAt: number;
}

export async function exportRecoveryPackage(password: string): Promise<Blob> {
  const account = await db.accounts.toCollection().first();
  const tasks = await db.tasks.toArray();
  const payload = JSON.stringify({ account, tasks });

  const kdfSalt = new Uint8Array(crypto.getRandomValues(new Uint8Array(16)));
  const key = await deriveMasterKey(password, kdfSalt);
  const ciphertext = await encrypt(payload, key);

  const pkg: RecoveryPackage = {
    version: 1,
    ownerId: account.ownerId,
    vaultId: account.vaultId,
    kdfSalt: toBase64(kdfSalt),
    ciphertext,
    createdAt: Date.now()
  };

  return new Blob([JSON.stringify(pkg)], { type: 'application/eisen-recovery' });
}
```

The package contains the **account record, all tasks, and vault IDs**, wrapped in AES-256-GCM. This is the “user-held recovery package” described in `docs/specs/recovery-package.md`.

### Cloud backup variant

`backupToCloud` calls `exportRecoveryPackage` and uploads the encrypted package text to the Worker.

From `clients/pwa-svelte/src/lib/backup.ts`:

```ts
const blob = await exportRecoveryPackage(password);
const packageId = crypto.randomUUID();
const packageText = await blob.text();

const res = await fetch('/api/backup', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    ownerId: account.ownerId,
    deviceId: device.deviceId,
    packageId,
    packageText
  })
});
```

### What reaches the worker and where it lands

The Worker receives the already-encrypted `packageText`, stores it in R2, and records the pointer in D1.

From `clients/pwa-svelte/src/routes/api/backup/+server.ts`:

```ts
const r2Key = `backups/${ownerId}/${packageId}`;
await r2.put(r2Key, packageText, {
  httpMetadata: { contentType: 'application/eisen-recovery' }
});

await d1
  .prepare(
    `INSERT INTO backups (package_id, owner_id, r2_key, created_at)
     VALUES (?, ?, ?, ?)
     ON CONFLICT(package_id) DO UPDATE SET
       r2_key = excluded.r2_key,
       created_at = excluded.created_at`
  )
  .bind(packageId, ownerId, r2Key, Date.now())
  .run();
```

So the **recovery package ciphertext is in R2** and the **index (package ID, owner ID, R2 key, timestamp) is in D1 `backups`**:

```sql
CREATE TABLE IF NOT EXISTS backups (
  package_id TEXT PRIMARY KEY,
  owner_id TEXT NOT NULL,
  r2_key TEXT NOT NULL,
  created_at INTEGER NOT NULL
);
```

(`clients/pwa-svelte/migrations/0002_add_accounts_and_devices.sql`)

Listing and downloading are authenticated against the D1 `devices` table, then read from R2.

---

## 7. Pairing data

### Local store

No pairing state is persisted locally. The PWA generates a 6-character code in memory, sends it to the Worker, and immediately forgets it after `initiatePairing` returns.

From `clients/pwa-svelte/src/lib/pairing.ts`:

```ts
const code = generateShortCode();
const expiresAt = Date.now() + 5 * 60 * 1000;

const res = await fetch('/api/pairing/initiate', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    code,
    ownerId: account.ownerId,
    vaultId: account.vaultId,
    deviceId: device.deviceId,
    expiresAt
  })
});
```

### What reaches the worker and where it lands

The Worker stores the pairing code in **KV** with a 300-second TTL and records the account/device in D1.

From `clients/pwa-svelte/src/routes/api/pairing/initiate/+server.ts`:

```ts
await kv.put(
  `pairing:${code}`,
  JSON.stringify({ ownerId, vaultId, expiresAt }),
  { expirationTtl: 300 }
);
```

On `claim`, the Worker reads KV, deletes the code, and adds the new device to D1 `devices`:

```ts
const stored = await kv.get(`pairing:${code}`);
...
await d1
  .prepare(
    `INSERT OR IGNORE INTO accounts (owner_id, vault_id, created_at, device_count)
     VALUES (?, ?, ?, ?)`
  )
  .bind(ownerId, vaultId, Date.now(), 1)
  .run();

await d1
  .prepare(
    `INSERT OR IGNORE INTO devices (device_id, owner_id, enrolled_at, last_seen_at)
     VALUES (?, ?, ?, ?)`
  )
  .bind(deviceId, ownerId, Date.now(), Date.now())
  .run();
```

So pairing is transient in **KV**, with durable side effects in **D1 `accounts` and `devices`**.

---

## 8. What is *not* stored in D1 / what the server never sees

- **Plaintext task content.** The Worker only receives and stores `encrypted_blob` for each task and the ciphertext recovery package. The architecture documents state: “Plaintext task content never leaves the device” and “The server is an opaque blob store” (`docs/architecture-data-flow.md`).
- **The user’s passphrase or derived master key.** These never leave the browser. The master key is marked `extractable: false` and stays in the Web Crypto key store or as a `CryptoKey` in IndexedDB (`clients/pwa-svelte/src/lib/crypto.ts`).
- **The device private key** (in the Svelte implementation, the AES master key is the only device-local key; no Ed25519 device signing key is implemented in this client yet).
- **The sync cursor / last sync version.** This is in `localStorage` only.

---

## 9. OPFS vs. IndexedDB in the current client

The architecture docs and phasing plan describe a future stack with **Origin Private File System (OPFS)** for encrypted local storage in a Rust/WASM core (`docs/architecture-data-flow.md`, `phasing-plan.md`).

The `pwa-svelte` client in this repository does **not** use OPFS. Local data is in a Dexie-wrapped IndexedDB database named `eisen-pwa` with the stores `tasks`, `accounts`, `deviceState`, and `sessions`.

```ts
export class EisenDB extends Dexie {
  tasks!: Table<Task, string>;
  accounts!: Table<Account, string>;
  deviceState!: Table<DeviceState, string>;
  sessions!: Table<Session, 'current'>;

  constructor() {
    super('eisen-pwa');
    ...
  }
}
```

(`clients/pwa-svelte/src/lib/db.ts`)

There are no OPFS imports or references in `clients/pwa-svelte` (`grep` for `OPFS|opfs` returned no matches in that directory). This is an important implementation gap relative to the target architecture.

---

## 10. Cloud services bindings

`clients/pwa-svelte/wrangler.toml` shows the three backend namespaces:

```toml
[[kv_namespaces]]
binding = "KV"
id = "70281fa565714be7b7c26af569556071"

[[d1_databases]]
binding = "DB"
database_name = "eisen-db"
database_id = "7cf88082-4cb8-4ba8-ad03-c26db5f710c4"

[[r2_buckets]]
binding = "ATTACHMENTS"
bucket_name = "eisen-attachments"
```

Usage:
- **D1 (`DB`)** — `accounts`, `devices`, `vault_records`, `backups` metadata/tables.
- **R2 (`ATTACHMENTS`)** — actual encrypted recovery package blobs at `backups/{ownerId}/{packageId}`.
- **KV (`KV`)** — transient pairing codes at `pairing:{code}`.

---

## Citation list

- `clients/pwa-svelte/src/lib/db.ts`
- `clients/pwa-svelte/src/lib/vault.ts`
- `clients/pwa-svelte/src/lib/sync.ts`
- `clients/pwa-svelte/src/lib/backup.ts`
- `clients/pwa-svelte/src/lib/pairing.ts`
- `clients/pwa-svelte/src/lib/recovery.ts`
- `clients/pwa-svelte/src/lib/crypto.ts`
- `clients/pwa-svelte/src/lib/enrollment.ts`
- `clients/pwa-svelte/src/routes/api/sync/+server.ts`
- `clients/pwa-svelte/src/routes/api/backup/+server.ts`
- `clients/pwa-svelte/src/routes/api/backup/[packageId]/+server.ts`
- `clients/pwa-svelte/src/routes/api/devices/enroll/+server.ts`
- `clients/pwa-svelte/src/routes/api/pairing/initiate/+server.ts`
- `clients/pwa-svelte/src/routes/api/pairing/claim/+server.ts`
- `clients/pwa-svelte/migrations/0001_init.sql`
- `clients/pwa-svelte/migrations/0002_add_accounts_and_devices.sql`
- `clients/pwa-svelte/wrangler.toml`
- `docs/architecture-data-flow.md`
- `docs/specs/cloud-api.md`
- `docs/specs/recovery-package.md`
- `docs/adr/004-owner-key-custody.md`
- `docs/adr/007-local-at-rest-coverage.md`
- `docs/adr/010-cloud-api.md`
- `docs/adr/011-recovery-package.md`
- `docs/research/pwa-e2ee-cloudflare-blueprint.md`
