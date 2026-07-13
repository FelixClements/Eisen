# Eisen — V1 Architecture and Implementation Plan

## 1. Product intent and V1 scope

Eisen is a **local-first native** task application organized as an Eisenhower Matrix (urgent × important). A user's local device remains usable without an account, network connection, or sync service. Windows and Android are V1 targets. macOS is explicitly future scope and must not delay V1.

V1 offers three data-transport states for a vault:

| State | V1 behavior | Availability trade-off |
| --- | --- | --- |
| Local only | Data remains on the device. | No multi-device recovery or sync. |
| Encrypted cloud sync (optional) | Clients exchange encrypted, immutable operations through an opaque delivery store. | Devices may synchronize at different times. |
| Volatile relay (optional) | Simultaneously connected peers exchange the same encrypted operations through a non-persistent relay. | A peer must be online; missed operations are not retained by the relay. |

The cloud and relay are transports, not authorities and not a plaintext “master copy.” Conflict resolution is deterministic and runs entirely on clients.

### Included in V1

- Create, edit, complete, restore, and delete tasks; move tasks among the four matrix quadrants.
- A local encrypted vault with an append-only mutation log and materialized task view.
- Optional vault account, device enrollment, cloud delivery, and volatile relay transport.
- Cross-device convergence using one encrypted, idempotent mutation protocol.
- Explicit backup/export and device-revocation flows, subject to the recovery limitations below.
- Native Windows and Android applications. The UI may be functionally simple until protocol correctness is established.

### Explicitly out of scope for V1

- macOS client (future: native macOS implementation using the established protocol and test vectors).
- Server-side task search, task analytics, server-side conflict resolution, or server-side plaintext notifications.
- Collaborative/shared vaults, per-task permissions, web client, and automated recovery of lost secrets.
- Guaranteed background synchronization on every Android vendor/OS configuration.
- Hiding all network metadata from the service or from a network observer.

## 2. Threat model, guarantees, and limitations

### Security goals

- Task content, task identifiers where practical, mutation contents, and snapshot contents are encrypted before leaving a device.
- The cloud storage provider and relay operator must not need vault keys to route, persist, deduplicate, or paginate operations.
- A compromised transport must not be able to cause a valid operation to be applied more than once or silently alter authenticated ciphertext.
- A client must retain a locally durable mutation before it reports that mutation synchronized or before it acknowledges a received mutation.

### Threats addressed

- Honest-but-curious or breached cloud/relay infrastructure reading stored or forwarded payloads.
- Network attackers modifying, replaying, dropping, duplicating, or reordering transport frames.
- Concurrent offline edits from enrolled devices.
- Loss of connectivity, restarts during sync, and repeated upload/download attempts.

### Boundaries and non-guarantees

- **Metadata remains visible.** Cloud infrastructure necessarily sees account and device routing identifiers, operation sizes, delivery timing, IP/network metadata, retention/usage records, and approximate activity volume. A relay sees connection metadata, relay capability use, peer presence while connected, sizes, timing, and routing events. Encryption does not make either mode anonymous or hide that a vault is active.
- **Availability is not guaranteed.** A cloud outage, relay outage, network censorship, or a malicious service can delay, drop, withhold, or delete deliveries. Cloud sync improves asynchronous delivery; volatile relay does not provide offline delivery.
- **Endpoint compromise is out of scope.** Malware, a rooted/jailbroken device, an unlocked user session, screenshots, or a compromised secure-storage provider can expose plaintext and keys.
- **No lost-key recovery promise.** If every enrolled device and every user-held recovery/passphrase secret are lost, the vault cannot be decrypted. Support staff and servers cannot reset a vault key without destroying access to existing encrypted data.
- **Cryptographic implementation needs review.** Use maintained, platform-reviewed cryptographic APIs/libraries and protocol test vectors. Do not implement primitives manually.

## 3. Decisions, open choices, and repository shape

The exact persistence library, UI framework, backend language/framework, database, hosting provider, and WebSocket library are **not architecture decisions in this document**. Candidate native stacks may be evaluated separately, but V1 requires Windows-native and Android-native clients with SQLite-or-equivalent transactional local storage and platform-backed secure storage.

Suggested repository boundaries (names are illustrative, not a mandated stack):

