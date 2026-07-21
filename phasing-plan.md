# Eisen coding-agent phasing plan

This is the executable delivery backlog derived from `plan-To-Do-App.md`. Complete phases and their gates in the stated order; a later phase may prepare no implementation that changes an earlier frozen contract.

## Approval authority

All gate approvals, reviews, and sign-offs in this plan are performed by the project owner. A gate is not considered passed until the project owner confirms the stated deliverable and evidence.

## Execution rules

- Protocol, cryptographic, key-lifecycle, merge, or storage-contract changes require a versioned specification update, positive and negative vectors, compatibility analysis, and migration/rollback notes in the same change.
- Never log, telemetry-report, crash-report, commit, or place in test fixtures keys, passphrases, decrypted task data, recovery data, enrollment capabilities, or full encrypted payloads.
- Validate size, structure, version, limits, and authorization before expensive parsing, allocation, decryption, or database writes; fail closed where the protocol requires it.
- Make durable local commits before reporting a local mutation complete, acknowledging received data, or marking an outbox item delivered. Keep cloud and relay cursors/inventories separate.
- Each task closes with its stated artifact and relevant automated evidence; do not substitute a manual claim for a gate criterion.

## Required delivery order

1. Decisions and protocol freeze
2. Encrypted local core
3. Local-only native product
4. Cloud-sync beta
5. Cloud hardening
6. Volatile relay
7. Public-release evidence

---

## - [ ] P0 — Decisions and protocol freeze

**Prerequisites:** None. This phase starts with repository bootstrap and ends before persistence, client product, or service implementation.

| ID | Task | Owner | Deliverable |
|---|---|---|---|
| P0.01 | [x] Create the agreed repository boundaries for clients, core, protocol, storage, servers, tests, docs, ops, and tools. | project owner | buildable empty module skeleton with ownership locations. |
| P0.02 | [x] Configure local and CI checks for formatting, static analysis, dependency/license/SBOM checks, schema compatibility, protocol vectors, reference-model tests, secret scanning, and unit tests. | project owner | documented commands and a blocking CI pipeline. |
| P0.03 | [x] Record D-001 in an ADR: selected supported native stack, minimum OS versions, lifecycle APIs, and release/signing approach for Windows and Android. | project owner | spike-build, secure-storage, and lifecycle-test evidence. |
| P0.04 | [x] Record D-002 in an ADR and canonical-encoding specification. | project owner | bounded deterministic encoding rules, field ordering/types, duplicate-field rejection, integer/string rules, Unicode policy, version negotiation, and digest input. |
| P0.05 | [x] Record D-003 in an ADR. | project owner | maintained platform-reviewed API/library capability matrix, approved AES-GCM/HKDF/Argon2id/digest/signing/key-exchange choices, Argon2id benchmarked parameters, and review plan. |
| P0.06 | [x] Record D-004 and D-005 in ADRs. | project owner | owner-key custody/transfer rules, genesis trust anchor, epoch-key rotation and per-recipient distribution rules, retention boundary, and old-epoch cutover behavior. |
| P0.07 | [x] Record D-006 in an ADR and freeze the task schema. | project owner | field-level LWW tuple comparator, tombstone/restore semantics, mutation kinds, field bounds, and merge-history UX requirement. |
| P0.08 | [x] Record D-007 and D-008 in ADRs. | project owner | local-at-rest coverage for database derivatives, WAL/journal, caches, exports, backups, and crash artifacts plus fail-closed counter rollback/clone policy. |
| P0.09 | [x] Record D-009 in an ADR. | project owner | signed snapshot acceptance, per-origin coverage, monotonic checkpoint, retention, stale/incomplete/replay rejection, and repair rules. |
| P0.10 | [x] Record D-010 in an ADR and cloud API contract. | project owner | append idempotency, operation-ID/digest conflict behavior, durable receipts, cursor ordering/expiry, retention, bounded requests, and retry outcome semantics. |
| P0.11 | [x] Record D-011 in an ADR and recovery-package contract. | project owner | user-held package boundary, versioned KDF limits, AEAD AAD, encrypted keyring/trust contents, locator, and no-server-passphrase-reset behavior. |
| P0.12 | [x] Write the numbered envelope-verification algorithm. | project owner | fail-closed order for limits, structure, version, canonical re-encoding, routing, identity, manifest, signature, nonce/key domain, AEAD, plaintext schema, and merge transaction, including retry versus quarantine failures. |
| P0.13 | [x] Specify the versioned envelope and nonce contract. | project owner | exact canonical header/AAD/signature inputs, nonce layout/counter width/initial value/exhaustion limit, key-domain binding, required rejection cases, and compatible error codes. |
| P0.14 | [x] Specify HLC, mutation, membership, revocation, ownership-transfer, and epoch-cutover state machines. | project owner | valid transitions, retry rules, pre/post-cutover behavior, and deterministic ordering rules. |
| P0.15 | Specify the enrollment handshake. | project owner | owner initiation, new-device keys, bounded QR payload, expiring single-use capability, authenticated key exchange, confirmation code, encrypted epoch-key transfer, membership publication, cancellation, and disappearance handling. |
| P0.16 | Specify cloud and relay transport contracts. | project owner | separate service authentication/device-signature authorization, cloud pagination and snapshot APIs, relay manifest exchange and range inventory encoding, bounded frames, acknowledgement meaning, and reconnect rules. |
| P0.17 | Specify recovery, compatibility, and error contracts. | project owner | restore transaction and wrong-passphrase behavior, supported-version/deprecation/downgrade/rollback policy, and non-secret stable error taxonomy. |
| P0.18 | Publish the threat-model review, retention policy, account/enrollment authentication design, and service metadata limitations. | project owner | reviewed documents that explicitly separate service access from cryptographic authorization and define offline old-epoch handling. |

