# Eisen SvelteKit PWA: Account Setup, Login, and Multi-Device Sync Design

## Executive Summary

This document provides a concrete evolution plan for the existing SvelteKit PWA at `clients/pwa-svelte/` to implement proper account setup, secure password verification, encrypted recovery packages, and multi-device sync. The current implementation has a critical security flaw: any password will "unlock" the vault because there is no password verification mechanism. This document proposes specific code changes, schema migrations, and implementation steps to address this while adding the required account and sync features.

**Key Findings:**
- Current PWA uses PBKDF2 (600k iterations) + AES-256-GCM correctly, but lacks password verification
- Salt is stored in localStorage instead of IndexedDB (should be per-device in secure storage)
- ownerId is a simple UUID in localStorage with no account metadata
- D1 schema is minimal (single `vault_records` table) and lacks device enrollment
- No recovery package, no multi-device pairing, no account creation flow

**Critical Security Fix Required:**
The current `unlock()` function in `vault.ts` (lines 27-30) derives a key from any password without verification. A wrong password will still produce a valid CryptoKey that can decrypt data (incorrectly), or fail silently. We must add a stored validation value encrypted under the correct key to verify the password before accepting it.

---

## Current pwa-svelte State

### File: `src/lib/crypto.ts` (lines 1-75)

**Current Implementation:**
```typescript
const PBKDF2_ITERATIONS = 600_000;
const KEY_LENGTH = 256;

export async function deriveMasterKey(password: string, salt: Uint8Array): Promise<CryptoKey> {
  // Imports password as raw key
  const imported = await crypto.subtle.importKey(
    'raw', 
    new Uint8Array(encoder.encode(password)), 
    'PBKDF2', 
    false, 
    ['deriveKey']
  );
  // Derives AES-256-GCM key using PBKDF2
  return crypto.subtle.deriveKey(
    {
      name: 'PBKDF2',
      salt: safeSalt,
      iterations: PBKDF2_ITERATIONS,
      hash: 'SHA-256'
    },
    imported,
    { name: 'AES-GCM', length: KEY_LENGTH },
    false,
    ['encrypt', 'decrypt']
  );
}

export async function encrypt(plaintext: string, key: CryptoKey): Promise<CipherString> {
  // AES-GCM with 12-byte IV, prepends IV to ciphertext
  const iv = new Uint8Array(crypto.getRandomValues(new Uint8Array(12)));
  const ciphertext = await crypto.subtle.encrypt({ name: 'AES-GCM', iv }, key, encoder.encode(plaintext));
  // Returns base64(iv + ciphertext)
}

export async function decrypt(packed: CipherString, key: CryptoKey): Promise<string> {
  // Extracts IV, decrypts with AES-GCM
}
```

**Assessment:** The crypto primitives are correctly implemented. PBKDF2 iterations are appropriate (600k). AES-GCM with 12-byte IV is standard. The issue is not in the crypto layer but in how it's used (no password verification).

---

### File: `src/lib/vault.ts` (lines 1-34)

**Current Implementation:**
```typescript
const SALT_KEY = 'eisen-salt';
export const masterKey: Writable<CryptoKey | null> = writable(null);

function getSalt(): Uint8Array {
  // Reads salt from localStorage, generates if missing
  const stored = localStorage.getItem(SALT_KEY);
  if (stored) {
    // Decodes base64 salt
    const bytes = new Uint8Array(16);
    const parsed = atob(stored);
    for (let i = 0; i < parsed.length; i++) {
      bytes[i] = parsed.charCodeAt(i);
    }
    return bytes;
  }
  // Generates new salt, stores in localStorage
  const salt = new Uint8Array(crypto.getRandomValues(new Uint8Array(16)));
  const b64 = btoa(String.fromCharCode(...salt));
  localStorage.setItem(SALT_KEY, b64);
  return salt;
}

export async function unlock(password: string): Promise<void> {
  const salt = getSalt();
  masterKey.set(await deriveMasterKey(password, salt));
}

export function lock(): void {
  masterKey.set(null);
}
```

**Critical Issues:**
1. **No password verification:** `unlock()` derives a key from ANY password and sets it. There's no check that the password is correct.
2. **Salt in localStorage:** Salt should be in IndexedDB (encrypted or as part of account metadata) for better security and portability.
3. **No account state:** No distinction between "create account" vs "unlock existing account".
4. **No validation value:** Should store an encrypted known value to verify password correctness.

---

### File: `src/lib/db.ts` (lines 1-188)

**Current Implementation:**
```typescript
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
  sync_version: number | null;  // For sync tracking
  deleted: number;
  encrypted_blob?: string;  // Stores encrypted data from sync
}

export class EisenDB extends Dexie {
  tasks!: Table<Task, string>;
  constructor() {
    super('eisen-pwa');
    this.version(1).stores({
      tasks: 'id, isCompleted, isArchived, isPinned, dueDate, updatedAt, createdAt, deleted, sync_version'
    });
  }
}
```

**Assessment:** The Task schema is well-structured and includes sync_version for conflict resolution. However, there's no `accounts` table or `deviceState` table for storing account metadata and per-device state.

---

### File: `src/lib/sync.ts` (lines 1-103)