```text
clients/windows/       native Windows app, storage adapter, platform key adapter
clients/android/       native Android app, storage adapter, platform key adapter
protocol/              canonical schema, vectors, compatibility documentation
server/cloud/          opaque cloud delivery service
server/relay/          volatile relay service (may share deployment code with cloud)
test-vectors/          cross-platform encrypted envelopes and merge cases
docs/                  threat model, operations runbooks, API specification
```

The protocol schema, canonical serialization, and test vectors are shared artifacts. Each client may have its own native implementation; do not require a shared runtime just to share crypto or merge code.

## 4. Vault, account, and device model

### Vault and account boundaries

- A **vault** is the encryption and synchronization unit. It contains tasks, encrypted mutation history, encrypted snapshots, signed non-secret membership/control-plane records, and local settings that are safe to sync.
- A **local-only vault** is created without an account. It can later be prepared for cloud or relay sync by creating/enrolling an account and devices; that transition is an explicit, backed-up user action.
- An **account** is an optional service identity used for cloud authorization, quotas, and device enrollment. It is not the cryptographic owner of task content and must not receive the vault key in plaintext.
- A person may have more than one vault. Do not use an email address, room name, or account ID as a vault encryption key.

### Device identities and enrollment

- On first use, each device generates separate long-lived signing and encryption key pairs and a stable random `device_id`. Private identity material is held in platform secure storage. Public keys are published in the signed, non-secret membership manifest; the service may also receive the minimum routing copies it needs.
- A new device is enrolled by an already enrolled device, preferably through an authenticated QR transfer. The QR flow must establish the intended peer/device identity, transfer or derive access to the vault key over an authenticated encrypted channel, and require user confirmation on both devices.
- A recovery/passphrase flow is an alternative only when the user has deliberately configured one. It unwraps the vault key; it is not a new, server-known encryption key.
- Every enrollment has a durable membership mutation and a new signed membership manifest. Every operation envelope is signed by its origin device signing key; a recipient verifies that signature against the manifest version applicable to the envelope's epoch before decrypting or applying it. Transport account/session authentication only authorizes service access; it is not cryptographic authorization to create or apply a vault operation.

### Cryptographic membership control plane

- A versioned, canonical, non-secret membership manifest contains the `vault_routing_id`, protocol version, membership version, key epoch and revocation/cutover state, enrolled device IDs, their signing and encryption public keys, and device status. It is signed by the V1 vault owner signing key. The owner key and signed genesis manifest are the trust anchor, created with the vault and retained in every recovery package; owner-controlled membership changes are the V1 authorization policy (multi-owner quorum is deferred).
- An owner-authorized device creates each replacement manifest, increments its membership version, and signs it with the owner key; all clients verify a contiguous chain to genesis, reject forks or unauthorized changes, and retain applicable historical manifests. A device may not self-enroll, revoke, or rotate membership merely because it has a valid service token.
- Cloud membership endpoints distribute immutable manifest versions, and peers exchange them during enrollment and relay reconciliation. Recipients fetch missing versions, verify them before use, and validate each envelope against its declared epoch and the corresponding applicable manifest. The service can authenticate uploaders and enforce a presented manifest version, but cannot replace client signature validation.

### QR, recovery, and revocation boundaries

- QR codes may contain a short-lived enrollment capability, public keys, protocol version, and connection/bootstrap information. They must not contain a raw vault key, a reusable account password, or unbounded sensitive history.
- Enrollment capabilities are high entropy, single-use where a service participates, expire quickly, and are invalidated after completion/cancellation. Display a human-verifiable confirmation code in addition to QR scanning.
- V1 recovery is a **user-held encrypted recovery package**, generated client-side and never uploaded as plaintext. It contains a versioned package header, vault routing ID/restore locator, encrypted vault-root and current-epoch material, the genesis trust anchor and required membership manifests, package nonce/ciphertext/tag, and the versioned Argon2id salt and parameters used to derive its passphrase credential. AEAD verification is mandatory before any key or metadata is used. To restore, the user supplies the package and passphrase, the client validates format/KDF limits and AEAD, restores keys and trust state into secure storage, then uses the locator plus separately authenticated account/device enrollment to obtain a compatible snapshot and later operations. The service cannot validate or reset the passphrase. A cloud-hosted encrypted recovery record is explicitly deferred and is not a V1 alternative.
- Revocation is advisory until rotation: possession of an old signing key cannot prove when an old-epoch envelope was created. The owner publishes the revocation/rotation manifest, encrypts the new root/epoch material separately to each remaining recipient's manifest encryption public key, and establishes a documented cutover after confirmed distribution or a declared deadline. At cutover, clients and services accept only new-epoch envelopes/signatures and reject all old-epoch envelopes, including legitimate previously unseen offline mutations. Such a device must be re-enrolled; its user may manually review and reissue desired changes as new operations. Revocation cannot retract ciphertext already copied by that device, erase plaintext it already read, or guarantee deletion of previously delivered server ciphertext.