### - [ ] G0 — Protocol and product freeze

**Prerequisites:** P0.01–P0.18 are complete.

| ID | Task | Owner | Deliverable |
|---|---|---|---|
| G0.01 | Approve ADRs D-001 through D-011. | project owner | approved ADR set with protocol consequences and executable evidence for each decision. |
| G0.02 | Review and freeze the canonical schema, task/snapshot rules, protocol contracts, limits, and error taxonomy. | project owner | signed-off versioned specification bundle. |
| G0.03 | Verify positive and negative vectors run in CI and unresolved critical threat-model findings are closed. | project owner | passing CI report and review record. |
| G0.04 | Record the release sequence as local-only before cloud sync and defer relay implementation. | project owner | approved delivery-slice record. |

---

## - [ ] P1 — Encrypted local core

**Prerequisites:** G0 is passed. Implement only the frozen contracts; any needed contract change returns to P0 rules.

| ID | Task | Owner | Deliverable |
|---|---|---|---|
| P1.01 | Implement the reference mutation/merge model. | project owner | deterministic task state oracle for create, update, complete, restore, delete, and local-only purge request. |
| P1.02 | Implement persisted HLC generation and receive merge. | project owner | monotonic restart-safe clock with canonical tuple comparison and skew diagnostics input. |
| P1.03 | Implement canonical protocol parsers and validators. | project owner | bounded duplicate-rejecting decode/re-encode verification before crypto or storage application. |
| P1.04 | Implement device identity and owner trust-anchor creation. | project owner | separate signing/encryption device keys, stable random device ID, separate owner signing key, and signed genesis manifest stored through platform secure storage. |
| P1.05 | Implement epoch-root generation and scoped key derivation. | project owner | random epoch 0 key, epoch/purpose/origin-device HKDF contexts, historical-key retention, and no passphrase-derived vault key. |
| P1.06 | Implement persisted nonce reservation. | project owner | transactionally reserved monotonic counter, specified nonce construction, exhaustion stop, rollback detection, and fail-closed write path. |
| P1.07 | Implement signed encrypted envelopes. | project owner | canonical AES-GCM envelope creation and verification that authenticates header/AAD, checks manifest authorization, and rejects invalid versions, signatures, tags, and nonce domains. |
| P1.08 | Implement owner-signed manifest-chain verification. | project owner | contiguous genesis-to-current verification, historical-manifest lookup by epoch, fork/unauthorized-change rejection, and retained applicable manifests. |
| P1.09 | Implement encrypted local-store schema and wrapping. | project owner | encrypted vault records, log, materialized view, WAL/journal and disk-derivative handling defined by D-007, with non-secret operational metadata only where permitted. |
| P1.10 | Implement the local mutation transaction. | project owner | nonce reservation, HLC advancement, encrypted immutable log insert, operation-ID/digest integrity check, materialized-view update, and outbox enqueue in the required durable order. |
| P1.11 | Implement local apply and deduplication. | project owner | immutable operation-ID log, canonical-digest conflict quarantine, per-origin sequence/range coverage, idempotent merge, and tombstone visibility rules. |
| P1.12 | Implement snapshot primitives. | project owner | encrypted snapshot creation/verification with signed manifest, state/tombstone commitments, coverage indexes, compatibility checks, and no-silent-local-replacement rule. |
| P1.13 | Implement local repair/full-resync primitives. | project owner | preserve-and-mark-repair path that installs only compatible verified snapshot coverage and replays later operations without silently discarding local data. |
| P1.14 | Implement the user-held recovery package. | project owner | Argon2id-derived wrapping, AEAD-verified versioned package containing encrypted keyring, trust state, retained keys, locator, and fresh device/nonce domain on restore. |
| P1.15 | Implement encrypted export/import staging. | project owner | explicit encrypted export/import flow that never writes plaintext vault content to excluded disk locations and preserves recovery limitations. |
| P1.16 | Add transaction-boundary fault injection. | project owner | reproducible crash, rollback, full-disk, corrupted-row/snapshot, migration-interruption, and secure-storage-unavailability scenarios. |
| P1.17 | Add core test suites. | project owner | unit tests for HLC, tuples, manifest/cutover, outbox, cursor primitives, digest conflicts, and recovery plus property tests for permutations, replay, duplicate, drop, and reorder convergence. |
| P1.18 | Implement the vector runner and seed vectors. | project owner | canonical-byte, valid-envelope, malformed/negative-envelope, manifest, recovery, nonce, HLC, merge, snapshot, and compatibility vector fixtures runnable by both clients. |
| P1.19 | Run vectors independently on Windows and Android core implementations. | project owner | byte-for-byte vector report and matching reference-model resulting-state report. |

