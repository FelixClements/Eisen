# ADR-012: Cloud and relay server stack

## Status

Proposed / G0 amendment

## Context

The Eisen server boundary (`servers/`) provides the cloud-sync service, volatile relay service, account/session APIs, and operational tooling. The server must never hold decryption keys or plaintext task content. It stores and serves opaque encrypted envelopes and snapshots, validates device signatures, and manages account authentication, quotas, cursors, and retention.

The server stack was not fixed during P0 because the protocol contracts (D-010, D-011, and the transport specs) are independent of implementation language. Before G0 passes, the backend stack needs a recorded decision so that P3/P5 implementation can begin with a frozen direction.

## Decision

- **Server language:** Go.
- **Primary frameworks:** `net/http` or a lightweight router such as Gin or Echo; `net/http` is preferred for the initial implementation to minimize dependencies.
- **WebSocket library:** a maintained third-party library for the volatile relay, since the Go standard library does not ship a WebSocket implementation. Candidates are `github.com/coder/websocket` (formerly `nhooyr.io/websocket`) or `github.com/gorilla/websocket`; the exact choice is fixed in P5 when the relay is implemented.
- **Database:** a transactional SQL store (e.g., PostgreSQL) for account metadata, envelope references, cursors, and snapshot advertisements. Envelope and snapshot blobs are opaque bytes and may be stored in object storage or as large binary columns.
- **Build and deployment:** single static binary produced by `go build`; container image or bare-metal deployment supported through `ops/` configurations.
- **Crypto / protocol validation:** signature and manifest validation may call the shared Rust core through FFI or a sidecar. The server does not perform AEAD decryption and must not hold vault keys.

## Consequences

- Go's standard library and ecosystem provide fast, safe HTTP API development, which matches the server's primarily API-and-blob-store role.
- Build times and CI cycles are faster than Rust for the server boundary, while the security-sensitive crypto remains in the audited Rust core.
- Rust cross-compilation is still required for the shared core and any validation sidecar, but not for the main server binary.
- Deployment remains simple: a single Go binary with TLS termination and a PostgreSQL-compatible store.
- Server engineers work in Go; client/core engineers continue to work in Kotlin, C#, and Rust.

## Evidence

### Go toolchain availability

```bash
go version
```

A local Go installation is required. The minimum supported Go version will be recorded in `servers/go.mod` when P3 begins.

### Spike build

```bash
cd servers
go mod init eisen.dev/servers
go build ./...
```

This will be verified in P3.01 when the first cloud-sync skeleton is implemented.

## Relationship to other ADRs

- D-001 fixes Android, Windows, and the shared Rust core.
- D-003 fixes that all cryptographic operations live in the shared Rust core; the Go server may call that core for validation but does not decrypt content.
- D-010 and the cloud API spec define the contract the Go server must implement.
- D-011 and the relay transport spec define the relay contract the Go server may optionally implement in P5.