## 5. Key hierarchy and encrypted envelope

### Key lifecycle

1. Generate a random 256-bit vault root key on the client using a cryptographically secure RNG. Never derive this root directly from a passphrase.
2. Store the root key (or a device-specific wrapping key for it) in platform secure storage. The local database stores only encrypted records and non-secret operational metadata.
3. When a user elects a recovery/passphrase option, derive a passphrase wrapping key using **Argon2id** with a unique random salt and versioned, stored parameters selected for the supported devices. Use it only to wrap the random vault key. Parameter values are configuration/version data, not protocol magic constants; benchmark and set them before release.
4. Derive subkeys from the vault root using HKDF with explicit, versioned context labels. Mutation and snapshot encryption keys are scoped at least by epoch, purpose, and origin device; membership/enrollment material and local database wrapping use separate purposes. Do not reuse an encryption key across unrelated purposes or origin devices.
5. For each origin-device encryption key, reserve and persist a strictly monotonic nonce counter before encryption; a crash may leave gaps but must never reuse a reservation. The AES-GCM nonce is constructed from the device's protocol-defined nonce domain and that counter, never selected randomly. Phase 0 must freeze the exact byte layout, counter width, encoding, initial value, and maximum-use limit; counters cannot reset or be restored from backup under the same key, and encryption stops for that key on exhaustion until a new authorized key epoch/device key is established.
6. Zeroize transient secret buffers where platform/runtime facilities make that meaningful, and never log keys, decrypted payloads, passphrases, QR enrollment secrets, or recovery data.

### Versioned envelope contract

All transmitted mutations and snapshots use a canonical, versioned encrypted envelope. The protocol specification must define canonical serialization before client implementation. An envelope includes, at minimum:

| Field | Purpose |
| --- | --- |
| `protocol_version` and `key_epoch` | Select parsing, crypto behavior, and key epoch. |
| `vault_routing_id` | Opaque routing scope; not a plaintext task identifier. |
| `operation_id`, `device_id`, origin sequence, and HLC | Idempotency, origin/range reconciliation, and deterministic ordering. |
| nonce/IV | Persisted-counter AES-GCM nonce, unique for that origin-device encryption key. |
| ciphertext and authentication tag | Authenticated encrypted mutation/snapshot content, including mutation kind and task data. |
| membership version and origin signature | Select the applicable authorization manifest and authenticate the canonical envelope. |
| authenticated header/AAD | Canonical non-secret routing and anti-substitution fields bound to the ciphertext. |

Use AES-GCM with a key appropriate for the selected platform API. Nonce size, tag size, serialization format, signature algorithm/input, and exact AAD field list must be specified in the versioned protocol; do not invent constants or rely on library defaults without recording the chosen, interoperable format. AAD binds version, vault routing scope, operation ID, origin device/sequence, epoch, membership version, and transport-independent anti-substitution fields. Mutation kind is ciphertext-only unless a future routing requirement explicitly makes it cleartext and adds it to AAD. The origin signature covers the canonical header/AAD, nonce, ciphertext, and tag. Reject malformed, unsupported-version, duplicate-field, signature-failed, unauthorized, or authentication-failed envelopes before applying them.

## 6. Local data model and deterministic merge

### Canonical local mutation log

Each client has one transactional local store containing:

- encrypted vault records and a materialized task view;
- an immutable `mutations` log keyed by globally unique `operation_id`;
- the canonical envelope digest for every operation ID, plus persisted per-origin sequence/range coverage and snapshot coverage indexes for peer anti-entropy;
- an `outbox` of locally durable, not-yet-confirmed delivery attempts;
- per-transport receive cursors/checkpoints and deduplication indexes;
- membership/revocation state, HLC state, key epoch, and snapshot/compaction metadata.