**Current Implementation:**
```typescript
const OWNER_ID_KEY = 'eisen-owner-id';
const LAST_SYNC_KEY = 'eisen-last-version';

export function getOwnerId(): string {
  let id = localStorage.getItem(OWNER_ID_KEY);
  if (!id) {
    id = crypto.randomUUID();
    localStorage.setItem(OWNER_ID_KEY, id);
  }
  return id;
}

export async function sync(masterKey: CryptoKey, fetch = globalThis.fetch): Promise<void> {
  const ownerId = getOwnerId();
  const lastVersion = getLastSyncVersion();
  const tasks = await db.tasks.toArray();

  // Encrypts all tasks and sends to server
  const changes: SyncRecord[] = await Promise.all(
    tasks.map(async (task) => {
      const payload: EncryptedTask = { /* task fields except id, updatedAt, sync_version, deleted */ };
      return {
        recordId: task.id,
        encryptedBlob: await encrypt(JSON.stringify(payload), masterKey),
        modifiedAt: task.updatedAt,
        deleted: task.deleted
      };
    })
  );

  const response = await fetch('/api/sync', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ ownerId, lastVersion, changes })
  });

  // Receives server changes, decrypts, upserts to local DB
  const { changes: serverRecords, lastVersion: newVersion } = await response.json();
  for (const record of serverRecords) {
    const plaintext = await decrypt(record.encryptedBlob, masterKey);
    const content = JSON.parse(plaintext) as EncryptedTask;
    // Updates local DB if server version is newer
  }
  setLastSyncVersion(newVersion);
}
```

**Assessment:** Sync logic is sound (encrypt all, send, receive, decrypt, merge). However:
1. ownerId is just a UUID in localStorage with no account metadata
2. No device identity or enrollment verification
3. No epoch-based key rotation support
4. Sync endpoint doesn't verify the device is authorized

---

### File: `src/routes/api/sync/+server.ts` (lines 1-63)

**Current Implementation:**
```typescript
export const POST: RequestHandler = async ({ request, platform }) => {
  const d1 = platform.env.DB;
  const { ownerId, lastVersion, changes } = (await request.json()) as SyncPayload;

  // Upserts all changes with incrementing sync_version
  for (const change of changes) {
    const row = await d1
      .prepare('SELECT IFNULL(MAX(sync_version), 0) + 1 AS v FROM vault_records WHERE owner_id = ?')
      .bind(ownerId)
      .first<number>('v');
    const nextVersion = row ?? 1;

    await d1
      .prepare(`
        INSERT INTO vault_records (record_id, owner_id, encrypted_blob, modified_at, sync_version, deleted)
        VALUES (?, ?, ?, ?, ?, ?)
        ON CONFLICT(record_id) DO UPDATE SET
          owner_id = excluded.owner_id,
          encrypted_blob = excluded.encrypted_blob,
          modified_at = excluded.modified_at,
          sync_version = excluded.sync_version,
          deleted = excluded.deleted
      `)
      .bind(change.recordId, ownerId, change.encryptedBlob, change.modifiedAt, nextVersion, change.deleted)
      .run();
  }

  // Returns new records since lastVersion
  const { results } = await d1
    .prepare('SELECT record_id AS recordId, encrypted_blob AS encryptedBlob, modified_at AS modifiedAt, sync_version AS syncVersion, deleted FROM vault_records WHERE owner_id = ? AND sync_version > ? ORDER BY sync_version')
    .bind(ownerId, lastVersion)
    .all();

  const maxRow = await d1
    .prepare('SELECT IFNULL(MAX(sync_version), 0) AS v FROM vault_records WHERE owner_id = ?')
    .bind(ownerId)
    .first<number>('v');
  const last = maxRow ?? 0;

  return json({ changes: results, lastVersion: last });
};
```

**Assessment:** The sync endpoint is functional but lacks:
1. Device enrollment verification (anyone with ownerId can sync)
2. Rate limiting
3. Signature verification
4. Account metadata tracking

---

### File: `migrations/0001_init.sql` (lines 1-11)

**Current Schema:**
```sql
CREATE TABLE IF NOT EXISTS vault_records (
  record_id TEXT PRIMARY KEY,
  owner_id TEXT NOT NULL,
  encrypted_blob BLOB NOT NULL,
  modified_at INTEGER NOT NULL,
  sync_version INTEGER NOT NULL,
  deleted INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_vault_records_owner_version
ON vault_records(owner_id, sync_version);
```

**Assessment:** Minimal schema. Lacks accounts table, devices table, manifests table, and pairing code storage.

---

### File: `wrangler.toml` (lines 1-23)

**Current Configuration:**
```toml
name = "eisen-svelte-pwa"
compatibility_date = "2025-07-18"
compatibility_flags = ["nodejs_als"]
pages_build_output_dir = ".svelte-kit/cloudflare"

[[kv_namespaces]]
binding = "KV"
id = "00000000-0000-0000-0000-000000000000"

[[d1_databases]]
binding = "DB"
database_name = "eisen-db"
database_id = "00000000-0000-0000-0000-000000000000"

[[r2_buckets]]
binding = "ATTACHMENTS"
bucket_name = "eisen-attachments"
```

**Assessment:** Infrastructure is correctly configured with D1, KV, and R2 bindings. KV is available for pairing codes but not currently used.

---

