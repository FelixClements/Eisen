# PWA "Keep me signed in" Implementation Spec

## Goal

The installed Eisen PWA opens like a native app: after the user has typed the passphrase once and enabled **"Keep me signed in"**, the next cold open does not ask for the passphrase. Manual lock or sign-out requires the passphrase again.

## Non-goals

- No biometrics, passkeys, or hybrid native wrappers.
- No server-side account or sign-in flow.
- No change to the recovery-passphrase model.
- No timeout auto-lock for v1.

## First unlock

1. User opens the PWA on a new or signed-out device.
2. The unlock screen shows:
   - passphrase input
   - **"Keep me signed in"** checkbox, default unchecked
3. User types the passphrase and checks the box.
4. The app derives the `masterKey` and validates it (existing `unlock()` flow).
5. The app generates a non-extractable `CryptoKey` (`sessionKey`) and wraps the `masterKey` under it.
6. The wrapped `masterKey` and the `sessionKey` are stored in IndexedDB.
7. The app proceeds to the task list.

## Auto unlock

1. On the next cold open, the app looks for a persisted session in IndexedDB.
2. If found and the `validationValue` decrypts correctly with the unwrapped key, the app sets the in-memory `masterKey` and skips the unlock screen.
3. If the session is missing or the wrapped key is corrupt, the app falls back to the normal unlock screen.

## Manual lock and sign-out

- The existing **Lock** button in the header clears the in-memory `masterKey` and shows the unlock screen. It may also clear the persisted session if the user wants a full sign-out; for v1 it will clear the session so the next open requires the passphrase.
- A **"Sign out"** option in Settings clears the persisted session so the next open requires the passphrase.

## Storage

Add a `sessions` object store to the existing Dexie `EisenDB`:

```ts
export interface Session {
	id: 'current';
	kek: CryptoKey; // non-extractable AES-GCM key for wrapping
	wrappedKey: string; // base64-wrapped master key
	createdAt: number;
}
```

`CryptoKey` objects are serializable and can be stored in IndexedDB.

## Crypto flow

- `deriveMasterKey(password, salt)` returns a non-extractable `CryptoKey` as today.
- `generateSessionKey()` returns a non-extractable AES-GCM `CryptoKey` for wrapping.
- `wrapMasterKey(masterKey, sessionKey)` returns the wrapped `masterKey` as a base64 string.
- `unwrapMasterKey(wrappedKey, sessionKey)` returns the `masterKey` `CryptoKey`.

The `SubtleCrypto.wrapKey()` / `unwrapKey()` APIs are used. If the current non-extractable `masterKey` cannot be wrapped directly, the spec will make the `masterKey` extractable for the wrapping step and then re-import the unwrapped copy as non-extractable.

## Files to touch

- `clients/pwa-svelte/src/lib/vault.ts`
  - `persistSession(masterKey)`
  - `loadSession()`
  - `clearSession()`
  - `lock()` calls `clearSession()`
- `clients/pwa-svelte/src/lib/db.ts`
  - add `sessions` table
- `clients/pwa-svelte/src/routes/+page.svelte`
  - add "Keep me signed in" checkbox and pass it to `unlock`
- `clients/pwa-svelte/src/routes/settings/+page.svelte`
  - add "Sign out" action

## UX copy

- Checkbox label: **"Keep me signed in"**
- Settings → Vault → **"Sign out"** button

## Security notes

- The passphrase itself is never stored.
- The unwrapped `masterKey` lives in memory only.
- The `sessionKey` is non-extractable, but both it and the wrapped `masterKey` live in the browser's IndexedDB sandbox.
- Clearing browser data or signing out removes the session.
- The recovery passphrase remains the authoritative key for new devices and recovery.

## Verification

- Create an account, check "Keep me signed in", close the PWA, reopen → no passphrase prompt.
- Tap Lock → passphrase required.
- Tap Sign out → passphrase required.
- `npm run build` in `clients/pwa-svelte` passes.