A local user action first durably reserves its nonce counter (a separate reservation commit is allowed and gaps are safe), then commits the action transaction: advance/persist HLC, construct and encrypt the immutable mutation, insert it into the log, update the materialized view according to merge rules, and enqueue it in the outbox. UI success is shown after the action transaction, not after network delivery.

The encrypted plaintext of each task mutation carries a stable random task ID, operation ID, originating device ID and sequence, HLC timestamp, mutation kind, and either a complete field update or a clearly versioned patch. For V1, prefer field-level last-writer-wins registers so independent edits (for example title and completion) do not overwrite each other unnecessarily. Every mutable field stores its winning version tuple.

### HLC and ordering

- Use a Hybrid Logical Clock (HLC) with persisted state so it remains monotonic across restarts. On receive, merge the remote HLC with local wall time and local logical state using the documented HLC algorithm.
- Compare versions lexicographically by `(hlc_physical, hlc_logical, device_id)`. Device ID is a stable random byte value with a canonical encoding; it is the deterministic final tie-breaker, not a security claim.
- Wall-clock time is display metadata only. Clock skew cannot make the merge nondeterministic, but an incorrectly far-future device clock can make its writes win. Apply documented skew diagnostics/UX and never rewrite remote timestamps merely to make them look reasonable.

### Delete, restore, and retention

- A delete is a tombstone mutation with its own version tuple; it wins over an older update. A newer restore mutation explicitly clears the tombstone and is visible only if it wins against the delete.
- An update targeting a deleted task is retained in the log but does not resurrect the task unless it is an explicit newer restore. Concurrent delete/update resolution follows the same tuple ordering; if tuples are equal only the canonical device-ID tie-break applies.
- Keep tombstones and dedupe information until every configured durable transport has passed the relevant compaction frontier and the retention policy permits removal. For V1, prefer conservative retention over aggressive deletion. A stale/offline device that returns after its history has been compacted must bootstrap from a snapshot and then replay later operations.
- Restore and delete actions must be visible in the UI. Permanent local purge is a separate destructive operation and must not silently discard sync evidence.

## 7. One sync protocol for cloud and relay

Cloud and relay carry the **same immutable encrypted operation envelopes** and use the same local apply, dedupe, cursor, snapshot, and reconciliation logic. Transport adapters may differ in authentication and retention, but neither may alter plaintext semantics or resolve conflicts.

### Operation lifecycle

1. Client commits an encrypted operation to its local log and durable outbox.
2. Client authenticates the transport and announces protocol version, vault routing ID, device ID, supported key epoch(s), receive checkpoint/snapshot generation, and (for relay) its persisted anti-entropy inventory.
3. Client uploads paginated outbox operations. The transport accepts an opaque envelope idempotently by `(vault_routing_id, operation_id, envelope_digest)`: a repeat with the same digest is idempotent, while the same operation ID with a different digest is a protocol-integrity rejection. Each durable receipt binds the operation ID and envelope digest. Upload retries are safe.
4. Client fetches/receives operations after its cursor in bounded pages/frames. It validates envelope/version/AAD/signature, verifies membership authorization, and compares the canonical envelope digest for an existing operation ID; a mismatch is a protocol-integrity error and is rejected by the client. It durably inserts valid unseen operations, deterministically updates the materialized view, and persists its receive cursor/checkpoint in the same local transaction.
5. Only after that local durable commit does the client acknowledge receipt to the transport/peer. An acknowledgement means locally committed, not merely displayed.
6. The client may remove or mark an outbox entry delivered only after the applicable transport receipt. Retain enough local history to serve peer reconciliation and support export/repair.

### Cursors, bootstrap, compaction, and repair