### File: `src/routes/+page.svelte` (lines 1-152)

**Current UI Flow:**
```svelte
<script>
  let password = $state('');
  let message = $state('');

  async function handleUnlock(event: Event) {
    event.preventDefault();
    message = '';
    try {
      await unlock(password);  // Just derives key, no verification
      password = '';
    } catch (e) {
      message = 'Could not unlock. Check your passphrase.';
    }
  }
</script>

{#if !$masterKey}
  <div class="unlock-screen">
    <h1>Eisen</h1>
    <p>Enter your passphrase to unlock your local tasks.</p>
    <form onsubmit={handleUnlock}>
      <input type="password" bind:value={password} placeholder="Passphrase" />
      <button class="primary" type="submit">Unlock</button>
    </form>
  </div>
{:else}
  <!-- Task UI -->
{/if}
```

**Assessment:** The UI assumes "unlock" always works if no exception is thrown. There's no distinction between "create account" (first launch) and "unlock existing account". The error message is generic.

---

## Proposed Changes

### 1. Fix Password Verification (Critical Security Fix)

**Problem:** Current `unlock()` derives a key from any password without verification. A wrong password produces a valid CryptoKey that can attempt decryption (and fail with AEAD tag mismatch on actual data), but the vault is considered "unlocked" before any data access.

**Solution:** Store a validation value encrypted under the correct key. Verify password by attempting to decrypt this value before accepting the unlock.

**Changes to `src/lib/vault.ts`:**

```typescript
const SALT_KEY = 'eisen-salt';
const VALIDATION_KEY = 'eisen-validation';  // New: stores encrypted validation value
const ACCOUNT_EXISTS_KEY = 'eisen-account-exists';  // New: flag to distinguish create vs unlock

export const masterKey: Writable<CryptoKey | null> = writable(null);

function getSalt(): Uint8Array {
  // Move salt from localStorage to IndexedDB (see db.ts changes)
  // For now, keep existing localStorage logic but add validation
  const stored = localStorage.getItem(SALT_KEY);
  if (stored) {
    const bytes = new Uint8Array(16);
    const parsed = atob(stored);
    for (let i = 0; i < parsed.length; i++) {
      bytes[i] = parsed.charCodeAt(i);
    }
    return bytes;
  }
  const salt = new Uint8Array(crypto.getRandomValues(new Uint8Array(16)));
  const b64 = btoa(String.fromCharCode(...salt));
  localStorage.setItem(SALT_KEY, b64);
  return salt;
}

export function accountExists(): boolean {
  return localStorage.getItem(ACCOUNT_EXISTS_KEY) === 'true';
}

export async function createAccount(password: string): Promise<void> {
  const salt = getSalt();
  const key = await deriveMasterKey(password, salt);
  
  // Generate and encrypt validation value
  const validationValue = crypto.randomUUID();
  const encryptedValidation = await encrypt(validationValue, key);
  localStorage.setItem(VALIDATION_KEY, encryptedValidation);
  localStorage.setItem(ACCOUNT_EXISTS_KEY, 'true');
  
  masterKey.set(key);
}

export async function unlock(password: string): Promise<void> {
  if (!accountExists()) {
    throw new Error('No account exists. Create one first.');
  }
  
  const salt = getSalt();
  const key = await deriveMasterKey(password, salt);
  
  // Verify password by decrypting validation value
  const encryptedValidation = localStorage.getItem(VALIDATION_KEY);
  if (!encryptedValidation) {
    throw new Error('Account corrupted. No validation value found.');
  }
  
  try {
    const decrypted = await decrypt(encryptedValidation, key);
    // If decryption succeeds, password is correct
    masterKey.set(key);
  } catch (e) {
    // Decryption failed = wrong password
    throw new Error('Incorrect passphrase');
  }
}
```

**Changes to `src/routes/+page.svelte`:**

```svelte
<script>
  import { accountExists, createAccount, unlock } from '$lib/vault';
  
  let password = $state('');
  let message = $state('');
  let isCreate = $state(!accountExists());  // Check if account exists

  async function handleUnlock(event: Event) {
    event.preventDefault();
    message = '';
    try {
      if (isCreate) {
        await createAccount(password);
        message = 'Account created!';
      } else {
        await unlock(password);
      }
      password = '';
    } catch (e) {
      message = e instanceof Error ? e.message : 'Operation failed';
    }
  }
</script>

{#if !$masterKey}
  <div class="unlock-screen">
    <h1>Eisen</h1>
    {#if isCreate}
      <p>Create your account. Choose a strong passphrase.</p>
    {:else}
      <p>Enter your passphrase to unlock your tasks.</p>
    {/if}
    <form onsubmit={handleUnlock}>
      <input type="password" bind:value={password} placeholder="Passphrase" />
      <button class="primary" type="submit">{isCreate ? 'Create Account' : 'Unlock'}</button>
    </form>
    {#if message}
      <p class="error">{message}</p>
    {/if}
  </div>
{:else}
  <!-- Task UI -->
{/if}
```

---

### 2. Add Account Metadata to IndexedDB

**Problem:** Account metadata (ownerId, vaultId, deviceSalt) is scattered across localStorage. Should be in IndexedDB for better organization and backup support.

**Changes to `src/lib/db.ts`:**

