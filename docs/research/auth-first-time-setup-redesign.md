# Research Note: Auth and First-Time Setup Flows for Eisen SvelteKit PWA

## 1. Summary of Current Flow and Exact Failure Modes

### Current Implementation Overview

**Account Creation** (`/Users/maarten/Documents/Github/Eisen/clients/pwa-svelte/src/lib/vault.ts` lines 17-32):
- User enters a passcode on the unlock screen
- System generates a random 16-byte salt
- Derives master key using PBKDF2 with 600,000 iterations (line 10 in `crypto.ts`)
- Encrypts a validation value ("eisen-validation-value") with the derived key
- Stores `ownerId`, `vaultId`, `deviceSalt` (base64-encoded), and `validationValue` in IndexedDB
- Creates a device record with a new `deviceId`

**Unlock Flow** (`vault.ts` lines 34-54):
- Retrieves account from IndexedDB
- Decodes `deviceSalt` from base64
- Derives master key from passcode and salt
- Attempts to decrypt `validationValue`
- If decrypted value matches "eisen-validation-value", unlock succeeds

**Pairing Flow** (`/Users/maarten/Documents/Github/Eisen/clients/pwa-svelte/src/lib/pairing.ts`):

*Initiate* (lines 21-50):
- Requires existing account and device state
- Generates 6-character alphanumeric code
- Calls `/api/pairing/initiate` which:
  - Upserts account in D1 `accounts` table
  - Upserts device in D1 `devices` table
  - Stores pairing data in Cloudflare KV with 5-minute TTL

*Claim* (lines 52-82):
- Generates new `deviceId`
- Calls `/api/pairing/claim` which:
  - Retrieves pairing data from KV
  - Inserts account and device in D1
  - Returns `ownerId` and `vaultId`
- **CRITICAL BUG**: Clears local IndexedDB `accounts` and `deviceState` tables (lines 69-70)
- **CRITICAL BUG**: Creates new account record with empty `deviceSalt` and `validationValue` (lines 71-77)
- User is told to "Unlock to continue" but unlock will always fail

**Sync Requirements** (`/Users/maarten/Documents/Github/Eisen/clients/pwa-svelte/src/routes/api/sync/+server.ts` lines 30-37):
- Device must exist in D1 `devices` table with matching `ownerId` and `deviceId`
- Device must not be revoked (`revoked_at IS NULL`)
- If not enrolled, returns 403 error

### Exact Failure Modes

**Problem 1: Confusing First-Time Setup**
- User creates account locally (IndexedDB only)
- No indication that cloud sync requires device enrollment
- Must manually navigate to Settings → "Generate pairing code"
- No clear guidance that this is required for sync/backup
- Sync silently fails with 403 until enrollment

**Problem 2: Claiming Code on Same Device Corrupts Vault**
- `claimPairingCode()` in `pairing.ts` lines 68-79:
  - Clears existing account metadata
  - Sets `deviceSalt: ''` and `validationValue: ''`
  - User cannot unlock because validation check fails
  - No warning that claiming on same device is destructive
  - No way to recover without pre-exported recovery package

**Problem 3: No Recovery from Lost Vault Data**
- If IndexedDB is cleared (browser data wipe, device failure), user is permanently locked out
- Passcode alone is insufficient - need the original `deviceSalt`
- Recovery package exists (`recovery.ts`) but:
  - Requires proactive export
  - No prompts to export during setup
  - No in-app recovery flow from unlock screen

**Problem 4: No Clear Recovery/Reset Flow**
- Unlock screen (`+page.svelte` lines 91-102) only shows passcode input
- No "Forgot passcode?" or "Recover account" option
- No way to initiate recovery package import from locked state
- Settings page has recovery import but requires navigating while unlocked

**Problem 5: Unsafe Multi-Device Join**
- Pairing code can be claimed on any device
- No confirmation code verification (unlike Vauchi, Prism, Holos patterns)
- No protection against claiming on same device
- No explicit "new device" vs "existing device" flow distinction

## 2. UX and Security Principles for First-Time Setup of Encrypted Local-First Apps

### Industry Best Practices