- Cloud cursors are opaque monotonically advancing delivery positions supplied by the cloud store; clients must not infer task order from them. Fetch APIs support bounded pagination, an `after_cursor`, a page limit, and a continuation cursor.
- A bootstrap obtains a versioned encrypted snapshot plus a watermark/cursor and signed snapshot manifest, verifies and commits them, then downloads operations after that watermark. A snapshot is an optimization, not an authority: it is generated from canonical mutation state and includes compaction frontier information.
- A snapshot manifest has canonical schema/protocol version, key epoch, membership version, snapshot/envelope digests, per-origin sequence/range coverage, a commitment to full materialized state and tombstone state, source device ID/signature, and retention boundary. Clients accept it only if the source was authorized by the applicable manifest, signature and commitments verify, and its epoch/membership schema is compatible; they then reconcile operations not covered by it. Multiple snapshots may exist.
- The client periodically publishes/uploads snapshots only through this authenticated snapshot process. No client or service may prune an operation until at least one recoverable, compatible, verified snapshot covers it and the conservative acknowledgement/retention policy permits pruning. Server retention/expiry may force a full resync.
- If a cursor is expired, state is inconsistent, an envelope is missing, or corruption is detected, preserve the local vault, mark sync as needing repair, and execute full resync: obtain a compatible snapshot, replay available operations, compare/checkpoint state, and surface any unrecoverable discrepancy. Never replace local data silently.

## 8. Cloud delivery store

### Service model

Cloud mode uses an opaque, append-only encrypted delivery store. It does **not** decrypt task data, maintain task rows, apply Last-Write-Wins, generate plaintext snapshots, or claim to be the authoritative task database.

Illustrative stored fields are:

| Field | Service use |
| --- | --- |
| `vault_routing_id` | Partition/routing scope. |
| server `sequence` | Opaque delivery order and cursor creation. |
| `operation_id` | Idempotent append/deduplication within a vault scope. |
| envelope digest | Detect conflicting bytes for a reused operation ID; bind receipts. |
| `origin_device_id`, `key_epoch`, protocol version | Validation/routing/operational compatibility only. |
| encrypted envelope bytes and size | Opaque payload delivery. |
| accepted time, retention state | Operations, quotas, and expiry. |

The service may enforce schema, size, rate, membership-token, quota, and signature/manifest authorization checks using non-secret public keys, but must not inspect encrypted task fields or choose mutation winners. A valid account/session token does not substitute for a valid origin signature under the applicable manifest.

### Required API behavior

The final API may be REST, streaming, or both, but it must expose equivalent behavior:

- account/session registration and token refresh, without treating account authentication as proof of vault-key possession;
- device enrollment, membership publication/retrieval, and revocation authorization with replay protection;
- idempotent append of one or a bounded batch of opaque operations, rejecting an operation-ID/digest conflict and returning per-operation acceptance/rejection and a receipt/sequence bound to its digest;
- paginated read by vault routing scope and opaque cursor, returning ordered envelopes and continuation cursor;
- authenticated snapshot advertisement/download/upload where enabled;
- delivery acknowledgement/checkpoint submission, health/quota status, and a clear full-resync/expired-cursor response.

### Security and operations

- Require TLS, strict authentication/authorization per account and enrolled device, short-lived access tokens, rate/size/concurrency limits, replay-safe requests, and audit events that exclude payloads and secrets.
- Encrypt service disks/backups, isolate tenants, use least-privilege service credentials, validate all untrusted fields, and cap request/frame sizes before allocation.
- Document retention, backup, deletion, and legal-access policies precisely. “E2EE” describes task-content confidentiality, not invisibility of account/network metadata or guaranteed deletion from all backups.
- Monitor availability, queue depth, error rates, cursor expiry, abuse limits, and storage growth using non-content telemetry. Provide incident, key compromise, retention, and restore runbooks.

## 9. Volatile P2P relay

Relay mode is a best-effort, non-persistent transport for the same reconciliation protocol. It does not improve anonymity: the relay operator and network can still observe connections and traffic metadata. It is “zero knowledge” only with respect to encrypted task content, assuming correct client encryption.