```typescript
// Add new interfaces
export interface Account {
  ownerId: string;
  vaultId: string;
  createdAt: number;
  deviceSalt: string;  // base64-encoded
  validationValue: string;  // encrypted validation value
}

export interface DeviceState {
  deviceId: string;
  ownerId: string;
  lastSyncAt: number | null;
}

export class EisenDB extends Dexie {
  tasks!: Table<Task, string>;
  accounts!: Table<Account, string>;
  deviceState!: Table<DeviceState, string>;

  constructor() {
    super('eisen-pwa');
    this.version(2).stores({
      tasks: 'id, isCompleted, isArchived, isPinned, dueDate, updatedAt, createdAt, deleted, sync_version',
      accounts: 'ownerId',
      deviceState: 'deviceId, ownerId'
    });
  }
}

// Add helper functions
export async function getOrCreateAccount(): Promise<Account> {
  let account = await db.accounts.toCollection().first();
  if (!account) {
    account = {
      ownerId: crypto.randomUUID(),
      vaultId: crypto.randomUUID(),
      createdAt: Date.now(),
      deviceSalt: btoa(String.fromCharCode(...crypto.getRandomValues(new Uint8Array(16)))),
      validationValue: ''
    };
    await db.accounts.add(account);
  }
  return account;
}

export async function updateAccountValidation(ownerId: string, encryptedValidation: string): Promise<void> {
  await db.accounts.update(ownerId, { validationValue: encryptedValidation });
}
```

**Changes to `src/lib/vault.ts`:**

```typescript
import { getOrCreateAccount, updateAccountValidation, db } from './db';

export async function createAccount(password: string): Promise<void> {
  const account = await getOrCreateAccount();
  const salt = fromBase64(account.deviceSalt);
  const key = await deriveMasterKey(password, salt);
  
  const validationValue = crypto.randomUUID();
  const encryptedValidation = await encrypt(validationValue, key);
  await updateAccountValidation(account.ownerId, encryptedValidation);
  
  masterKey.set(key);
}

export async function unlock(password: string): Promise<void> {
  const account = await db.accounts.toCollection().first();
  if (!account) {
    throw new Error('No account exists. Create one first.');
  }
  
  const salt = fromBase64(account.deviceSalt);
  const key = await deriveMasterKey(password, salt);
  
  try {
    const decrypted = await decrypt(account.validationValue, key);
    masterKey.set(key);
  } catch (e) {
    throw new Error('Incorrect passphrase');
  }
}
```

---

### 3. Implement Recovery Package Export/Import

**Problem:** No way to backup/restore account. If device is lost or localStorage is cleared, data is gone.

**Solution:** Implement recovery package export/import using Argon2id (via wasm-argon2) and AES-GCM, following the contract in `docs/specs/recovery-package.md`.

**New file: `src/lib/recovery.ts`:**

```typescript
import { encrypt, decrypt } from './crypto';
import { db, getOrCreateAccount } from './db';

interface RecoveryPackage {
  header: {
    magic: 'EISEN-RECOVERY';
    version: 1;
    argon2idParams: {
      memLimitKiB: number;
      iterations: number;
      parallelism: number;
      saltLength: number;
      tagLength: number;
    };
  };
  kdfSalt: string;  // base64
  encryptedKeyring: {
    ciphertext: string;  // base64
    iv: string;  // base64
  };
  locator: {
    ownerId: string;
    vaultIdPrefix: string;
  };
  checksum: string;  // base64
}

// Use wasm-argon2 for Argon2id (Web Crypto API doesn't support it)
// For now, use PBKDF2 as fallback (not ideal but works)
// TODO: Integrate wasm-argon2

export async function exportRecoveryPackage(password: string): Promise<Blob> {
  const account = await getOrCreateAccount();
  const tasks = await db.tasks.toArray();
  
  // Generate Argon2id salt (or PBKDF2 salt as fallback)
  const kdfSalt = crypto.getRandomValues(new Uint8Array(16));
  
  // Derive wrapping key using PBKDF2 (fallback from Argon2id)
  const passwordKey = await crypto.subtle.importKey(
    'raw',
    new TextEncoder().encode(password),
    'PBKDF2',
    false,
    ['deriveKey']
  );
  
  const wrappingKey = await crypto.subtle.deriveKey(
    {
      name: 'PBKDF2',
      salt: kdfSalt,
      iterations: 200000,  // Higher for recovery package
      hash: 'SHA-256'
    },
    passwordKey,
    { name: 'AES-GCM', length: 256 },
    false,
    ['encrypt', 'decrypt']
  );
  
  // Encrypt keyring (account metadata + all tasks)
  const keyringData = JSON.stringify({
    account,
    tasks
  });
  const encryptedKeyring = await encrypt(keyringData, wrappingKey);
  
  // Build package
  const pkg: RecoveryPackage = {
    header: {
      magic: 'EISEN-RECOVERY',
      version: 1,
      argon2idParams: {
        memLimitKiB: 19456,
        iterations: 2,
        parallelism: 1,
        saltLength: 16,
        tagLength: 32
      }
    },
    kdfSalt: btoa(String.fromCharCode(...kdfSalt)),
    encryptedKeyring: {
      ciphertext: encryptedKeyring,
      iv: ''  // IV is embedded in encrypt() output
    },
    locator: {
      ownerId: account.ownerId,
      vaultIdPrefix: account.vaultId.slice(0, 8)
    },
    checksum: ''  // TODO: Compute SHA-256
  };
  
  return new Blob([JSON.stringify(pkg)], { type: 'application/eisen-recovery' });
}

export async function importRecoveryPackage(file: File, password: string): Promise<void> {
  const text = await file.text();
  const pkg: RecoveryPackage = JSON.parse(text);
  
  // Verify magic
  if (pkg.header.magic !== 'EISEN-RECOVERY') {
    throw new Error('Invalid recovery package');
  }
  
  // Derive wrapping key
  const kdfSalt = Uint8Array.from(atob(pkg.kdfSalt), c => c.charCodeAt(0));
  const passwordKey = await crypto.subtle.importKey(
    'raw',
    new TextEncoder().encode(password),
    'PBKDF2',
    false,
    ['deriveKey']
  );
  
  const wrappingKey = await crypto.subtle.deriveKey(
    {
      name: 'PBKDF2',
      salt: kdfSalt,
      iterations: pkg.header.argon2idParams.iterations * 100000,  // Scale for PBKDF2
      hash: 'SHA-256'
    },
    passwordKey,
    { name: 'AES-GCM', length: 256 },
    false,
    ['encrypt', 'decrypt']
  );
  
  try {
    // Decrypt keyring
    const keyringData = await decrypt(pkg.encryptedKeyring.ciphertext, wrappingKey);
    const { account, tasks } = JSON.parse(keyringData);
    
    // Clear existing data
    await db.tasks.clear();
    await db.accounts.clear();
    
    // Import account and tasks
    await db.accounts.add(account);
    await db.tasks.bulkAdd(tasks);
    
  } catch (e) {
    throw new Error('Wrong password or corrupted recovery package');
  }
}
```