**Bitwarden** (https://bitwarden.com/help/create-bitwarden-account/):
- Email verification before master password creation
- Master password must be at least 12 characters (post-2023.3.0)
- Clear messaging: "Bitwarden employees and systems have no knowledge of, way to retrieve, or way to reset your master password"
- Master password hint sent to email (optional)
- Uses PBKDF2 or Argon2 with username as salt

**Standard Notes** (https://standardnotes.com/help/79/how-does-standard-notes-encrypt-data-on-my-device):
- Security is "on by default" - no configuration needed
- Passcode keys are ephemeral, never stored to disk
- Automatic daily encrypted backups to local disk
- Offline decryption script for worst-case scenarios
- Encrypted and decrypted backup options

**Proton** (https://proton.me/support/recovery-file):
- Recovery file as encrypted backup keychain
- Device data backup enabled by default on web apps
- Multiple recovery methods: recovery phrase, recovery file, device backup, old password
- Clear separation between password reset and data recovery
- Recovery file stored separately from password

**1Password** (https://support.1password.com/emergency-kit/):
- Emergency Kit PDF provided during account creation
- Contains sign-in address, email, Secret Key, space for password
- Clear storage recommendations: safe deposit box, personal cloud, trusted person
- Recovery codes as additional backup (https://1password.com/blog/introducing-1password-recovery-codes)
- 256-bit recovery key derives second encryption key

**Cryptomator** (https://github.com/cryptomator/docs/blob/develop/docs/desktop/password-and-recovery-key.md):
- Recovery key is human-readable form of decrypted master key
- Independent of current vault password
- Allows password reset without breaking encryption
- Recovery key must be kept as safe as password

### Security Principles

**OWASP Password Storage** (https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html):
- Use Argon2id (19 MiB memory, 2 iterations, 1 parallelism) or PBKDF2 (600,000+ iterations with HMAC-SHA-256)
- Unique random salt for each password (minimum 128 bits per NIST SP 800-132)
- Salt prevents rainbow table attacks and precomputation
- Current Eisen implementation correctly uses 600,000 PBKDF2 iterations and 16-byte (128-bit) salt

**NIST SP 800-132** (https://nvlpubs.nist.gov/nistpubs/Legacy/SP/nistspecialpublication800-132.pdf):
- Salt length at least 128 bits
- Derived key length at least 112 bits
- Iteration count should be as high as performance allows
- Eisen meets these requirements

**Local-First Auth Patterns** (https://jazz.tools/docs/auth/local-first-auth):
- Local-first auth lets users start immediately without signup
- Secret effectively is the account
- Recovery passphrase or passkey backup needed for multi-device
- Upgrade path from local-first to managed account recommended

### UX Principles

**Multi-Device Pairing** (Vauchi, Prism, Holos patterns):
- QR code or short code exchange (5-minute validity)
- Confirmation code displayed on both devices for MITM protection
- Explicit "Link New Device" vs "Join Existing Identity" flows
- Pairing requires both devices online and in same room (for QR)
- Encrypted key transfer using ECDH (X25519) for QR-based pairing

**IndexedDB Secure Storage** (https://stackoverflow.com/questions/43315530/store-sensitive-data-in-indexeddb):
- Data can be viewed and edited via DevTools
- Cannot prevent editing, but can detect tampering via checksums
- Use encrypted blob with checksum property for integrity verification
- `add()` method throws ConstraintError if key exists (prevents accidental overwrite)
- Current Eisen uses `clear()` then `add()` which is destructive

**PWA Onboarding** (https://github.com/wesselgrift/sveltekit-spa):
- Multi-step onboarding flow with feature flags
- Config-driven steps for easy modification
- Progressive disclosure of complex features
- Clear call-to-action at each step

## 3. Proposed Redesign with Step-by-Step User Flows

### Flow 1: Create Account (First-Time Setup)

**Screen 1: Welcome**
```
Header: "Welcome to Eisen"
Body: "Your tasks, encrypted and synced across your devices."
CTA: "Get Started"
```

**Screen 2: Choose Setup Mode**
```
Header: "How would you like to use Eisen?"
Option A: "Just on this device (offline-only)"
  - No cloud sync
  - Data stored locally only
  - Can enable sync later
  
Option B: "Sync across devices (recommended)"
  - End-to-end encrypted cloud sync
  - Access from any device
  - Requires internet connection
```

**Screen 3: Create Passcode (if sync enabled)**
```
Header: "Create your passcode"
Input: Passcode field (min 8 characters)
Input: Confirm passcode
Strength indicator
Warning: "This passcode encrypts all your data. If you lose it, you cannot recover your data without a recovery key."
CTA: "Create Account"
```

**Screen 4: Recovery Key Setup (Critical Step)**
```
Header: "Save your recovery key"
Body: "This recovery key is your only backup if you forget your passcode or lose your device."
Display: 12-word recovery phrase (or 24-character alphanumeric code)
Actions:
  - "Copy to clipboard"
  - "Download as file"
  - "Print as PDF"
Checkbox: "I have saved my recovery key in a safe place" (required)
CTA: "Continue" (disabled until checkbox checked)
```

**Screen 5: Device Enrollment (Auto)**
```
Header: "Setting up cloud sync..."
Spinner: "Enrolling this device..."
Success: "Device enrolled! Your tasks will sync automatically."
CTA: "Start using Eisen"
```

**Screen 6: Quick Tour (Optional)**
```
Header: "Quick tour"
3-4 screens highlighting key features
Skip option available
```

### Flow 2: Unlock (Returning User)

**Screen: Unlock**
```
Header: "Welcome back"
Input: Passcode
Link: "Forgot passcode?" (opens recovery flow)
Biometric option (if available and enabled)
CTA: "Unlock"
```

**If passcode incorrect:**
```
Error: "Incorrect passcode"
Link: "Forgot passcode?"
Link: "Recover from backup"
```

### Flow 3: Auto-Enroll First Device for Cloud Sync

**Implementation:**
- After account creation with sync enabled, automatically call enrollment API
- No separate "Generate pairing code" step needed for first device
- Show progress indicator during enrollment
- If enrollment fails, offer retry or offline-only mode

**API Change:**
```typescript
// New endpoint: POST /api/devices/enroll
// Called automatically after account creation
// Enrolls current device without pairing code
```

### Flow 4: Pair Additional Devices Safely

**On Existing Device:**
```
Settings → Devices → "Add New Device"
Display: QR code and 6-character code
Timer: "Expires in 5:00"
Message: "On your new device, select 'Join existing account' and enter this code"
```

**On New Device:**
```
Screen 1: "Join existing account"
Input: 6-character code (or QR scan button)
CTA: "Continue"
```

**Screen 2: Verification**
```
Header: "Verify pairing"
Both devices show: "Confirmation code: 123-456"
Message: "If the codes match, confirm on both devices"
CTA: "Confirm" (on both devices)
```

**Screen 3: Set Passcode**
```
Header: "Create your passcode for this device"
Input: Passcode
Input: Confirm
CTA: "Complete setup"
```

**Screen 4: Sync Data**
```
Header: "Syncing your data..."
Progress bar
Success: "Setup complete! Your tasks are now synced."
CTA: "Start using Eisen"
```

**Safety Measures:**
- Check if device already has account before allowing claim
- If claiming on same device, show warning: "This will replace your local data. Continue?"
- Use confirmation code to prevent MITM attacks
- Delete pairing code after successful claim or expiration

### Flow 5: Lock/Unlock

**Lock:**
```
Settings → "Lock vault" (or auto-lock after inactivity)
Clear master key from memory
Show unlock screen
```

**Unlock:**
```
Input passcode
Derive master key
Validate against stored validation value
If valid, grant access
If invalid, show error with recovery options
```

**Biometric Unlock (Optional Enhancement):**
```
Settings → Security → "Enable biometric unlock"
On unlock: offer biometric as alternative to passcode
Biometric unlocks device-specific key, which unlocks master key
Passcode still required periodically or after biometric failures
```

### Flow 6: Recovery/Reset

**From Unlock Screen: "Forgot passcode?"**

**Option A: Recovery Key**
```
Screen 1: "Recover with recovery key"
Input: 12-word phrase or recovery code
CTA: "Verify"
```

**Screen 2: Set New Passcode**
```
Header: "Set a new passcode"
Input: New passcode
Input: Confirm
CTA: "Update"
```

**Success:**
```
Header: "Account recovered"
Message: "Your data is now accessible with your new passcode."
CTA: "Continue"
```

**Option B: Recovery Package Import**
```
Screen 1: "Import recovery package"
File picker: Select .json recovery file
Input: Passphrase for recovery package
CTA: "Import"
```

**Screen 2: Confirm Import**
```
Header: "Warning: This will replace all local data"
Message: "Importing will overwrite your current tasks. This cannot be undone."
Checkbox: "I understand this will replace my data"
CTA: "Import" (disabled until checked)
```

**Option C: Reset Account (Nuclear Option)**
```
Screen 1: "Reset account"
Warning: "This will delete all your data and create a new account. This cannot be undone."
Input: Type "DELETE" to confirm
CTA: "Reset account"
```

**After Reset:**
```
Return to initial setup flow
```

## 4. Suggested Improvements for Each Problem Area

### Problem 1: Confusing First-Time Setup

**Solutions:**
1. **Multi-step onboarding flow** with clear progression
2. **Explicit sync choice** during setup (offline-only vs cloud sync)
3. **Auto-enroll first device** when sync is enabled
4. **Progress indicators** showing enrollment status
5. **Clear messaging** about what each step does

**Implementation:**
- Create `/routes/onboarding/+page.svelte` with step wizard
- Add setup mode selection (local-only vs cloud)
- Call enrollment API automatically after account creation
- Show sync status in settings with clear "Not enrolled" state

### Problem 2: Claiming Code on Same Device Corrupts Vault

**Solutions:**
1. **Prevent same-device claiming** by checking if account exists locally
2. **Clear warning** if user attempts to claim on device with data
3. **Require explicit confirmation** before overwriting local data
4. **Separate "Reset" flow** from "Join existing account" flow

**Implementation Changes:**
```typescript
// In pairing.ts claimPairingCode():
const existingAccount = await getAccount();
if (existingAccount) {
  const hasData = await db.tasks.count();
  if (hasData > 0) {
    throw new Error(
      'This device already has data. Use "Reset account" in settings to clear it first.'
    );
  }
}
```

### Problem 3: No Recovery from Lost Vault Data

**Solutions:**
1. **Mandatory recovery key creation** during initial setup
2. **Multiple export options** (copy, download, print)
4. **Recovery key import** from unlock screen
5. **Periodic reminders** to verify recovery key is stored safely

**Implementation:**
- Generate recovery key during account creation
- Require user to confirm they've saved it before proceeding
- Store recovery key encrypted with passcode (optional, for convenience)
- Add "Forgot passcode?" link on unlock screen
- Create `/routes/recover/+page.svelte` for recovery flows

### Problem 4: No Clear Recovery/Reset Flow

**Solutions:**
1. **"Forgot passcode?" link** on unlock screen
2. **Dedicated recovery page** with multiple options
3. **Clear reset flow** with warnings and confirmations
4. **Recovery package import** accessible from locked state

**Implementation:**
- Add recovery link to unlock screen in `+page.svelte`
- Create recovery route with tabs: Recovery Key, Recovery Package, Reset
- Add destructive action confirmation pattern
- Allow recovery package import without unlocking first

### Problem 5: Unsafe Multi-Device Join

**Solutions:**
1. **Confirmation code verification** (both devices show matching code)
2. **Explicit "New Device" vs "Existing Device" flow**
3. **QR code option** for more secure pairing (ECDH key exchange)
4. **Device list in settings** to manage and revoke devices
5. **Pairing code single-use** with immediate deletion after claim

**Implementation:**
- Add confirmation code to pairing flow (hash of shared secret)
- Create separate `/routes/pairing/join/+page.svelte` for new devices
- Add QR code generation using X25519 key pair
- Create `/routes/settings/devices/+page.svelte` for device management
- Ensure KV deletes pairing code immediately after successful claim

## 5. Risks and Trade-offs

### Security Risks

**Recovery Key Exposure:**
- **Risk**: If recovery key is stored alongside passcode, attacker gains full access
- **Mitigation**: Encourage storing recovery key separately (physical print, separate cloud storage)
- **Trade-off**: Convenience vs security - encrypted local storage of recovery key is convenient but less secure

**Pairing Code Interception:**
- **Risk**: Network attacker could intercept pairing code and claim it first
- **Mitigation**: Confirmation code verification, short TTL (5 minutes), single-use codes
- **Trade-off**: Additional verification step vs simpler UX

**Biometric Unlock:**
- **Risk**: Biometric data can be compelled or spoofed in some scenarios
- **Mitigation**: Require passcode periodically, use biometric only for convenience
- **Trade-off**: Better UX vs slightly reduced security

### UX Trade-offs

**Mandatory Recovery Key:**
- **Trade-off**: Adds friction to setup but prevents permanent lockout
- **Mitigation**: Make the process quick and clear, offer skip with strong warnings

**Multi-Step Onboarding:**
- **Trade-off**: Longer setup time but clearer understanding of features
- **Mitigation**: Allow skipping tour, save progress, make steps skippable where safe

**Confirmation Codes:**
- **Trade-off**: Extra step in pairing but prevents MITM attacks
- **Mitigation**: Auto-verify when possible, clear instructions

### Implementation Trade-offs

**IndexedDB Integrity:**
- **Current**: No protection against tampering
- **Proposed**: Add checksums to detect tampering
- **Trade-off**: Slightly more complex code, larger storage overhead

**Auto-Enrollment:**
- **Current**: Manual enrollment required
- **Proposed**: Automatic enrollment on first device
- **Trade-off**: More complex setup logic but better UX

**Recovery Package Format:**
- **Current**: JSON with encrypted blob
- **Proposed**: Consider multiple formats (JSON, PDF, plain text with separate key)
- **Trade-off**: More formats to maintain but better compatibility

## 6. Prioritized Implementation Plan

### Phase 1: Critical Security Fixes (P0)

**Timeline: 1-2 weeks**

1. **Fix same-device claim corruption** (`pairing.ts`)
   - Add check for existing account with data
   - Throw error if claiming would destroy data
   - Add warning in UI

2. **Add recovery key generation** (`recovery.ts`)
   - Generate 12-word recovery phrase during account creation
   - Implement BIP-39 or similar mnemonic encoding
   - Test recovery key import flow

3. **Add recovery flow from unlock screen** (`+page.svelte`)
   - Add "Forgot passcode?" link
   - Create `/routes/recover/+page.svelte`
   - Implement recovery key verification
   - Implement recovery package import

### Phase 2: Improved Onboarding (P1)

**Timeline: 2-3 weeks**

4. **Create multi-step onboarding flow**
   - Create `/routes/onboarding/+page.svelte`
   - Implement step wizard component
   - Add welcome screen
   - Add setup mode selection (local-only vs cloud)

5. **Implement auto-enrollment for first device**
   - Create `POST /api/devices/enroll` endpoint
   - Call enrollment after account creation when sync enabled
   - Add progress indicators
   - Handle enrollment failures gracefully

6. **Add recovery key setup step**
   - Display recovery phrase during onboarding
   - Add copy/download/print options
   - Require confirmation before proceeding
   - Store recovery key (encrypted) for convenience

### Phase 3: Safe Multi-Device Pairing (P1)

**Timeline: 2-3 weeks**

7. **Add confirmation code verification**
   - Generate confirmation code from shared secret
   - Display on both devices during pairing
   - Require user to confirm codes match
   - Implement in `pairing.ts` and pairing API

8. **Create dedicated pairing flow for new devices**
   - Create `/routes/pairing/join/+page.svelte`
   - Separate from settings page
   - Add QR code option (X25519 key exchange)
   - Improve error messages

9. **Add device management UI**
   - Create `/routes/settings/devices/+page.svelte`
   - List all enrolled devices
   - Show last seen timestamps
   - Add revoke device functionality
   - Add device renaming

### Phase 4: Enhanced Recovery Options (P2)

**Timeline: 1-2 weeks**

10. **Implement reset account flow**
    - Add reset option in settings
    - Require type "DELETE" confirmation
    - Clear all IndexedDB data
    - Return to onboarding

11. **Add biometric unlock (optional)**
    - Research WebAuthn integration
    - Implement passkey option
    - Add biometric unlock settings
    - Test across platforms

12. **Improve recovery package UX**
    - Add scheduled backup reminders
    - Show last backup date in settings
    - Add automatic cloud backup option
    - Implement backup versioning

### Phase 5: Polish and Testing (P2)

**Timeline: 1-2 weeks**

13. **Add comprehensive error handling**
    - Clear error messages for all failure modes
    - Retry logic for network failures
    - Offline mode indicators
    - Sync status visualization

14. **Security audit**
    - Review crypto implementation
    - Test against OWASP guidelines
    - Verify salt and iteration counts
    - Check for timing attacks

15. **User testing**
    - Test onboarding flow with new users
    - Test recovery flows
    - Test multi-device pairing
    - Gather feedback and iterate

### Dependencies and Prerequisites

**Required:**
- Cloudflare Workers/D1 access for API changes
- KV store for pairing codes (already in place)
- Decision on recovery key format (BIP-39 vs custom)

**Optional but Recommended:**
- QR code library for secure pairing
- BIP-39 library for mnemonic phrases
- WebAuthn library for biometric/passkey support

**Testing Requirements:**
- Test across Chrome, Firefox, Safari
- Test on mobile and desktop
- Test offline scenarios
- Test recovery scenarios

---

## Sources

### Code References
- `/Users/maarten/Documents/Github/Eisen/clients/pwa-svelte/src/lib/vault.ts` - Account creation and unlock logic
- `/Users/maarten/Documents/Github/Eisen/clients/pwa-svelte/src/lib/pairing.ts` - Pairing flow with critical bug
- `/Users/maarten/Documents/Github/Eisen/clients/pwa-svelte/src/lib/db.ts` - IndexedDB schema
- `/Users/maarten/Documents/Github/Eisen/clients/pwa-svelte/src/lib/crypto.ts` - PBKDF2 implementation (600,000 iterations)
- `/Users/maarten/Documents/Github/Eisen/clients/pwa-svelte/src/lib/recovery.ts` - Recovery package export/import
- `/Users/maarten/Documents/Github/Eisen/clients/pwa-svelte/src/routes/+page.svelte` - Unlock screen
- `/Users/maarten/Documents/Github/Eisen/clients/pwa-svelte/src/routes/settings/+page.svelte` - Settings with pairing
- `/Users/maarten/Documents/Github/Eisen/clients/pwa-svelte/src/routes/api/pairing/initiate/+server.ts` - Pairing initiate API
- `/Users/maarten/Documents/Github/Eisen/clients/pwa-svelte/src/routes/api/pairing/claim/+server.ts` - Pairing claim API
- `/Users/maarten/Documents/Github/Eisen/clients/pwa-svelte/src/routes/api/sync/+server.ts` - Sync with device enrollment check
- `/Users/maarten/Documents/Github/Eisen/clients/pwa-svelte/migrations/0002_add_accounts_and_devices.sql` - D1 schema

### External References
- Bitwarden account creation: https://bitwarden.com/help/create-bitwarden-account/
- Bitwarden master password: https://bitwarden.com/help/master-password/
- Bitwarden KDF algorithms: https://bitwarden.com/help/kdf-algorithms/
- Standard Notes encryption: https://standardnotes.com/help/79/how-does-standard-notes-encrypt-data-on-my-device
- Standard Notes backups: https://standardnotes.com/help/14
- Proton recovery file: https://proton.me/support/recovery-file
- Proton account recovery: https://proton.me/support/set-account-recovery-methods
- 1Password Emergency Kit: https://support.1password.com/emergency-kit/
- 1Password recovery codes: https://1password.com/blog/introducing-1password-recovery-codes
- Cryptomator recovery key: https://github.com/cryptomator/docs/blob/develop/docs/desktop/password-and-recovery-key.md
- OWASP Password Storage: https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html
- NIST SP 800-132: https://nvlpubs.nist.gov/nistpubs/Legacy/SP/nistspecialpublication800-132.pdf
- Local-first auth: https://jazz.tools/docs/auth/local-first-auth
- Vauchi multi-device: https://docs.vauchi.app/users/features/multi-device.html
- Prism sync: https://prismplural.com/docs/sync/
- Holos multi-device: https://holos.social/multi-device
- IndexedDB security: https://stackoverflow.com/questions/43315530/store-sensitive-data-in-indexeddb
- SvelteKit onboarding: https://github.com/wesselgrift/sveltekit-svelte