- Connect over WSS only. Authenticate a room/session with a high-entropy, unguessable relay capability; do not use a human-readable room ID as the sole credential.
- The capability is scoped to a vault/session, has an expiry and revocation/rotation behavior, and is exchanged through an authenticated enrollment or user-confirmed channel. Do not place it in URLs likely to be logged; use an authenticated handshake/header/subprotocol design selected during API design.
- The relay routes opaque bounded frames only among authorized peers for the capability. It does not decrypt, persist, merge, acknowledge as a durable store, or promise delivery. It may retain only minimal in-memory connection/routing state needed for an active session.
- On connection, peers exchange protocol version, identity/membership proof, signed manifest/snapshot summaries, and a persisted anti-entropy inventory: for every origin device, the highest contiguous origin sequence and explicit received ranges above it, plus snapshot coverage ranges. They range-request missing origin sequences and send the corresponding immutable envelopes in bounded batches; a snapshot may satisfy only the ranges its verified manifest covers. Cloud delivery cursors are store pagination positions, not relay inventories, and are never used as one. Peers use the same local durable-commit-before-ack rule as cloud mode.
- Apply per-capability peer counts, per-peer message/frame size, rate, queue, handshake, and idle-time limits. Disconnect abusive or incompatible peers with explicit error codes; never let one slow peer create unbounded buffering.
- On reconnect, discard volatile transport assumptions, reauthenticate, exchange summaries, and reconcile again. The client preserves its local outbox; it must not report relay delivery as durable remote backup.

## 10. Transport selection, mode switching, and sync UX

Transport configuration is separate from vault data. Local mutations are never deleted because a user disables cloud or relay.

- Enabling a transport first validates account/capability and protocol compatibility, records the configuration locally, then performs bootstrap/reconciliation. It does not overwrite the local view with remote state.
- Disabling cloud stops future uploads/downloads after in-flight work is safely checkpointed or cancelled; it does not request destructive server deletion by default. Offer a separately confirmed server-data deletion flow that explains retention/backups and does not affect local data.
- Leaving relay simply disconnects. It does not imply cloud upload, remote backup, or cleanup of another peer’s local copy.
- Switching cloud ↔ relay is non-destructive: retain the common log and outbox, create transport-specific delivery state, reconcile with the newly enabled transport, and preserve unresolved local state. Do not share a cursor between transports.
- If both transports are enabled, each independently carries the same operation IDs; local dedupe makes this safe. The UI must clarify that relay delivery is ephemeral and cloud delivery is subject to service retention/availability.
- Sync status must show at least: local-only, connecting, syncing, **caught up through the last observed cloud cursor** for cloud, **reconciled with currently connected peers** for relay, waiting for peers, offline, paused by OS, authentication/enrollment required, quota/limit blocked, and repair required. These evidence-bound states do not rule out service withholding or unavailable peers. Show last successful cloud receipt/checkpoint and meaningful errors without exposing content or secrets.

## 11. Client lifecycle and data at rest

### Windows

- Use a transactional local database and OS-supported secure storage/key protection. Keep the database, WAL/journal, snapshots, exports, logs, and crash reports within the appropriate user-data protection boundary.
- Sync in foreground and only through supported Windows lifecycle/background mechanisms. Background execution is opportunistic; persist work before suspension and resume reconciliation safely on launch.

### Android

- Use a transactional local database and Android Keystore-backed key protection where available. Respect device lock state and invalidate/recover gracefully when key material becomes unavailable according to platform policy.
- Use supported scheduled/background work constraints; do not depend on a continuously running socket. Treat process death, Doze, network changes, and vendor task killing as normal reconnect/reconcile events.

### Both clients

- Encrypt sensitive application records at rest with vault-derived/local wrapping keys as designed; platform secure storage protects root/wrapping material, not arbitrary database content by itself.
- Exclude vault databases, plaintext exports, and secret-bearing diagnostics from automatic backup unless the user has explicitly enabled a safe encrypted backup path. Avoid plaintext task content in notifications, clipboard, screenshots/app switcher previews where platform controls permit, logs, analytics, and crash reports.
- Use database transactions, fsync/durability semantics appropriate to the platform, schema migrations with backups, and corruption detection. A lock, secure-storage failure, or corrupted database must produce a recoverable user flow rather than a new empty vault that might overwrite data.

## 12. Testing and quality gates

### Required test layers

