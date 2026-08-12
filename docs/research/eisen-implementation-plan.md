# Eisen SvelteKit PWA — Implementation Plan

This plan is derived from `docs/research/eisen-account-sync-design.md`. It is broken into seven phases. Each phase has a goal, the files to touch, the acceptance criteria, and the Playwright tests that should be added.

---

## Phase 1 — Password verification and account creation (P0)

**Goal:** Stop the “any password unlocks” bug and make `unlock()` a real account setup action.

**Work:**
1. `src/lib/vault.ts`
   - Store a per-account validation value (encrypted known UUID) in IndexedDB.
   - Add `accountExists()`.
   - Add `createAccount(password)` that derives the key, creates the account, and stores the validation value.
   - Update `unlock(password)` to decrypt the validation value and fail cleanly on a wrong password.
2. `src/lib/db.ts`
   - Add `Account` and `DeviceState` tables (Dexie version bump).
   - Add `getOrCreateAccount()` that returns/stores `{ ownerId, vaultId, deviceSalt, validationValue }`.
   - Migrate the existing `localStorage` salt/ownerId into IndexedDB on first run.
3. `src/routes/+page.svelte`
   - Distinguish create vs unlock in the UI.
   - Show the right button label and error messages.

**Acceptance:**
- Creating an account with a password works.
- Unlocking with the same password works.
- Unlocking with a different password shows an error and does not set `masterKey`.

**Tests:**
- `vault/unlocks-with-correct-password.spec.ts`
- `vault/rejects-wrong-password.spec.ts`
- `vault/creates-account-first-run.spec.ts`

---

## Phase 2 — Account metadata in IndexedDB (P0)

**Goal:** Move owner identity and salt out of `localStorage` and make it the single source of truth.

**Work:**
1. `src/lib/db.ts`
   - Add `accounts` and `deviceState` tables.
   - Add `getAccount()`, `getOrCreateAccount()`, `getDeviceId()`.
2. `src/lib/vault.ts`
   - Read salt and validation value from IndexedDB, not `localStorage`.
3. `src/lib/sync.ts`
   - Read `ownerId` and `deviceId` from IndexedDB via `db` helpers.
4. `src/routes/settings/+page.svelte`
   - Add a card that shows the non-secret `ownerId` and `deviceId`.

**Acceptance:**
- A freshly built PWA creates one account row in IndexedDB.
- Reloading the page does not change `ownerId`.
- Existing localStorage data is migrated once.

**Tests:**
- `account/persists-owner-id.spec.ts`
- `account/migrates-from-localStorage.spec.ts`

---

## Phase 3 — Encrypted recovery package (P1)

**Goal:** Let users export and import a full backup that is tied to their `ownerId` and password.

**Work:**
1. `src/lib/recovery.ts` (new)
   - `exportRecoveryPackage(password)` — serialize account + tasks, derive a wrapping key (PBKDF2 with 200k iterations), encrypt with AES-GCM, return a `Blob`.
   - `importRecoveryPackage(file, password)` — parse the package, derive the wrapping key, decrypt, validate the `ownerId`, and overwrite local IndexedDB.
2. `src/routes/settings/+page.svelte`
   - Add “Export recovery package” and “Import recovery package” UI.
3. Update tests to cover wrong-password failure and successful round-trip.

**Acceptance:**
- Export produces a file.
- Import with the correct password restores the account and tasks.
- Import with the wrong password shows a clear error and does not corrupt local data.

**Tests:**
- `settings/exports-recovery-package.spec.ts`
- `settings/imports-recovery-package.spec.ts`
- `settings/rejects-wrong-recovery-password.spec.ts`

---

## Phase 4 — Multi-device pairing with KV (P1)

**Goal:** Add a device to an existing account using a short-lived one-time code.

**Work:**
1. `src/lib/pairing.ts` (new)
   - `initiatePairing()` — generate a 6-character code, post capability to the server.
   - `claimPairingCode(code)` — fetch the capability and adopt `ownerId`/`vaultId` locally.
2. `src/routes/api/pairing/initiate/+server.ts` (new)
   - Store `pairing:{code}` in KV with a 5-minute TTL.
3. `src/routes/api/pairing/claim/+server.ts` (new)
   - Verify and delete the KV entry; register the new `deviceId` in D1 `devices`.
4. `src/routes/settings/+page.svelte`
   - Add “Generate pairing code” and “Enter pairing code” sections.

**Acceptance:**
- Generating a code shows it and a countdown.
- A second device that enters the code joins the same account.
- Expired/used codes are rejected.

**Tests:**
- `settings/pairing-code-works.spec.ts`
- `settings/pairing-code-expires.spec.ts`

---

## Phase 5 — Sync with device verification (P1)

**Goal:** The server only accepts sync requests from enrolled devices.

**Work:**
1. `migrations/0002_add_accounts_and_devices.sql`
   - Add `accounts`, `devices`, `backups` tables.
2. `src/routes/api/sync/+server.ts`
   - Accept `deviceId` in the request body.
   - Verify the device exists and is not revoked before processing changes.
   - Update `last_seen_at`.
