# Repository Boundaries

This document defines the top-level boundaries for the Eisen project. Each boundary has an owning team/role and a stated responsibility. Boundaries may not change downstream contracts without returning to the P0 protocol-freeze process.

| Boundary | Path | Owner | Responsibility |
|---|---|---|---|
| Android client | `clients/android/` | project owner / mobile | Native Android vault create/unlock, task UI, local-only product UX, and platform lifecycle handling. |
| Windows client | `clients/windows/` | project owner / desktop | Native Windows vault create/unlock, task UI, local-only product UX, and platform lifecycle handling. |
| Core library | `core/` | project owner / platform | Reference mutation/merge model, HLC, canonical protocol parsers, device identity, epoch-root/keys, nonce reservation, signed encrypted envelopes, manifest-chain verification, encrypted local store, and mutation transactions. |
| Protocol specifications | `protocol/` | project owner / platform | Canonical encoding, envelope/nonce contracts, HLC/mutation/membership/epoch state machines, enrollment handshake, transport contracts, recovery/compatibility/error contracts, and test vectors. |
| Storage abstractions | `storage/` | project owner / platform | Encrypted local-store schema, wrapping, WAL/journal handling, snapshot primitives, export/import staging, and repair/resync primitives. |
| Servers | `servers/` | project owner / backend | Cloud-sync service, relay service, account/session APIs, and operational tooling. |
| Tests | `tests/` | project owner / qa | Cross-platform vectors, property tests, fault-injection scenarios, acceptance tests, and compatibility matrices. |
| Documentation | `docs/` | project owner / docs | Architecture Decision Records (ADRs), specifications, threat-model review, retention policy, and release-readiness packets. |
| Operations | `ops/` | project owner / ops | Deployment configs, runbooks, dashboards, alerts, backup/disaster-recovery procedures, and CI pipelines. |
| Development tools | `tools/` | project owner / platform | Vector runner, fuzz harnesses, migration helpers, and local-development scripts. |

## Cross-cutting concerns

- Security-sensitive code (crypto, key lifecycle, envelope verification) lives in `core/` and is specified in `protocol/`.
- Client-specific code must not duplicate `core/` or `protocol/` contracts; it consumes the reference model and vectors.
- `servers/` may depend on `protocol/` and `core/` for validation but must never hold decryption keys or plaintext task content.