| Layer | Coverage |
| --- | --- |
| Crypto/protocol vectors | Canonical encoding, recovery-package Argon2id/AEAD verification, HKDF epoch/purpose/origin-device contexts, persisted-counter nonce uniqueness/exhaustion, AES-GCM envelope/AAD and signature validation, manifest chain/authorization, bad tags/nonces/signatures/versions, cross-language Windows↔Android compatibility. |
| Unit tests | HLC progression/persistence, tuple ordering, field merge, tombstone/restore behavior, owner-controlled manifest changes and revocation/cutover checks, old-epoch rejection, outbox transitions, cursor handling, and operation-ID/digest conflict rejection. |
| Property/fuzz tests | Random operation permutations and duplicate/replay/drop/reorder delivery; all valid replicas converge to the same materialized state. Fuzz parsers, manifest/snapshot verification, and allocation limits before decryption. |
| Storage fault tests | Crash between each local transaction step, full disk, transaction rollback, corrupted rows/snapshots, migration interruption, secure-storage unavailability. |
| Transport integration | Idempotent cloud append and conflicting-digest rejection, pagination/expired cursors, digest-bound receipt and acknowledgement semantics, signed-snapshot bootstrap/full-resync/pruning safety, relay inventory/range reconciliation, reconnect/backpressure/limits. |
| End-to-end device tests | Offline editing on Windows and Android, QR enrollment and user-held-package recovery, rotation/cutover including unseen old-epoch mutations, mode switches, OS suspension/restart, and evidence-bound UI status accuracy. |
| Security review | Dependency scanning, secret/log inspection, authorization tests, rate-limit tests, threat-model review, and independent crypto/protocol review before public sync release. |

CI must run protocol vectors and merge tests for every client implementation. A protocol change requires a versioning decision, compatibility tests, migration plan, and updated vectors before release.

## 13. Migration, observability, deployment, and support

### Migration and compatibility

- Start V1 with explicit database schema and protocol versions. Never infer versions from payload shape alone.
- Make local migrations transactional where possible, backed up before destructive transformations, resumable/idempotent after interruption, and tested from every supported prior schema.
- Preserve old decryptors/readers during a documented compatibility window. Key-epoch and protocol upgrades require an explicit rollout plan and rollback behavior; do not silently re-encrypt or discard old logs.
- Provide an encrypted export/import or recovery package design before encouraging users to rely on optional sync. Clearly state what secrets a user must preserve to restore it.

### Privacy-preserving observability

- Client telemetry is opt-in where required and excludes task plaintext, decrypted identifiers, keys, passphrases, capabilities, full encrypted payloads, and precise unnecessary timestamps.
- Measure only operational signals such as sync state transitions, protocol-version counts, aggregate latency/error categories, queue sizes, cursor-expiry counts, and background-work outcomes. Hashing an identifier is not automatically safe telemetry; document retention and access.
- Server logs redact tokens/capabilities and never log request bodies or encryption headers beyond the minimum required for operations. Use structured audit events for account/device administration and retention actions.

### Deployment and operations

- Separate development, staging, and production identities, secrets, and data. Use reproducible builds, dependency pinning, signed release artifacts, staged rollout, rollback, health checks, backups, disaster-recovery exercises, and least-privilege production access.
- Establish service SLOs for delivery availability/latency while documenting that these do not guarantee user-data availability. Alert on authentication failures, abuse, data-store errors, retention jobs, quota exhaustion, and relay resource pressure.
- Publish support guidance for device loss, forgotten recovery secret, revoked device, corrupted local storage, cloud outage, and relay-only limitations. Support must never ask users to send vault keys or recovery phrases.

## 14. Phased implementation plan

### Phase 0 — Specify before building

- [ ] Write the versioned protocol specification: canonical serialization; exact envelope/AAD/signature fields; origin-device nonce-domain/counter byte format, persistence, exhaustion limits; HLC algorithm; owner-signed genesis/manifest chain and epoch-cutover validation; cloud cursors; relay range inventories; digest-bound receipts; signed snapshot manifests/retention boundaries; recovery-package format; limits; and errors.
- [ ] Select maintained platform crypto APIs/libraries and secure-storage adapters; benchmark and approve Argon2id parameters for supported devices.
- [ ] Create shared test vectors, threat-model review, data-retention policy, and API contract. Resolve account/enrollment authentication design before server implementation, explicitly separating service authentication from device-signature authorization and documenting old-epoch offline-mutation handling.

### Phase 1 — Local vault, crypto, and merge core

- [ ] Implement encrypted local vault storage, secure key handling, random vault-key generation, the user-held Argon2id/AEAD recovery package, epoch/purpose/origin HKDF subkeys, persisted nonce counters, and signed versioned AES-GCM envelopes.
- [ ] Implement immutable mutation log, HLC persistence, operation-ID/digest dedupe integrity, transactional materialized view, tombstones/restores, manifest/snapshot verification, per-origin range inventory, outbox, snapshots, and full-resync primitives.
- [ ] Implement protocol/merge/crypto vectors and fault/property tests independently on Windows and Android. Do not begin polished UI or service work until cross-client vectors and convergence tests pass.

