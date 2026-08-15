# Auth and First-Time Setup Implementation Plan

This is the concrete, step-by-step implementation plan derived from `auth-first-time-setup-redesign.md`. It is ordered so each phase can be built, tested, and deployed independently. Items earlier in the list fix the current data-loss and 403 bugs; later items add the full onboarding and recovery experience.

**Prerequisites**: the Cloudflare Pages deployment and bindings are already in place (`docs/research/cloudflare-implementation-plan.md` Phases 0–3 are done).

---

## Phase 0: Critical Safety Net (Do This First)

These changes stop the app from corrupting the vault or leaving users locked out while the rest of the work is done.

### 0.1 Prevent `claimPairingCode` from wiping an existing account

**Why**: the current `pairing.ts` clears `accounts` and `deviceState` then stores an account with empty `deviceSalt` and `validationValue`, making unlock impossible.

**Files to edit**:
- `clients/pwa-svelte/src/lib/pairing.ts`

**Change**:

```typescript
export async function claimPairingCode(code: string): Promise<{ ownerId: string; vaultId: string }> {
  if (!browser) throw new Error('Pairing is browser-only.');

  const existingAccount = await getAccount();
  if (existingAccount) {
    throw new Error(
      'This device already has an account. Use “Reset this device” to clear it, or enter the code on a new device.'
    );
  }

  // existing fetch + D1 logic continues
  ...
}
```

**Verification**:
- Playwright test: create an account, try to claim the same code on the same device, expect an error, and expect the original account still unlocks.
- Run `npm run check` and `npm run build`.

### 0.2 Add a `POST /api/devices/enroll` endpoint for the current device

**Why**: the first device should not need a pairing code. It should be enrolled in D1 automatically.

**Files to create/edit**:
- `clients/pwa-svelte/src/routes/api/devices/enroll/+server.ts` (new)
- `clients/pwa-svelte/src/routes/api/sync/+server.ts` (keep 403 logic, it stays the same)

**Endpoint body**:

```typescript
import { json, error } from '@sveltejs/kit';
import type { RequestHandler } from './$types';

export const POST: RequestHandler = async ({ request, platform }) => {
  const d1 = platform?.env?.DB;
  if (!d1) throw error(500, 'D1 binding not configured.');

  const { ownerId, vaultId, deviceId } = (await request.json()) as {
    ownerId: string;
    vaultId: string;
    deviceId: string;
  };

  if (!ownerId || !vaultId || !deviceId) {
    throw error(400, 'Missing device fields.');
  }

  await d1
    .prepare(
      `INSERT INTO accounts (owner_id, vault_id, created_at, last_sync_at, device_count)
       VALUES (?, ?, ?, ?, ?)
       ON CONFLICT(owner_id) DO UPDATE SET
         vault_id = excluded.vault_id,
         last_sync_at = excluded.last_sync_at,
         device_count = excluded.device_count`
    )
    .bind(ownerId, vaultId, Date.now(), null, 1)
    .run();

  await d1
    .prepare(
      `INSERT OR IGNORE INTO devices (device_id, owner_id, enrolled_at, last_seen_at, revoked_at)
       VALUES (?, ?, ?, ?, ?)`
    )
    .bind(deviceId, ownerId, Date.now(), Date.now(), null)
    .run();

  return json({ success: true });
};
```

**Verification**:
- Test the endpoint manually with `curl` and verify the row appears in `devices`.

### 0.3 Auto-enroll the first device on `createAccount`

**Why**: right now sync and cloud backup fail with 403 until the user manually presses “Generate pairing code.”

**Files to edit**:
- `clients/pwa-svelte/src/lib/vault.ts`
- `clients/pwa-svelte/src/lib/db.ts` (ensure `createAccountRecord` returns the new `ownerId`/`vaultId`)

**Change in `vault.ts`**:

```typescript
import { enrollDevice } from './enrollment';

export async function createAccount(password: string): Promise<void> {
  ... // existing key derivation
  const account = await createAccountRecord(encryptedValidation, salt);
  if (!account) throw new Error('Failed to create account.');

  masterKey.set(key);

  // Auto-enroll this first device in the cloud
  await enrollDevice(account).catch(() => {
    // Enrollment failure should not block local usage; user can retry in settings
    console.warn('First-device cloud enrollment failed; falling back to offline-only.');
  });
}
```

**Files to create**:
- `clients/pwa-svelte/src/lib/enrollment.ts`