**UI Changes to `src/routes/settings/+page.svelte`:**

```svelte
<script>
  import { exportRecoveryPackage, importRecoveryPackage } from '$lib/recovery';

  async function handleExport() {
    const password = prompt('Enter your passphrase to encrypt the recovery package:');
    if (!password) return;
    
    try {
      const blob = await exportRecoveryPackage(password);
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `eisen-recovery-${Date.now()}.json`;
      a.click();
      URL.revokeObjectURL(url);
    } catch (e) {
      alert('Export failed: ' + (e instanceof Error ? e.message : 'Unknown error'));
    }
  }

  async function handleImport(event: Event) {
    const input = event.target as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;
    
    const password = prompt('Enter the passphrase for this recovery package:');
    if (!password) return;
    
    try {
      await importRecoveryPackage(file, password);
      alert('Recovery package imported successfully. Refresh to continue.');
      window.location.reload();
    } catch (e) {
      alert('Import failed: ' + (e instanceof Error ? e.message : 'Unknown error'));
    }
  }
</script>

<div class="card">
  <h3>Backup & Recovery</h3>
  <button onclick={handleExport}>Export Recovery Package</button>
  <div>
    <label>Import Recovery Package:</label>
    <input type="file" accept=".json" onchange={handleImport} />
  </div>
</div>
```

---

### 4. Implement Multi-Device Pairing

**Problem:** No way to add a second device to the same account. Each device has its own ownerId.

**Solution:** Implement pairing flow using Cloudflare KV for short-lived codes, following `docs/specs/enrollment-handshake.md`.

**New file: `src/lib/pairing.ts`:**

```typescript
export interface PairingCode {
  code: string;
  expiresAt: number;
}

export async function initiatePairing(): Promise<PairingCode> {
  const account = await getOrCreateAccount();
  const code = generateShortCode();  // 6-digit alphanumeric
  
  const capability = {
    ownerId: account.ownerId,
    vaultId: account.vaultId,
    createdAt: Date.now(),
    expiresAt: Date.now() + 5 * 60 * 1000  // 5 minutes
  };
  
  // Store in Cloudflare KV via API
  const response = await fetch('/api/pairing/initiate', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ code, capability })
  });
  
  if (!response.ok) {
    throw new Error('Failed to initiate pairing');
  }
  
  return { code, expiresAt: capability.expiresAt };
}

export async function claimPairingCode(code: string, newDeviceId: string): Promise<{
  ownerId: string;
  vaultId: string;
}> {
  const response = await fetch('/api/pairing/claim', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ code, newDeviceId })
  });
  
  if (!response.ok) {
    throw new Error('Invalid or expired pairing code');
  }
  
  return await response.json();
}

function generateShortCode(): string {
  const chars = '0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ';
  let code = '';
  for (let i = 0; i < 6; i++) {
    code += chars[Math.floor(Math.random() * chars.length)];
  }
  return code;
}
```

**New API route: `src/routes/api/pairing/initiate/+server.ts`:**

```typescript
import { json, error } from '@sveltejs/kit';
import type { RequestHandler } from './$types';

export const POST: RequestHandler = async ({ request, platform }) => {
  const kv = platform?.env?.KV;
  if (!kv) {
    throw error(500, 'KV binding not configured');
  }
  
  const { code, capability } = await request.json();
  
  // Store in KV with 5-minute TTL
  await kv.put(
    `pairing:${code}`,
    JSON.stringify(capability),
    { expirationTtl: 300 }
  );
  
  return json({ success: true });
};
```