### Phase 2 — Functional native task experience

- [ ] Build minimal native Windows and Android vault unlock/create flows, matrix view, task editor, completed/deleted views, local backup/export, and clear local-only status.
- [ ] Integrate lifecycle-safe local persistence and user-visible corruption/recovery paths.

### Phase 3 — Enrollment and opaque cloud delivery

- [ ] Implement account/session boundaries, separate device signing/encryption identities, QR enrollment, user-held recovery-package creation/restore, owner-signed membership publication, per-recipient epoch-material delivery, revocation/cutover UX, and key rotation.
- [ ] Implement the opaque append-only cloud store, manifest/signature authorization, idempotent append/read pagination with digest-conflict rejection and bound receipts, cursor expiry, snapshot endpoints, quotas, authentication, retention, and operational controls.
- [ ] Integrate cloud adapter with durable outbox, bootstrap, acknowledgement-after-commit, retry/backoff, compaction, repair/full-resync, and transport status UI.

### Phase 4 — Volatile relay and transport safety

- [ ] Implement WSS capability authentication, bounded peer routing, relay reconciliation, reconnection, backpressure, abuse controls, and no-persistence guarantees.
- [ ] Implement safe cloud/relay enable-disable/switch semantics and test dual-transport dedupe/convergence.

### Phase 5 — Hardening and release

- [ ] Complete end-to-end device, lifecycle, migration, load, fuzz, security, and disaster-recovery testing.
- [ ] Conduct independent crypto/protocol review, accessibility review, privacy review, staged deployment, monitoring/runbook validation, and release rollback rehearsal.

## 15. Corner-case release checklist

- [ ] Concurrent edits to the same field and independent edits to different fields converge deterministically.
- [ ] Clock skew, clock rollback, and restart preserve HLC monotonicity and produce understandable diagnostics.
- [ ] Delete versus update, delete versus delete, restore versus delete, and edits from stale devices follow tombstone rules.
- [ ] Duplicated, dropped, reordered, replayed, malformed, oversized, and authentication-failed frames cannot corrupt or double-apply state.
- [ ] Every accepted envelope has a valid origin signature and applicable owner-signed membership manifest; service credentials alone cannot authorize it, and manifest forks/unauthorized changes fail closed.
- [ ] A repeated operation ID has identical canonical envelope bytes/digest; a conflicting digest is rejected by cloud, relay, and client, and receipts bind the accepted digest.
- [ ] Persisted per-device nonce counters survive restart/rollback without reuse, enforce exhaustion, and are isolated by epoch, purpose, and origin device.
- [ ] An offline device returning after compaction/cursor expiry bootstraps safely without silently losing local mutations.
- [ ] Snapshot bootstrap verifies source authorization, epoch/membership/schema, state and tombstone commitments, and per-origin coverage; no covered history is pruned without a recoverable compatible snapshot.
- [ ] Relay anti-entropy range inventories reconcile missing per-origin sequences; cloud cursors are not treated as peer inventories.
- [ ] Cloud/relay enable, disable, interruption, and mode switching are non-destructive; cursors and receipts stay transport-specific.
- [ ] Database corruption, partial writes, failed migrations, secure-storage loss, and unavailable recovery material do not cause an empty vault to overwrite recoverable data.
- [ ] Server and relay limits (payload, batch, peers, queues, rates, quota, retention) fail explicitly and leave the durable outbox recoverable.
- [ ] Windows suspension and Android background restrictions/process death leave a consistent local transaction boundary and reconnect through reconciliation.
- [ ] User-held recovery packages validate KDF-version/limits and AEAD before restore, restore trust/key state without server passphrase reset, and clearly identify the required locator and credential.
- [ ] Lost, replaced, and revoked devices have clear UX; revocation is described as advisory until rotation, cutover rejects previously unseen old-epoch mutations, and per-recipient new-epoch delivery is verified.
- [ ] Cloud status says only “caught up through last observed cloud cursor” and relay status only “reconciled with currently connected peers,” with withholding/unavailable-peer caveats.