```typescript
import { browser } from '$app/environment';
import { getAccount } from './db';

export async function enrollDevice(account?: { ownerId: string; vaultId: string }): Promise<void> {
  if (!browser) return;

  const a = account ?? (await getAccount());
  if (!a) throw new Error('No account to enroll.');

  const state = await db.deviceState.toCollection().first();
  if (!state) throw new Error('No device state.');

  const res = await fetch('/api/devices/enroll', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      ownerId: a.ownerId,
      vaultId: a.vaultId,
      deviceId: state.deviceId
    })
  });

  if (!res.ok) {
    throw new Error('Enrollment failed: ' + res.status);
  }
}
```

**Verification**:
- Playwright: create account, add a task, press Sync → should succeed (HTTP 200) instead of 403.
- The `devices` table in D1 should have a new row for the first device.

### 0.4 Add a “Reset this device” button on the unlock screen

**Why**: when the vault is corrupted or the user forgot the passcode, they can start over without needing DevTools.

**Files to edit**:
- `clients/pwa-svelte/src/routes/+page.svelte`
- `clients/pwa-svelte/src/lib/db.ts` (add `clearAllData` helper)

**Change in `db.ts`**:

```typescript
export async function clearAllData(): Promise<void> {
  await db.delete();
  if (typeof window !== 'undefined') {
    localStorage.clear();
  }
}
```

**Change in `+page.svelte`**:
- Add a small “Reset this device” link under the passcode input.
- On click, show a confirmation: “This will delete all local data. The encrypted cloud data is only recoverable if you have an export. Type DELETE to confirm.”
- On confirm, call `await clearAllData()` and reload the page.

**Verification**:
- Playwright: create an account, reset, verify the unlock screen returns to the “Create account” state.

### 0.5 (Optional but recommended) Rename the pairing buttons in Settings

**Why**: “Generate pairing code” and “Claim pairing code” are misleading.

**Files to edit**:
- `clients/pwa-svelte/src/routes/settings/+page.svelte`

**Change**:
- “Generate pairing code” → “Enroll this device” or “Add this device to cloud sync”
- “Claim pairing code” → “Join from another device”
- Add short descriptions under each button.

**Deploy after Phase 0**:
- `npm run check && npm run build && npx wrangler pages deploy .svelte-kit/cloudflare --project-name=eisen-svelte-pwa --branch=main`

---

## Phase 1: Recovery Flow (P1)

### 1.1 Generate a recovery key during account creation

**Why**: without a recovery key, losing IndexedDB or forgetting the passcode means data loss.

**Files to create/edit**:
- `clients/pwa-svelte/src/lib/recovery.ts`
- `clients/pwa-svelte/src/lib/crypto.ts` (extend if needed)

**Approach**:
- Derive a 256-bit recovery key from the master key.
- Encode it as a 12-word BIP-39 mnemonic or a 24-character base64 string.
- Store an encrypted copy in IndexedDB (encrypted with the master key) for in-app use, but force the user to save the plaintext version.

**Verification**:
- Playwright: create account, verify recovery key is shown, and that the user cannot proceed without checking “I saved it.”

### 1.2 Add `/routes/recover/+page.svelte`

**Why**: the unlock screen currently has no recovery path.

**Flow**:
- “Forgot passcode?” link on the unlock screen.
- Recovery tab: enter recovery key → set new passcode → re-derive master key and restore.
- Recovery package import tab: pick a `.json` file, enter its passphrase, import.
- Reset tab: wipe and start over.

**Verification**:
- Playwright: export recovery package, reset device, import recovery package, and unlock with the same data.

### 1.3 Update `recovery.ts` to support passcode change

**Why**: when the user imports a recovery key, the `deviceSalt` and `validationValue` must be regenerated.

**Function to add**:

```typescript
export async function rekeyWithNewPassphrase(
  newPassphrase: string,
  existingMasterKey: CryptoKey
): Promise<{ account: Account; masterKey: CryptoKey }> {
  // generate new salt, derive new key, re-encrypt validation, keep same ownerId/vaultId
}
```

---

## Phase 2: Multi-Step Onboarding (P1)

### 2.1 Create `/routes/onboarding/+page.svelte`

**Why**: the current flow puts create/unlock and settings in one screen and is not discoverable.

**Steps**:
1. Welcome / what is Eisen
2. Offline-only vs cloud sync
3. Create passcode
4. Save recovery key
5. Enroll device (auto)
6. Quick tour (skip-able)

**Files to edit**:
- `clients/pwa-svelte/src/routes/+page.svelte` (redirect to onboarding if no account, otherwise show unlock)
- `clients/pwa-svelte/src/routes/onboarding/+page.svelte` (new)
- `clients/pwa-svelte/src/app.css` (small onboarding styles)

### 2.2 Add step state machine

**Files to create**:
- `clients/pwa-svelte/src/lib/onboarding.ts`