### - [ ] G1 — Durable local vault

**Prerequisites:** P1.01–P1.19 are complete.

| ID | Task | Owner | Deliverable |
|---|---|---|---|
| G1.01 | Verify offline task mutations recover across forced crashes at every local transaction boundary without an empty replacement vault. | project owner | fault-test log and recovery walkthrough. |
| G1.02 | Verify resumable migrations, encrypted-storage inspection, and secure-storage-loss handling. | project owner | inspection report with no prohibited plaintext and recoverable failure evidence. |
| G1.03 | Verify network-free encrypted export/import and recovery-package restore, including KDF limits and AEAD failure. | project owner | restore test report. |
| G1.04 | Verify uncertain nonce state fails closed and does not reuse a nonce. | project owner | rollback/clone/crash test evidence. |

### - [ ] G2 — Cross-platform convergence

**Prerequisites:** G1 is passed and P1.19 is complete.

| ID | Task | Owner | Deliverable |
|---|---|---|---|
| G2.01 | Verify both platforms consume identical canonical and negative vectors. | project owner | cross-platform vector CI report. |
| G2.02 | Verify valid permutations, duplicates, delay, replay, and reordering converge to one materialized state. | project owner | model/property test report. |
| G2.03 | Verify HLC rollback/skew, tombstone/restore, epoch cutover, and counter tests. | project owner | cross-platform core correctness report. |
| G2.04 | Define how the product will expose merge/history evidence for overwritten concurrent edits. | project owner | approved UX behavior note tied to the frozen merge contract. |

---

## - [ ] P2 — Local-only native product

**Prerequisites:** G1 and G2 are passed. Cloud accounts, cloud transport, and relay transport are not exposed in this phase.

