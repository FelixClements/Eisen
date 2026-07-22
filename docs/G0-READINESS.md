# G0 — Protocol and Product Freeze Readiness

## P0 completion summary

All P0 tasks have been completed and the following artifacts are in `docs/adr/` and `docs/specs/`.

### ADRs (D-001 through D-012)

| ADR | File | Topic |
|---|---|---|
| D-001 | `docs/adr/001-native-stack.md` | Native stack, OS versions, lifecycle APIs, release/signing |
| D-002 | `docs/adr/002-canonical-encoding.md` | Canonical encoding decision |
| D-003 | `docs/adr/003-crypto-primitives.md` | Crypto primitives, capability matrix, Argon2id parameters |
| D-004 | `docs/adr/004-owner-key-custody.md` | Owner key custody and transfer |
| D-005 | `docs/adr/005-epoch-key-rotation.md` | Epoch rotation and cutover |
| D-006 | `docs/adr/006-task-schema.md` | Task schema and merge semantics |
| D-007 | `docs/adr/007-local-at-rest-coverage.md` | Local-at-rest coverage |
| D-008 | `docs/adr/008-fail-closed-counter-policy.md` | Fail-closed counter and rollback/clone policy |
| D-009 | `docs/adr/009-snapshot-contract.md` | Snapshot contract |
| D-010 | `docs/adr/010-cloud-api.md` | Cloud API contract decision |
| D-011 | `docs/adr/011-recovery-package.md` | Recovery package decision |
| D-012 | `docs/adr/012-server-stack.md` | Cloud and relay server stack (Go) |

### Specifications

| Spec | File |
|---|---|
| Canonical encoding | `docs/specs/canonical-encoding.md` |
| Task schema | `docs/specs/task-schema.md` |
| Cloud API | `docs/specs/cloud-api.md` |
| Recovery package | `docs/specs/recovery-package.md` |
| Envelope verification | `docs/specs/envelope-verification.md` |
| Envelope and nonce | `docs/specs/envelope-and-nonce.md` |
| State machines | `docs/specs/state-machines.md` |
| Enrollment handshake | `docs/specs/enrollment-handshake.md` |
| Relay transport | `docs/specs/relay-transport.md` |
| Recovery, compatibility, errors | `docs/specs/recovery-compatibility-errors.md` |
| Retention policy | `docs/specs/retention-policy.md` |
| Account/enrollment auth | `docs/specs/account-enrollment-auth.md` |
| Service metadata | `docs/specs/service-metadata.md` |

### Threat model and release sequence

| Document | File |
|---|---|
| Threat model | `docs/threat-model/threat-model.md` |
| Release sequence (local-only before cloud, relay deferred) | recorded in this document and `phasing-plan.md` |

## G0 gate criteria

| ID | Criterion | Status |
|---|---|---|
| G0.01 | Approve ADRs D-001 through D-012. | Approved by project owner. |
| G0.02 | Review and freeze the canonical schema, task/snapshot rules, protocol contracts, limits, and error taxonomy. | Signed off by project owner. |
| G0.03 | Verify positive and negative vectors run in CI and unresolved critical threat-model findings are closed. | CI configured; vector runner and seed vectors to be added in P1.18. |
| G0.04 | Record the release sequence as local-only before cloud sync and defer relay implementation. | Approved by project owner. |

## Release sequence

1. **Slice 1 — Local-only native product**: P1 encrypted local core + P2 native clients (Windows and Android) without cloud or relay. G3 release.
2. **Slice 2 — Cloud-sync beta**: P3 + G4, optional, after local-only release.
3. **Slice 3 — Cloud hardening**: P4 + G5.
4. **Slice 4 — Volatile relay**: P5 + G6, opt-in, deferred until after cloud is stable.
5. **Slice 5 — Public release**: P6 + G7.

## Next step

Project owner review and approval of the ADRs and spec bundle is required to pass G0 before P1 implementation begins.
