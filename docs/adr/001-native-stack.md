# ADR-001: Supported client stack, browsers, lifecycle APIs, storage, and deployment

## Status

Accepted / Redesigned for PWA — 2026-08-02

## Context

P0 requires a recorded decision on the supported client stack, minimum platform requirements, lifecycle APIs, secure storage, release/signing, and deployment approach.

The project originally planned separate native Windows and Android clients. After prototyping the build toolchain (Trunk + Wrangler), core WASM compilation, OPFS storage, and the Leptos PWA shell, the stack is consolidated into a single local-first Progressive Web App (PWA).

## Decision

### Client

- **Framework:** Leptos 0.8 client-side rendered (CSR) in Rust.
- **Build tooling:** `trunk` compiles `clients/pwa/index.html` and the Rust/WASM app to `clients/pwa/dist/`.
- **Deployment:** Cloudflare Workers + Static Assets, served by `wrangler` from the `dist/` directory.

### Supported browsers

- **Minimum baseline:** Chromium (Chrome/Edge 88+), Firefox 107+, Safari 16+.
- **Required APIs:** `Web Crypto`, Origin Private File System (OPFS), Service Workers, Web App Manifest, `crypto.getRandomValues()`.
- **Installability:** Optional PWA install from the browser. The app is also usable directly from the URL without installing, which is the primary fallback for locked-down work laptops.

### Shared core

- **Language:** Rust compiled to `wasm32-unknown-unknown`.
- **Interface:** The core is linked as a WASM crate and runs in a **Web Worker** so it can use OPFS `FileSystemSyncAccessHandle`.
- **Crypto primitives:** `ed25519-dalek`, `x25519-dalek`, `aes-gcm`, `hkdf`, `argon2`, `sha2` (same as before; now compiled to WASM).

### Secure storage

- **Device key material:** stored in `OpfsSecureStorage`, a single OPFS file encrypted under a passphrase-derived key.
- **Vault data:** encrypted snapshots and WAL/outbox metadata are stored in `SnapshotStore` backed by the same encrypted `OpfsSecureStorage`.
- **HLC counter:** stored in `OpfsClockStorage`, or in `SnapshotStore` when using `EncryptedClockStorage`.
- **No OS secure enclave:** the browser has no equivalent to Android Keystore or Windows Credential Locker, so all at-rest protection is in-app encryption.

### Lifecycle APIs

- Browser page lifecycle (`pagehide`, `pageshow`, `beforeunload`, `visibilitychange`).
- Service worker `install`, `activate`, and `fetch` events.
- Web Worker lifecycle for the core worker.

### Release / signing

- No app-store packaging or OS-level code signing.
- HTTPS delivery via Cloudflare.
- `trunk build --release` produces the deployable artifact.

## Previous decision

- Native Windows (C# / WinUI 3) and Android (Kotlin / Jetpack Compose) clients.
- Platform secure storage (Windows Credential Locker / DPAPI, Android Keystore / Tink).
- See commit history for the original ADR-001 text.

## Consequences

- Single UI codebase for all desktop and mobile platforms.
- The PWA can run on work laptops where users cannot install `.exe` or mobile apps from app stores.
- The core and UI share the Rust language and `cargo` ecosystem.
- Build requires `rustup` with the `wasm32-unknown-unknown` target and `trunk` for the client build.
- Secure-storage verification must prove the encrypted OPFS file cannot be read without the passphrase and that the device key material is cleared on lock/reset.
- Cloud/server stack is recorded in ADR-012.

## Evidence

### Core WASM compile

```bash
cd core
cargo check --target wasm32-unknown-unknown
```

Result: `Finished`.

### PWA shell compile

```bash
cd clients/pwa
cargo check --target wasm32-unknown-unknown
```

Result: `Finished`.

### OPFS storage prototype

Branch: `prototype/opfs-storage`
- `OpfsSecureStorage` and `OpfsClockStorage` implement core traits.
- Uses `FileSystemSyncAccessHandle` in a Web Worker.

### Wrangler dry run

```bash
cd cloudflare-deploy
npx wrangler deploy --dry-run
```

Result:

```
✨ Read 2 files from the assets directory .../dist
Total Upload: 0.34 KiB / gzip: 0.24 KiB
No bindings found.
--dry-run: exiting now.
```
