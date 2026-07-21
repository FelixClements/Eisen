# ADR-001: Supported native stack, OS versions, lifecycle APIs, and release/signing approach

## Status

Proposed / P0.03

## Context

P0 requires a recorded decision on the supported native stack, minimum OS versions, lifecycle APIs, and release/signing approach for the Windows and Android clients before core implementation begins.

## Decision

### Android client

- **Language / UI:** Kotlin with Jetpack Compose
- **Build tooling:** Gradle with Kotlin DSL, Android Gradle Plugin 9.2.1, Kotlin 2.3.10, KSP
- **Persistence:** Room with KSP-compiled DAOs
- **Background work:** WorkManager
- **Minimum OS:** Android 14 (API 34)
- **Target / compile OS:** targetSdk 36, compileSdk 37
- **JDK:** OpenJDK 21
- **Lifecycle APIs:** Activity/Fragment/Compose lifecycle, `ViewModel`, `rememberSaveable`, `LaunchedEffect`, WorkManager workers, Room transactions
- **Secure storage:** Android Keystore for key material; EncryptedSharedPreferences / Tink for small secrets; encrypted Room or SQLCipher for at-rest task data
- **Release / signing:** Google Play App Signing for releases; AAB distribution; debug keystore for local development

### Windows client

- **Language / UI:** C# with WinUI 3 (Windows App SDK)
- **Build tooling:** .NET SDK + MSBuild
- **Minimum OS:** Windows 10 version 2004 (build 19041) or Windows 11
- **Lifecycle APIs:** Windows App SDK app lifecycle, window events, background tasks, suspension/resume
- **Secure storage:** `Windows.Security.Credentials.PasswordVault` / Credential Locker for keys; Windows Data Protection API (DPAPI) for local data encryption
- **Release / signing:** MSIX packaging; Authenticode code-signing certificate; Microsoft Store or enterprise sideload distribution

### Shared core

- **Language:** Rust
- **Rationale:** Memory safety, deterministic builds, and mature audited crypto crates
- **Crypto / primitives:** `aes-gcm`, `hkdf`, `argon2`, `sha2`, `ed25519-dalek`, `x25519-dalek` (or `ring` where appropriate)
- **Interface:** Rust core exposed through a small C API; consumed by Android via JNI and by Windows via P/Invoke
- **Build tooling:** `cargo` + `rustc` with cross-compilation targets (`aarch64-linux-android`, `armv7-linux-androideabi`, `x86_64-pc-windows-msvc`, `aarch64-pc-windows-msvc`)

## Consequences

- Android and Windows clients share no UI code but share the Rust core for crypto, protocol, and merge logic.
- Core development can proceed in parallel with client UI work once the C API surface is stable.
- Windows builds require a Windows environment or CI runner; macOS cannot produce Windows binaries.
- Rust toolchain is required for local core development; `rustup` is the recommended installer.

## Evidence

### Android spike build

```bash
export JAVA_HOME="/Applications/Android Studio.app/Contents/jbr/Contents/Home"
./gradlew assembleDebug --no-daemon
```

Result: `BUILD SUCCESSFUL in 6s` — 37 actionable tasks: 7 executed, 30 up-to-date.

### Windows spike build (pending Windows toolchain)

```bash
# Requires Windows + .NET SDK + Windows App SDK
msbuild Eisen.Windows.sln /p:Configuration=Release /p:Platform=x64
```

### Core spike build (pending Rust toolchain)

```bash
# Requires rustup + Android NDK + Windows SDK
cargo build --release
cargo test
```

## Lifecycle and secure-storage verification

- Android lifecycle and background behavior will be validated in P2 with instrumented tests and manual lifecycle transitions.
- Windows lifecycle behavior will be validated in P2 on Windows hardware.
- Secure storage APIs chosen above are the baseline; P1/P2 will add functional tests that verify key material never leaves platform secure storage and that encrypted data cannot be read without the key.