3. `src/lib/sync.ts`
   - Send `deviceId` with every sync request.

**Acceptance:**
- Sync works for a paired device.
- Sync is rejected for an unknown `deviceId`.
- New devices can pull down records after pairing.

**Tests:**
- `sync/works-for-enrolled-device.spec.ts`
- `sync/rejects-unknown-device.spec.ts`
- `sync/paired-device-gets-tasks.spec.ts`

---

## Phase 6 — Encrypted R2 backup storage (P1)

**Goal:** Store the encrypted recovery package in R2 so users can recover from any browser.

**Work:**
1. `wrangler.toml`
   - Confirm or add an `ATTACHMENTS` / `BACKUPS` R2 binding.
2. `migrations/0002_add_accounts_and_devices.sql`
   - Already adds the `backups` table in this plan.
3. `src/routes/api/backup/+server.ts` (new)
   - `POST` — accept `{ ownerId, deviceId, packageId, encryptedBlob }`, verify device, write to R2 at `backups/{ownerId}/{packageId}`, insert D1 row.
   - `GET` (list) — return available backups for the account (device verified).
   - `GET /:packageId` — stream the R2 object back to the client.
4. `src/lib/backup.ts` (new)
   - `uploadBackup(encryptedBlob)` — choose a package id, call `POST /api/backup`.
   - `listBackups()` and `downloadBackup(packageId)` for restore.
5. `src/lib/recovery.ts`
   - After a local export, optionally call `uploadBackup()` to push to R2.
   - During import, optionally call `listBackups()` + `downloadBackup()` before decrypting.
6. `src/routes/settings/+page.svelte`
   - Add “Back up to cloud” and “Restore from cloud” controls.

**Acceptance:**
- A backup can be uploaded to R2.
- The same account on a different device can list and download it.
- The server cannot decrypt the backup (it only stores opaque bytes).

**Tests:**
- `settings/cloud-backup-round-trip.spec.ts`
- `settings/cloud-backup-wrong-password.spec.ts`
- `backup/rejects-unauthorized-download.spec.ts`

---

## Phase 7 — Lifecycle, error handling, and polish (P2)

**Goal:** Make the new account flow safe and usable in real life.

**Work:**
1. `src/lib/vault.ts` / `+page.svelte`
   - Clear, non-technical error messages.
   - Disable buttons during KDF (PBKDF2 is slow).
2. `src/lib/sync.ts`
   - Handle network failure, retry with exponential back-off.
   - Debounce sync after edits.
3. `src/lib/pairing.ts` / endpoints
   - Rate-limit pairing code attempts.
   - Revoke device support (optional in this phase).
4. Playwright end-to-end suite
   - Full new-device onboarding.
   - Wrong password after logout.
   - Backup restore on a clean browser profile.

**Acceptance:**
- No password string is logged or persisted.
- Wrong password is handled gracefully on unlock, import, and cloud restore.
- All existing 36 tests still pass; new tests cover Phases 1–6.

**Tests:**
- `e2e/onboarding.spec.ts`
- `e2e/logout-and-wrong-password.spec.ts`
- `e2e/cloud-restore-new-device.spec.ts`

---

## Dependency order

```
Phase 1 → Phase 2 → Phase 3 → Phase 6
                    ↓
              Phase 4 → Phase 5 → Phase 7
```

Phases 1 and 2 are prerequisites for everything else.
Phases 3 and 6 can be worked independently after Phase 2.
Phases 4 and 5 need the D1 schema and account metadata.
Phase 7 is the final QA/polish pass.

---

## Cloudflare schema summary

```sql
CREATE TABLE IF NOT EXISTS accounts (
  owner_id TEXT PRIMARY KEY,
  vault_id TEXT NOT NULL UNIQUE,
  created_at INTEGER NOT NULL,
  last_sync_at INTEGER,
  device_count INTEGER DEFAULT 1
);

CREATE TABLE IF NOT EXISTS devices (
  device_id TEXT PRIMARY KEY,
  owner_id TEXT NOT NULL,
  enrolled_at INTEGER NOT NULL,
  last_seen_at INTEGER,
  revoked_at INTEGER,
  FOREIGN KEY (owner_id) REFERENCES accounts(owner_id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_devices_owner ON devices(owner_id);

CREATE TABLE IF NOT EXISTS backups (
  package_id TEXT PRIMARY KEY,
  owner_id TEXT NOT NULL,
  r2_key TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  FOREIGN KEY (owner_id) REFERENCES accounts(owner_id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_backups_owner ON backups(owner_id);
```

KV: `pairing:{code}` (5-minute TTL).
R2: `backups/{ownerId}/{packageId}` for opaque encrypted blobs.

---

## Estimated effort

- Phase 1: ~2–3 days
- Phase 2: ~1–2 days
- Phase 3: ~2–3 days
- Phase 4: ~2–3 days
- Phase 5: ~1–2 days
- Phase 6: ~2–3 days
- Phase 7: ~2–3 days

Total: **2–3 weeks** of focused work, plus review and deployment.
