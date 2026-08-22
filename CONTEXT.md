# Eisen

A local-first Eisenhower-matrix task app. Task content is encrypted in the browser before it is stored or synced; the server is an opaque blob store.

## Language

**Account**:
A Better Auth email-and-password identity. The Account password is the only secret the user types for normal use.
_Avoid_: user (as a domain noun), owner, enrollment

**Account password**:
The password typed at sign-in. The browser derives both the login verifier and the Vault key from it. The raw password never leaves the device.
_Avoid_: vault passphrase, master password, PIN

**Auth verifier**:
A domain-separated stretch of the Account password sent to Better Auth instead of the raw password.
_Avoid_: hashed password (that is what Better Auth stores after it receives the verifier)

**Vault key**:
An AES-GCM key derived in the browser from the Account password and the account's public KDF salt. It encrypts Task blobs and never goes to the server.
_Avoid_: owner key, epoch root, master key, passphrase

**KDF salt**:
A public per-Account random value stored once in D1 so every device derives the same Vault key. It is not a secret and cannot decrypt Tasks by itself.
_Avoid_: device salt, nonce

**Check-blob**:
An opaque ciphertext of a known string, stored next to the KDF salt, used to prove a derived Vault key is correct.
_Avoid_: validation value (implementation nickname)

**Task**:
An Eisenhower item the user creates, completes, archives, or deletes. Plaintext exists only in memory while the Workspace is open.
_Avoid_: issue, ticket, mutation, envelope

**Quadrant**:
One of Do Now, Schedule, Delegate, or Eliminate, from a Task's important and urgent flags.
_Avoid_: quadrant number 0–3, owner key

**Workspace**:
The unlocked client session: the Task catalog, Sync, recovery, and reminders behind one interface.
_Avoid_: vault (as the app session), database, store

**Sync**:
Upload and download of encrypted Task blobs for an Account. Only dirty records are uploaded. Merge is last-write-wins on updated time and device id.
_Avoid_: append log, snapshot, HLC, cursor (as a user-facing word)

**Recovery package**:
An optional encrypted file of Tasks for disaster (lost Account password). Not used to add a second device.
_Avoid_: pairing, enrollment handshake, genesis manifest

**Wake-clock**:
A server-side nudge `{ deviceId, wakeAt, nonce }` with no Task content. The device then reads due reminders locally.
_Avoid_: push payload, FCM data message

The web client follows [ADR-013](docs/adr/013-web-one-password-e2ee.md): one Account password, public insert-once KDF salt, Better Auth verifier ≠ Vault KDF, password reset does not decrypt Tasks, Dexie stores ciphertext, merge is record last-write-wins.