| ID | Task | Owner | Deliverable |
|---|---|---|---|
| P2.01 | Build the Windows vault create/unlock flow on the encrypted local core. | project owner | native flow with secure-storage failure and locked-vault recovery states. |
| P2.02 | Build the Android vault create/unlock flow on the encrypted local core. | project owner | native flow that respects device lock/key availability and has recoverable failure states. |
| P2.03 | Implement the native four-quadrant matrix views. | project owner | local materialized tasks rendered by quadrant without adding remote assumptions. |
| P2.04 | Implement bounded task create and edit UI. | project owner | title/notes/quadrant field validation and local mutation submission. |
| P2.05 | Implement complete, restore, delete, and move UI actions. | project owner | explicit mutations with completed/deleted views and no implicit tombstone clearing. |
| P2.06 | Implement local conflict/history evidence. | project owner | user-visible indication sufficient to understand a winning concurrent LWW update without exposing secrets. |
| P2.07 | Implement recovery-package creation and encrypted local export/import UX. | project owner | explicit user-held backup/recovery actions and clear required-secret messaging. |
| P2.08 | Implement corruption and repair UX. | project owner | preserve-local-vault, repair-required path that cannot silently create an empty vault over recoverable data. |
| P2.09 | Implement lifecycle-safe local persistence. | project owner | Windows suspension/resume and Android process-death/Doze/network-change paths resume from a consistent transaction boundary. |
| P2.10 | Apply local privacy controls. | project owner | platform-appropriate suppression of task plaintext from logs, analytics, crash reports, backups, notifications, clipboard, and app-switcher previews where controls permit. |
| P2.11 | Implement honest local-only status and failure messaging. | project owner | visible local-only state with no sync-ready claim and recovery limitations stated. |
| P2.12 | Add native acceptance tests. | project owner | Windows and Android offline task, restart, backup/export, recovery, corruption, and lifecycle scenario results. |

### - [ ] G3 — Local-only product release

**Prerequisites:** P2.01–P2.12 are complete.

| ID | Task | Owner | Deliverable |
|---|---|---|---|
| G3.01 | Verify both native clients can create/unlock a vault and perform create, edit, complete, restore, delete, and move operations offline. | project owner | device-matrix acceptance report. |
| G3.02 | Verify local recovery/export/import and rollback behavior before user reliance. | project owner | independent local-only recovery walkthrough results. |
| G3.03 | Verify local-only status and repair messaging do not overstate sync, backup, or recovery guarantees. | project owner | reviewed product-copy and UX evidence. |

---

## - [ ] P3 — Cloud-sync beta

**Prerequisites:** G3 is passed; G0 cloud contracts and D-010 are frozen; the local log/outbox and repair primitives remain the source of client truth.

| ID | Task | Owner | Deliverable |
|---|---|---|---|
| P3.01 | Implement account/session registration, refresh, and service authorization boundaries. | project owner | short-lived authenticated service access that is explicitly separate from vault-operation authorization. |
| P3.02 | Implement owner-controlled device enrollment. | project owner | separate new-device signing/encryption identities, authenticated QR flow, confirmation code, expiring/cancelled capability, encrypted epoch-key transfer, and durable membership mutation. |
| P3.03 | Implement membership publication and retrieval. | project owner | immutable versioned owner-signed manifests with replay protection and client-side chain verification. |
| P3.04 | Implement ownership transfer, revocation, rotation, and cutover flows. | project owner | per-recipient new epoch-key delivery, declared cutover, old-epoch rejection, re-enrollment requirement, and clear advisory-until-rotation UX. |
| P3.05 | Implement opaque cloud-store persistence and migrations. | project owner | append-only vault-partitioned records containing only allowed routing/operational fields and encrypted envelope bytes. |
| P3.06 | Implement idempotent cloud append. | project owner | bounded one/batch upload, operation-ID plus digest dedupe, conflicting-digest rejection, per-item outcome, and digest-bound durable receipt. |
| P3.07 | Implement cloud reads and cursor lifecycle. | project owner | bounded pagination by opaque cursor, ordered envelope delivery, continuation, expired-cursor/full-resync response, and no task ordering derived from cursor. |
| P3.08 | Implement authenticated snapshot endpoints. | project owner | bounded upload/advertisement/download of signed opaque snapshot artifacts without server plaintext interpretation. |
| P3.09 | Implement service validation and operational limits. | project owner | TLS/authentication/tenant isolation, schema/size/rate/concurrency/quota checks before allocation, non-content audit events, and explicit limit responses. |
| P3.10 | Implement retention, backup, deletion, and health operations. | project owner | documented retention semantics, encrypted service storage/backups, quota/health status, and no overclaim of deletion or availability. |
| P3.11 | Implement the client cloud transport adapter. | project owner | locally stored transport configuration, authentication, protocol compatibility check, and transport-specific cursors/checkpoints. |
| P3.12 | Implement durable outbox upload and receipt handling. | project owner | safe retry/backoff and delivery marking only after the matching durable digest-bound receipt. |
| P3.13 | Implement cloud receive and acknowledgement handling. | project owner | bounded fetch, full envelope verification, local insert/merge/cursor commit in one transaction, then acknowledgement after that durable commit. |
| P3.14 | Implement cloud bootstrap, compaction safety, and repair. | project owner | verified snapshot-plus-watermark bootstrap, coverage-safe pruning, cursor-expiry/missing/corruption full resync, and preserved local discrepancies. |
| P3.15 | Implement cloud transport status UX. | project owner | evidence-bound connecting/syncing/caught-up-through-last-observed-cursor/offline/auth/quota/repair states, last receipt/checkpoint, and non-secret errors. |
| P3.16 | Implement cloud contract and adversarial integration tests. | project owner | fault-injected staging service tests for forged operations, retries, conflicts, pagination, expiry, snapshots, acknowledgement ordering, retention, and full resync. |
| P3.17 | Add cloud observability and support runbooks. | project owner | non-content operational signals, redacted server logging, incident/restore/retention/key-compromise guidance, and support prohibition on requesting secrets. |