**New API route: `src/routes/api/pairing/claim/+server.ts`:**

```typescript
import { json, error } from '@sveltejs/kit';
import type { RequestHandler } from './$types';

export const POST: RequestHandler = async ({ request, platform }) => {
  const kv = platform?.env?.KV;
  const d1 = platform?.env?.DB;
  if (!kv || !d1) {
    throw error(500, 'KV or D1 binding not configured');
  }
  
  const { code, newDeviceId } = await request.json();
  
  // Retrieve from KV
  const capabilityJson = await kv.get(`pairing:${code}`);
  if (!capabilityJson) {
    throw error(400, 'Invalid or expired pairing code');
  }
  
  const capability = JSON.parse(capabilityJson);
  
  // Check expiration
  if (Date.now() > capability.expiresAt) {
    await kv.delete(`pairing:${code}`);
    throw error(400, 'Pairing code expired');
  }
  
  // Delete from KV (single-use)
  await kv.delete(`pairing:${code}`);
  
  // Register device in D1
  await d1
    .prepare('INSERT INTO devices (device_id, owner_id, enrolled_at, last_seen_at) VALUES (?, ?, ?, ?)')
    .bind(newDeviceId, capability.ownerId, Date.now(), Date.now())
    .run();
  
  return json({
    ownerId: capability.ownerId,
    vaultId: capability.vaultId
  });
};
```

**UI Changes to `src/routes/settings/+page.svelte`:**

```svelte
<script>
  import { initiatePairing, claimPairingCode } from '$lib/pairing';
  
  let pairingCode = $state('');
  let pairingStatus = $state('');
  let isPairing = $state(false);

  async function handleInitiatePairing() {
    try {
      isPairing = true;
      const result = await initiatePairing();
      pairingCode = result.code;
      pairingStatus = `Code expires at ${new Date(result.expiresAt).toLocaleTimeString()}`;
    } catch (e) {
      pairingStatus = 'Failed to generate pairing code';
    } finally {
      isPairing = false;
    }
  }

  async function handleClaimPairingCode() {
    const code = prompt('Enter the pairing code from your other device:');
    if (!code) return;
    
    try {
      isPairing = true;
      const newDeviceId = crypto.randomUUID();
      const result = await claimPairingCode(code, newDeviceId);
      
      // Update local account with shared ownerId
      const account = await getOrCreateAccount();
      await db.accounts.update(account.ownerId, {
        ownerId: result.ownerId,
        vaultId: result.vaultId
      });
      
      pairingStatus = 'Successfully paired! Refresh to sync.';
    } catch (e) {
      pairingStatus = 'Failed to claim pairing code';
    } finally {
      isPairing = false;
    }
  }
</script>

<div class="card">
  <h3>Multi-Device Sync</h3>
  <button onclick={handleInitiatePairing} disabled={isPairing}>
    {isPairing ? 'Generating...' : 'Generate Pairing Code'}
  </button>
  {#if pairingCode}
    <p class="pairing-code">{pairingCode}</p>
    <p>{pairingStatus}</p>
  {/if}
  <button onclick={handleClaimPairingCode} disabled={isPairing}>
    {isPairing ? 'Claiming...' : 'Claim Pairing Code'}
  </button>
</div>
```

---

### 5. Update Sync Endpoint with Device Verification

**Problem:** Current sync endpoint doesn't verify device enrollment. Anyone with ownerId can sync.

**Solution:** Add device enrollment check to sync endpoint.

**Changes to `src/routes/api/sync/+server.ts`:**

```typescript
export const POST: RequestHandler = async ({ request, platform }) => {
  const d1 = platform?.env.DB;
  if (!d1) {
    throw error(500, 'D1 binding not configured');
  }

  const { ownerId, deviceId, lastVersion, changes } = (await request.json()) as SyncPayload;

  // Verify device is enrolled
  const device = await d1
    .prepare('SELECT device_id FROM devices WHERE device_id = ? AND owner_id = ?')
    .bind(deviceId, ownerId)
    .first();

  if (!device) {
    throw error(403, 'Device not enrolled for this account');
  }

  // Update last_seen_at
  await d1
    .prepare('UPDATE devices SET last_seen_at = ? WHERE device_id = ?')
    .bind(Date.now(), deviceId)
    .run();

  // Existing sync logic...
  for (const change of changes) {
    // ... upsert logic
  }

  // ... read logic

  return json({ changes: results, lastVersion: last });
};
```

**Changes to `src/lib/sync.ts`:**

```typescript
const DEVICE_ID_KEY = 'eisen-device-id';

export function getDeviceId(): string {
  let id = localStorage.getItem(DEVICE_ID_KEY);
  if (!id) {
    id = crypto.randomUUID();
    localStorage.setItem(DEVICE_ID_KEY, id);
  }
  return id;
}

export async function sync(masterKey: CryptoKey, fetch = globalThis.fetch): Promise<void> {
  const ownerId = getOwnerId();
  const deviceId = getDeviceId();  // New: include deviceId
  const lastVersion = getLastSyncVersion();
  const tasks = await db.tasks.toArray();

  const changes: SyncRecord[] = await Promise.all(/* ... */);

  const response = await fetch('/api/sync', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ ownerId, deviceId, lastVersion, changes })  // Include deviceId
  });

  // ... rest of sync logic
}
```