A simple Svelte store that tracks the current step and completion state.

**Verification**:
- Playwright: onboarding reaches the final step, and the app is usable.

### 2.3 Update the main `+page.svelte` to handle setup vs unlock

**Why**: new users should see onboarding; returning users should see unlock.

**Change**:

```svelte
{#if accountExists}
  <UnlockForm />
{:else}
  <Onboarding />
{/if}
```

---

## Phase 3: Safe Multi-Device Pairing (P2)

### 3.1 Add confirmation codes to the pairing flow

**Why**: prevents an attacker who intercepts the pairing code from joining the account.

**Files to edit**:
- `clients/pwa-svelte/src/lib/pairing.ts`
- `clients/pwa-svelte/src/routes/api/pairing/initiate/+server.ts`
- `clients/pwa-svelte/src/routes/api/pairing/claim/+server.ts`

**Approach**:
- When `initiate` is called, generate a confirmation code (e.g., SHA-256 hash of the pairing code and a random nonce) and return it.
- Display the same confirmation code on both devices.
- The claiming device must POST the confirmation back to `/api/pairing/verify` before the new device is fully trusted.

### 3.2 Create a dedicated “Join existing account” flow on new devices

**Files to create**:
- `clients/pwa-svelte/src/routes/pairing/join/+page.svelte`

### 3.3 Add device management page

**Files to create**:
- `clients/pwa-svelte/src/routes/settings/devices/+page.svelte`

**Features**:
- List enrolled devices
- Revoke device
- Rename device

---

## Phase 4: Optional Enhancements (P2)

### 4.1 Biometric / passkey unlock
- Use `navigator.credentials.create` and `navigator.credentials.get` with WebAuthn.
- Store the device-specific key in IndexedDB, wrapped by the biometric credential.
- Always keep the passcode as a fallback.

### 4.2 Automatic cloud backup
- After each task change, queue an encrypted backup package to R2.
- Show last backup date in Settings.

### 4.3 Better error messages
- Replace `Sync failed: 403` with “This device is not enrolled. Go to Settings → Devices to add it.”

---

## Phase 5: Testing and Deployment

### 5.1 Add Playwright coverage for new flows

**Tests to add**:
- Create account, sync succeeds immediately
- Claim pairing code on same device is rejected
- Reset device clears data and returns to setup
- Recovery package export and import round-trip
- Passcode change using recovery key

### 5.2 Run full verification

```bash
cd clients/pwa-svelte
npm run check
npm run build
npx playwright test
```

### 5.3 Deploy

```bash
npx wrangler pages deploy .svelte-kit/cloudflare --project-name=eisen-svelte-pwa --branch=main
```

---

## Suggested Start

Begin with **Phase 0**.1 through **Phase 0.4**. Those stop the data corruption and make sync/backup work immediately, which is the smallest set of changes that resolves the current user-facing issues. Everything else (onboarding UI, recovery key, multi-device pairing) can come after without breaking the basic flow.

---

## Tracking Sheet

| Phase | # | Done | Deployed | Notes |
|-------|---|------|----------|-------|
| 0.1 | Prevent same-device claim wipe | [x] | [x] | Critical |
| 0.2 | `POST /api/devices/enroll` endpoint | [x] | [x] | Critical |
| 0.3 | Auto-enroll on `createAccount` | [x] | [x] | Critical |
| 0.4 | “Reset this device” button | [x] | [x] | Critical |
| 0.5 | Rename pairing buttons | [x] | [x] | Quick win |
| 1.1 | Recovery key generation | [ ] | [ ] | P1 |
| 1.2 | `/recover` page | [ ] | [ ] | P1 |
| 1.3 | Rekey with new passphrase | [ ] | [ ] | P1 |
| 2.1 | Onboarding page | [ ] | [ ] | P1 |
| 2.2 | Onboarding state machine | [ ] | [ ] | P1 |
| 2.3 | Setup vs unlock routing | [ ] | [ ] | P1 |
| 3.1 | Confirmation code pairing | [ ] | [ ] | P2 |
| 3.2 | Join existing account flow | [ ] | [ ] | P2 |
| 3.3 | Device management page | [ ] | [ ] | P2 |
| 4.1 | Biometric unlock | [ ] | [ ] | Optional |
| 4.2 | Automatic cloud backup | [ ] | [ ] | Optional |
| 4.3 | Better error messages | [ ] | [ ] | Optional |
| 5.1 | Playwright tests | [ ] | [ ] | Per phase |
| 5.2 | Verification | [ ] | [ ] | Per phase |
| 5.3 | Deploy | [ ] | [ ] | Per phase |