### - [ ] G4 — Cloud-sync beta

**Prerequisites:** P3.01–P3.17 are complete.

| ID | Task | Owner | Deliverable |
|---|---|---|---|
| G4.01 | Demonstrate in staging that cloud service processing cannot decrypt task content and service tokens cannot authorize a forged operation. | project owner | adversarial staging report. |
| G4.02 | Verify idempotent append, conflicting operation-ID/digest rejection, and digest-bound receipts. | project owner | API fault matrix. |
| G4.03 | Verify acknowledgement follows local durable commit and outbox recovery survives interruption. | project owner | transaction/interruption test evidence. |
| G4.04 | Verify cursor expiry, missing operations, snapshot bootstrap, retention, repair, and full resync preserve local data. | project owner | bootstrap and repair scenario report. |

---

## - [ ] P4 — Cloud hardening

**Prerequisites:** G4 is passed. Cloud beta reconciliation and repair behavior must remain stable while hardening work proceeds.

| ID | Task | Owner | Deliverable |
|---|---|---|---|
| P4.01 | Test protocol and database compatibility rollout paths. | project owner | versioned migration matrix, supported-reader window, downgrade prevention, and rollback behavior for partially deployed clients. |
| P4.02 | Test cloud storage migration interruption and restoration. | project owner | resumable migration, backup, retention, and restore exercise evidence. |
| P4.03 | Expand parser, manifest, snapshot, and allocation-limit fuzzing. | project owner | reproducible fuzz corpus and results for malformed/oversized/untrusted input before decryption. |
| P4.04 | Run cloud load and limit testing. | project owner | bounded payload/batch/rate/quota/cursor-expiry behavior and queue/storage-growth report without content telemetry. |
| P4.05 | Run cross-platform cloud lifecycle testing. | project owner | Windows suspension and Android background/process-death reconnection and reconciliation report. |
| P4.06 | Rehearse cloud backup, restore, retention, and disaster recovery. | project owner | runbook execution record showing preserved local-vault and service recovery behavior. |
| P4.07 | Complete independent crypto/protocol review for cloud sync. | project owner | closed findings or explicitly accepted residual findings with remediations tracked. |
| P4.08 | Validate cloud deployment controls. | project owner | separate environment identities/secrets/data, pinned reproducible inputs, signed artifacts, health checks, rollback procedure, least-privilege access, dashboards, and alerts. |
| P4.09 | Perform a staged cloud rollout rehearsal. | project owner | rollout/rollback decision record that preserves evidence-bound caught-up wording. |