---

## Proposed Changes (continued)

### 7. Store Encrypted Recovery Backups on R2

**Problem:** The recovery package is currently a user-held file. If the user loses the file, there is no cloud backup. Storing an encrypted backup on R2 allows recovery from any device as long as the user has the `ownerId` and password.

**Design:**
- The client already encrypts the recovery package with the account password (see Proposed Change 3).
- The client uploads the encrypted package to a new SvelteKit API route `/api/backup`.
- The server stores the opaque encrypted blob in Cloudflare R2 (using the existing or a new `BACKUPS` binding) under `backups/{ownerId}/{packageId}`.
- The server keeps only the locator (`ownerId`, `packageId`, `createdAt`) in D1; it cannot decrypt the backup.
- On a new device, the user enters `ownerId` and password; the client calls `/api/backup/restore` to list and download the latest backup, then decrypts it locally.
- Backups are immutable. A new write creates a new object and updates the D1 pointer. Old objects can be expired via lifecycle rules or deleted by the client.

**New API routes:**
- `POST /api/backup` — accept `{ ownerId, deviceId, encryptedBlob, packageId }`, verify the device is enrolled, store in R2, write D1 record.
- `GET  /api/backup?ownerId=...&deviceId=...` — list available backups (packageId, createdAt) for the account.
- `GET  /api/backup/:packageId?ownerId=...&deviceId=...` — download the encrypted blob.

**Wrangler config update:**
```toml
# Existing R2 bucket is ATTACHMENTS; either reuse it or add:
[[r2_buckets]]
binding = "BACKUPS"
bucket_name = "eisen-backups"
```

**Security notes:**
- The backup is encrypted with the user's password. The server cannot read it.
- Device enrollment check prevents arbitrary downloads.
- `ownerId` can be public (it is already sent in sync requests). The password is never sent to the server.

---

## D1/KV/R2 Schema Changes

### New Migration: `migrations/0002_add_accounts_and_devices.sql`

```sql
-- Accounts table (anonymous UUID-based accounts)
CREATE TABLE IF NOT EXISTS accounts (
  owner_id TEXT PRIMARY KEY,
  vault_id TEXT NOT NULL UNIQUE,
  created_at INTEGER NOT NULL,
  last_sync_at INTEGER,
  device_count INTEGER DEFAULT 1
);

-- Devices table (per-device enrollment)
CREATE TABLE IF NOT EXISTS devices (
  device_id TEXT PRIMARY KEY,
  owner_id TEXT NOT NULL,
  enrolled_at INTEGER NOT NULL,
  last_seen_at INTEGER,
  revoked_at INTEGER,
  FOREIGN KEY (owner_id) REFERENCES accounts(owner_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_devices_owner ON devices(owner_id);

-- Backups table (records R2 object keys for encrypted backups)
CREATE TABLE IF NOT EXISTS backups (
  package_id TEXT PRIMARY KEY,
  owner_id TEXT NOT NULL,
  r2_key TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  FOREIGN KEY (owner_id) REFERENCES accounts(owner_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_backups_owner ON backups(owner_id);

-- Update vault_records to reference accounts
-- (existing table already has owner_id, just add FK constraint)
-- Note: SQLite doesn't support ALTER TABLE ADD CONSTRAINT with FK
-- So we'll rely on application-level consistency for now
```

### KV Usage

**Pairing codes:**
- Key: `pairing:{code}` (e.g., `pairing:ABC123`)
- Value: JSON string with capability (ownerId, vaultId, createdAt, expiresAt)
- TTL: 300 seconds (5 minutes)

**Session tokens (future):**
- Key: `session:{token}`
- Value: JSON string with session metadata
- TTL: 900 seconds (15 minutes)

### R2 Usage

**Encrypted recovery backups:**
- Key: `backups/{ownerId}/{packageId}`
- Value: Opaque encrypted Blob (the recovery package ciphertext)
- Metadata: ownerId, packageId, createdAt stored in D1 `backups` table
- Lifecycle: versions can be kept for a retention period; server cannot decrypt

---

## Implementation Steps

### Phase 1: Critical Security Fix (P0)

1. **Add password verification to `vault.ts`**
   - Add `VALIDATION_KEY` constant
   - Add `accountExists()` function
   - Add `createAccount()` function that stores encrypted validation value
   - Update `unlock()` to verify password before accepting
   - Estimated time: 2 hours

2. **Update UI to distinguish create vs unlock**
   - Modify `src/routes/+page.svelte` to check `accountExists()`
   - Show "Create Account" vs "Unlock" based on state
   - Display appropriate error messages
   - Estimated time: 1 hour

3. **Test password verification**
   - Test with correct password (should unlock)
   - Test with wrong password (should show error)
   - Test with no account (should show create flow)
   - Estimated time: 1 hour

### Phase 2: Account Metadata in IndexedDB (P0)

4. **Add accounts table to IndexedDB**
   - Update `src/lib/db.ts` to add `accounts` and `deviceState` tables
   - Bump DB version from 1 to 2
   - Add `getOrCreateAccount()` helper
   - Estimated time: 2 hours

