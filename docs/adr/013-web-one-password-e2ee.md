# ADR-013: Web client one-password E2EE and record LWW

The SvelteKit web client (`clients/web`) is the active product. It does not implement the frozen Rust/HLC/envelope protocol. Sign-in uses one Account password: the browser sends a domain-separated auth verifier to Better Auth and derives the Vault key locally from the same password plus a public insert-once KDF salt. The server never receives the raw password or the Vault key, so D1 salt plus ciphertext is not enough to decrypt Tasks. A second device only signs in; it GETs the existing salt and pulls blobs. Forgot-password resets sign-in only. Local Tasks are AES-GCM blobs in Dexie (not OPFS). Sync merge is whole-record last-write-wins on `(updatedAt, deviceId)`, not field-level HLC.

This contradicts ADR-001 (Leptos/OPFS), ADR-003 (crypto only in Rust core; Argon2id), ADR-006 (field LWW), ADR-007 (IndexedDB must not hold vault material), and ADR-010 (device Ed25519 `/v1/*` API). Those ADRs remain the native/protocol track. This ADR is the web track.

**Considered options:** a second vault passphrase (rejected: redundant with Account password); storing plaintext Tasks on the server (rejected: breaks the opaque-blob invariant); field-level HLC merge (deferred: one pure function later, no route changes).