### - [ ] G5 — Cloud-sync release hardening

**Prerequisites:** P4.01–P4.09 are complete.

| ID | Task | Owner | Deliverable |
|---|---|---|---|
| G5.01 | Verify migration, rollback, restore, retention, fuzz, load, and cross-platform lifecycle evidence meets the cloud release bar. | project owner | reviewed cloud hardening packet. |
| G5.02 | Verify independent crypto/protocol findings are closed or formally accepted and cloud operational runbooks work. | project owner | security/operations sign-off record. |
| G5.03 | Verify cloud status remains limited to “caught up through the last observed cloud cursor” and does not claim universal delivery. | project owner | reviewed status and product-copy evidence. |

---

## - [ ] P5 — Volatile relay

**Prerequisites:** G5 is passed. Record D-012 with its required evidence before relay implementation. Relay is opt-in and cannot block local use, durable recovery, or the already stable cloud reconciliation path.

| ID | Task | Owner | Deliverable |
|---|---|---|---|
| P5.01 | Record D-012 in an ADR. | project owner | relay launch bar defining bounded active-session memory, non-persistence, common anti-entropy protocol, and persistence/flood/reconnect evidence requirements. |
| P5.02 | Implement relay capability lifecycle. | project owner | high-entropy vault/session-scoped capability with expiry and revocation/rotation behavior exchanged only through authenticated enrollment or user-confirmed channels, never logged in URLs. |
| P5.03 | Implement WSS relay authentication and bounded connection state. | project owner | authenticated handshake/header/subprotocol design, per-capability peer limits, and minimal in-memory routing state only. |
| P5.04 | Implement opaque bounded peer routing. | project owner | no decrypt, merge, durable acknowledgement, or persistence of operation payloads and explicit incompatible/abusive-peer error codes. |
| P5.05 | Implement relay peer handshake. | project owner | protocol version, identity/membership proof, signed manifest/snapshot summaries, and persisted anti-entropy inventory exchange. |
| P5.06 | Implement inventory and range reconciliation. | project owner | per-origin contiguous sequence plus received-range and snapshot-coverage exchange, bounded range requests, and immutable envelope batches without cloud cursors. |
| P5.07 | Implement relay client commit/ack semantics. | project owner | common envelope validation, local durable commit before peer acknowledgement, retained local outbox, and no relay-delivery-as-backup claim. |
| P5.08 | Implement relay resource controls. | project owner | per-peer frame/rate/queue/handshake/idle limits, slow-peer isolation, bounded buffering, and explicit disconnect behavior. |
| P5.09 | Implement reconnect behavior. | project owner | discarded volatile assumptions, reauthentication, summary exchange, and reconciliation after every reconnect. |
| P5.10 | Implement safe transport enable/disable/switch flows. | project owner | non-destructive cloud/relay configuration, separate delivery state, common log/outbox, no shared cursor, and safe dual-transport dedupe. |
| P5.11 | Implement relay status UX. | project owner | reconciled-with-currently-connected-peers/waiting/offline/repair states that never imply durable remote backup. |
| P5.12 | Add relay integration and abuse tests. | project owner | capability theft, flooding, oversized frame, slow peer, dropped frame, reconnect, range-reconciliation, and dual-transport convergence results. |
| P5.13 | Perform an independent relay persistence audit. | project owner | evidence that operation payloads are absent outside bounded active-session memory. |

### - [ ] G6 — Relay release

**Prerequisites:** P5.01–P5.13 are complete.

| ID | Task | Owner | Deliverable |
|---|---|---|---|
| G6.01 | Verify relay payload persistence is absent outside bounded active-session state. | project owner | independent persistence-audit report. |
| G6.02 | Verify capability theft, peer flooding, oversized frames, slow peers, reconnects, and drops fail safely within limits. | project owner | resource-limit and abuse-test report. |
| G6.03 | Verify relay reconciliation and dual-transport dedupe converge while relay UI never implies backup. | project owner | convergence and UX evidence. |

---

## - [ ] P6 — Public-release evidence

**Prerequisites:** G6 is passed. All prior release evidence remains current for the candidate build.