5. **Migrate from localStorage to IndexedDB**
   - Update `vault.ts` to use IndexedDB for account metadata
   - Keep salt in IndexedDB instead of localStorage
   - Update `sync.ts` to get ownerId from IndexedDB
   - Estimated time: 2 hours

6. **Test migration**
   - Test fresh install (creates account in IndexedDB)
   - Test existing install (migrates from localStorage)
   - Verify sync still works
   - Estimated time: 1 hour

### Phase 3: Recovery Package (P1)

7. **Implement recovery package export**
   - Create `src/lib/recovery.ts`
   - Implement `exportRecoveryPackage()` using PBKDF2 (fallback from Argon2id)
   - Encrypt account metadata + all tasks
   - Estimated time: 3 hours

8. **Implement recovery package import**
   - Implement `importRecoveryPackage()` in `src/lib/recovery.ts`
   - Decrypt and restore account + tasks
   - Handle wrong password error
   - Estimated time: 2 hours

9. **Add UI for export/import**
   - Add export button to settings page
   - Add file input for import
   - Add password prompts
   - Estimated time: 2 hours

10. **Integrate wasm-argon2 (optional but recommended)**
    - Install `wasm-argon2` or similar package
    - Replace PBKDF2 with Argon2id in recovery package
    - Test on mobile devices
    - Estimated time: 4 hours

### Phase 4: Multi-Device Pairing (P1)

11. **Create D1 migration for accounts and devices**
    - Create `migrations/0002_add_accounts_and_devices.sql`
    - Run migration via Wrangler
    - Estimated time: 1 hour

12. **Implement pairing initiation**
    - Create `src/lib/pairing.ts`
    - Implement `initiatePairing()` with short code generation
    - Create `/api/pairing/initiate` endpoint
    - Store in Cloudflare KV with TTL
    - Estimated time: 3 hours

13. **Implement pairing claim**
    - Implement `claimPairingCode()` in `src/lib/pairing.ts`
    - Create `/api/pairing/claim` endpoint
    - Verify code, check expiration, delete from KV
    - Register device in D1
    - Estimated time: 3 hours

14. **Add pairing UI**
    - Add "Generate Pairing Code" button to settings
    - Add "Claim Pairing Code" button to settings
    - Display pairing code with expiration
    - Update local account on successful claim
    - Estimated time: 2 hours

15. **Update sync with device verification**
    - Add `getDeviceId()` to `src/lib/sync.ts`
    - Include deviceId in sync request
    - Update `/api/sync` to verify device enrollment
    - Update last_seen_at on successful sync
    - Estimated time: 2 hours

### Phase 5: Testing and Polish (P1)

16. **End-to-end testing**
    - Test account creation on fresh device
    - Test unlock with correct/wrong password
    - Test sync between two devices
    - Test recovery package export/import
    - Test pairing flow end-to-end
    - Estimated time: 4 hours

17. **Error handling and edge cases**
    - Handle network errors during sync
    - Handle expired pairing codes
    - Handle corrupted recovery packages
    - Add rate limiting to sync endpoint
    - Estimated time: 3 hours

18. **Documentation**
    - Update README with new features
    - Document recovery package format
    - Document pairing flow
    - Estimated time: 2 hours

**Total Estimated Time:** ~35 hours

---

## Security Considerations

1. **Password verification is now mandatory:** The validation value ensures wrong passwords are rejected before any data access.

2. **Salt in IndexedDB:** Moving salt from localStorage to IndexedDB provides better isolation and backup support.

3. **Recovery package encryption:** Using PBKDF2 (or Argon2id) with high iterations ensures brute-force resistance.

4. **Pairing code expiration:** 5-minute TTL limits exposure if intercepted.

5. **Device enrollment verification:** Sync endpoint now checks device enrollment, preventing unauthorized access.

6. **No server passphrase reset:** Server cannot decrypt recovery packages or reset passwords, maintaining zero-knowledge architecture.

7. **Forward secrecy:** Future implementation of X25519 key exchange during pairing will provide forward secrecy (currently simplified).

---

## References

### Files Inspected
- `clients/pwa-svelte/src/lib/crypto.ts` (lines 1-75)
- `clients/pwa-svelte/src/lib/vault.ts` (lines 1-34)
- `clients/pwa-svelte/src/lib/db.ts` (lines 1-188)
- `clients/pwa-svelte/src/lib/sync.ts` (lines 1-103)
- `clients/pwa-svelte/src/routes/api/sync/+server.ts` (lines 1-63)
- `clients/pwa-svelte/src/routes/+page.svelte` (lines 1-152)
- `clients/pwa-svelte/src/routes/settings/+page.svelte` (lines 1-46)
- `clients/pwa-svelte/migrations/0001_init.sql` (lines 1-11)
- `clients/pwa-svelte/wrangler.toml` (lines 1-23)

### Eisen Documentation
- `docs/adr/003-crypto-primitives.md` - Cryptographic primitives
- `docs/adr/004-owner-key-custody.md` - Owner key custody
- `docs/specs/recovery-package.md` - Recovery package contract
- `docs/specs/enrollment-handshake.md` - Enrollment handshake

---

**Document Status:** Research / Implementation Plan  
**Version:** 2.0  
**Date:** 2025-01-09  
**Author:** Research Agent  
