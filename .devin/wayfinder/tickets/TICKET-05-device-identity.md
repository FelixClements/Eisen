---
labels: wayfinder:grilling
status: open
---

## Question

How should the Leptos PWA create, store, and recover a device identity when the browser has no OS secure enclave?

## Context

- Each browser profile is a new device.
- Device keys must be generated with **Web Crypto** (`SubtleCrypto`).
- The private device key must be encrypted under the vault key and stored in OPFS.
- Losing a browser profile (storage cleared) should require a recovery package or re-enrollment.

## Options to evaluate

- Generate `CryptoKeyPair` in Web Crypto, export as `PKCS8`/`SPKI`, encrypt with vault key, store in OPFS.
- Derive deterministic device keys from passphrase (simpler but weaker separation).
- Use WebAuthn for hardware-backed keys where available (optional later).