| ID | Task | Owner | Deliverable |
|---|---|---|---|
| P6.01 | Run end-to-end Windows/Android device scenarios. | project owner | offline edits, QR enrollment, recovery-package restore, rotation/cutover with unseen old-epoch mutations, mode switching, suspension/restart, and status-accuracy results. |
| P6.02 | Run the corner-case release suite. | project owner | evidence for concurrent fields, HLC rollback/skew, delete/restore races, replay/malformed frames, digest conflicts, nonce exhaustion, snapshot coverage, compaction, and transport-specific state. |
| P6.03 | Complete storage and secret-bearing data inspection. | project owner | report covering databases, WAL/journal, caches, backups, exports, logs, telemetry, crash reports, diagnostics, and committed fixtures. |
| P6.04 | Run final security assurance. | project owner | dependency scan, authorization tests, rate-limit tests, threat-model review, and independent crypto/protocol review closure or accepted residual findings. |
| P6.05 | Validate deployment and operational readiness. | project owner | signed reproducible builds, staged rollout, rollback rehearsal, dashboards, alerts, service SLO documentation, backup/disaster-recovery exercise, and least-privilege access review. |
| P6.06 | Validate privacy and support readiness. | project owner | metadata/availability limitations, telemetry retention/access documentation, support guidance for loss/revocation/corruption/outage/relay-only use, and explicit no-secret-sharing policy. |
| P6.07 | Assemble the public release-readiness packet. | project owner | traceable links to all gate evidence, open-risk disposition, release artifact identifiers, and approvals from engineering, security, privacy, and operations. |

### - [ ] G7 — Public-release evidence

**Prerequisites:** P6.01–P6.07 are complete.

| ID | Task | Owner | Deliverable |
|---|---|---|---|
| G7.01 | Confirm fuzz, load, migration, lifecycle, backup/restore, and disaster-recovery tests pass for the release candidate. | project owner | final consolidated test report. |
| G7.02 | Confirm independent review is closed or residual findings are explicitly accepted. | project owner | security approval record. |
| G7.03 | Confirm reproducible signed artifacts, rollback procedures, monitoring, alerts, privacy documentation, support guidance, and staged rollout are operational. | project owner | operations/privacy approval record. |
| G7.04 | Approve the release-readiness packet without claiming anonymity, guaranteed availability, or universal device delivery. | project owner | recorded engineering, security, privacy, and operations release decision. |

---

## Traceability to `plan-To-Do-App.md`

| Phasing Plan | `plan-To-Do-App.md` Reference |
|---|---|
| P0 — Decisions and protocol freeze | §14 Phase 0, §17 Decisions, §18 Protocol contract deliverables, §19 Repository bootstrap, §21 Gate 0 |
| G0 — Protocol and product freeze | §17 Decisions, §18 Protocol contract deliverables, §21 Gate 0 |
| P1 — Encrypted local core | §5 Key hierarchy, §6 Local data model, §11 Client lifecycle, §21 Gate 1, §21 Gate 2 |
| G1 — Durable local vault | §21 Gate 1, §12 Testing |
| G2 — Cross-platform convergence | §21 Gate 2, §12 Testing |
| P2 — Local-only native product | §11 Client lifecycle, §14 Phase 2, §21 Gate 1/2 |
| G3 — Local-only product release | §16 Slice 1, §21 Gate 1/2 |
| P3 — Cloud-sync beta | §4 Vault/account/device, §7 Sync protocol, §8 Cloud delivery, §14 Phase 3, §21 Gate 3 |
| G4 — Cloud-sync beta | §21 Gate 3 |
| P4 — Cloud hardening | §13 Migration/deployment, §14 Phase 5, §21 Gate 3/5, §22 Risk register |
| G5 — Cloud-sync release hardening | §21 Gate 3/5 |
| P5 — Volatile relay | §7 Sync protocol, §9 Volatile P2P relay, §14 Phase 4, §21 Gate 4 |
| G6 — Relay release | §21 Gate 4 |
| P6 — Public-release evidence | §10 Transport UX, §12 Testing, §15 Corner-case checklist, §16 Delivery strategy, §21 Gate 5 |
| G7 — Public-release evidence | §21 Gate 5 |
