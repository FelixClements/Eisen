//! Encrypted local store schema and wrapping (P1.09).
//!
//! Defines the encrypted storage layout for vault records, the immutable mutation
//! log, the materialized view, WAL/journal, and non-secret metadata, per ADR-007
//! and ADR-008. Also implements the local mutation transaction (P1.10): nonce
//! reservation, HLC advancement, encrypted immutable log insert, operation-ID
//! digest integrity, materialized-view update, and outbox enqueue.

use crate::canonical::{self, Limits};
use crate::clock::{Clock, ClockError, ClockStorage};
use crate::envelope::{hlc_to_value, mutation_to_value};
use crate::epoch::{EpochError, EpochKey, SnapshotStore};
use crate::export::{ExportError, VaultExport};
use crate::identity::SecureStorage;
use crate::identity::{OwnerTrust, VaultId};
use crate::manifest::ManifestChain;
use crate::snapshot::{Snapshot, SnapshotError, SnapshotManager};
use crate::snapshot_manifest::{task_store_commitments, SignedSnapshot, SnapshotManifest};
use crate::{DeviceId, Hlc, Mutation, TaskStore};
use cbor2::Value;
use ed25519_dalek::SigningKey;
use sha2::{Digest, Sha256};

/// Errors returned by the local store.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StoreError {
    Encode(String),
    Decode(String),
    Crypto(EpochError),
    Clock(ClockError),
    Snapshot(SnapshotError),
    Model(crate::ModelError),
    CounterUncertain,
    CounterExhausted,
    MissingSnapshot,
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreError::Encode(s) => write!(f, "store encode error: {s}"),
            StoreError::Decode(s) => write!(f, "store decode error: {s}"),
            StoreError::Crypto(e) => write!(f, "store crypto error: {e}"),
            StoreError::Clock(e) => write!(f, "store clock error: {e}"),
            StoreError::Snapshot(e) => write!(f, "store snapshot error: {e}"),
            StoreError::Model(e) => write!(f, "store model error: {e}"),
            StoreError::CounterUncertain => {
                write!(f, "nonce counter is uncertain; repair required")
            }
            StoreError::CounterExhausted => write!(f, "nonce counter exhausted"),
            StoreError::MissingSnapshot => write!(f, "no snapshot found in local store"),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<EpochError> for StoreError {
    fn from(e: EpochError) -> Self {
        StoreError::Crypto(e)
    }
}

impl From<ClockError> for StoreError {
    fn from(e: ClockError) -> Self {
        StoreError::Clock(e)
    }
}

impl From<SnapshotError> for StoreError {
    fn from(e: SnapshotError) -> Self {
        StoreError::Snapshot(e)
    }
}

impl From<crate::canonical::CanonError> for StoreError {
    fn from(e: crate::canonical::CanonError) -> Self {
        StoreError::Decode(e.to_string())
    }
}

impl From<crate::ModelError> for StoreError {
    fn from(e: crate::ModelError) -> Self {
        StoreError::Model(e)
    }
}

impl From<ExportError> for StoreError {
    fn from(e: ExportError) -> Self {
        match e {
            ExportError::Encode(s) | ExportError::Integrity(s) | ExportError::Entropy(s) => {
                StoreError::Encode(s)
            }
            ExportError::Decode(s) => StoreError::Decode(s),
            ExportError::Crypto(s) => StoreError::Crypto(EpochError::Encryption(s)),
            ExportError::Epoch(e) => StoreError::Crypto(e),
            ExportError::Snapshot(e) => StoreError::Snapshot(e),
            ExportError::Identity(e) => StoreError::Encode(e.to_string()),
            ExportError::Version(v) => {
                StoreError::Decode(format!("unsupported export version {v}"))
            }
        }
    }
}

/// A 32-byte operation identifier: SHA-256 digest over the canonical HLC and
/// mutation.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OperationId(pub [u8; 32]);

impl OperationId {
    /// Hex encoding for diagnostics.
    pub fn to_hex(&self) -> String {
        self.0.iter().map(|b| format!("{b:02x}")).collect()
    }
}

impl std::fmt::Display for OperationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

/// Non-secret operational metadata stored encrypted at rest.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StoreMetadata {
    pub operation_count: u64,
    pub last_nonce: u64,
    pub last_wall: u64,
    pub last_counter: u32,
    /// Immutable log of operation IDs applied to this store.
    pub applied: Vec<OperationId>,
    /// Per-origin sequence coverage (origin, start_seq, end_seq).
    pub coverage: Vec<CoverageRange>,
    /// Highest committed sequence per origin (used to assign local seqs).
    pub seq: std::collections::BTreeMap<DeviceId, u64>,
}

impl StoreMetadata {
    /// Serialize to canonical CBOR bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>, StoreError> {
        metadata_to_bytes(self)
    }

    /// Parse from canonical CBOR bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, StoreError> {
        metadata_from_bytes(bytes)
    }
}

/// Per-origin contiguous sequence coverage.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoverageRange {
    pub origin: DeviceId,
    pub start: u64,
    pub end: u64,
}

/// A single WAL/journal entry recording a not-yet-committed local mutation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WalEntry {
    pub op_id: OperationId,
    pub hlc: Hlc,
    pub nonce: u64,
    pub mutation: Mutation,
}

/// Local outbox entry: a mutation ready for cloud/relay upload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutboxEntry {
    pub op_id: OperationId,
    pub hlc: Hlc,
}

/// Encrypted local vault store.
///
/// Owns the materialized view, immutable mutation log, WAL, metadata, clock,
/// and nonce counter. All secret-bearing records are encrypted under
/// purpose-derived sub-keys of the epoch key.
pub struct LocalStore<'a> {
    snapshot_manager: SnapshotManager<'a>,
    clock: Clock<EncryptedClockStorage<'a>>,
    nonce_counter: NonceCounter<'a>,
    metadata_store: MetadataStore<'a>,
    wal_store: WalStore<'a>,
    outbox_store: OutboxStore<'a>,
    metadata: StoreMetadata,
    wal: Vec<WalEntry>,
    outbox: Vec<OutboxEntry>,
    device_id: DeviceId,
    epoch_key: EpochKey,
    applied: std::collections::HashSet<OperationId>,
}

impl<'a> LocalStore<'a> {
    /// Open an existing local store or create a fresh one if none exists.
    pub fn open(
        storage: &'a dyn SecureStorage,
        epoch_key: EpochKey,
        device_id: DeviceId,
    ) -> Result<Self, StoreError> {
        let snapshot_key = epoch_key.derive_purpose("snapshot")?;
        let snapshot_store = SnapshotStore::new(storage, snapshot_key);

        let snapshot_manager = match SnapshotManager::load(snapshot_store) {
            Ok(sm) => sm,
            Err(SnapshotError::Crypto(EpochError::Storage(_))) | Err(SnapshotError::Decode(_)) => {
                SnapshotManager::new(SnapshotStore::new(
                    storage,
                    epoch_key.derive_purpose("snapshot")?,
                ))
            }
            Err(e) => return Err(e.into()),
        };

        let clock_storage = EncryptedClockStorage::new(storage, epoch_key.clone())?;
        let clock = Clock::new(clock_storage, device_id, 0)?;

        let nonce_counter = NonceCounter::new(storage, epoch_key.clone())?;
        let metadata_store = MetadataStore::new(storage, epoch_key.clone())?;
        let wal_store = WalStore::new(storage, epoch_key.clone())?;
        let outbox_store = OutboxStore::new(storage, epoch_key)?;

        let metadata = metadata_store.load()?;
        let wal = wal_store.load()?;
        let outbox = outbox_store.load()?;

        let applied: std::collections::HashSet<OperationId> =
            metadata.applied.iter().copied().collect();
        let mut store = Self {
            snapshot_manager,
            clock,
            nonce_counter,
            metadata_store,
            wal_store,
            outbox_store,
            metadata,
            wal,
            outbox,
            device_id,
            epoch_key,
            applied,
        };

        // Recover any pending WAL entries from an interrupted transaction.
        store.replay_wal()?;
        Ok(store)
    }

    /// Commit a local mutation in the required durable order.
    ///
    /// 1. Reserve a monotonic nonce.
    /// 2. Advance the HLC.
    /// 3. Compute the operation ID.
    /// 4. Write a WAL entry.
    /// 5. Append to the immutable log and update the materialized view.
    /// 6. Persist the snapshot.
    /// 7. Clear the WAL.
    /// 8. Enqueue the operation in the local outbox.
    /// 9. Persist metadata.
    pub fn commit(&mut self, physical: u64, mutation: Mutation) -> Result<OperationId, StoreError> {
        if self.nonce_counter.is_uncertain() {
            return Err(StoreError::CounterUncertain);
        }

        let nonce = self.nonce_counter.next()?;
        let hlc = self.clock.now(physical)?;
        let op_id = operation_id(&hlc, &mutation)?;

        let wal_entry = WalEntry {
            op_id,
            hlc,
            nonce,
            mutation: mutation.clone(),
        };

        // Durably record intent before mutating the main state.
        self.wal.push(wal_entry);
        self.wal_store.store(&self.wal)?;

        // Apply to the materialized view and immutable log.
        self.snapshot_manager.apply(mutation)?;
        self.snapshot_manager.save()?;

        // Transaction is durable; clear WAL.
        self.wal.clear();
        self.wal_store.clear()?;

        // Update outbox and metadata.
        self.outbox.push(OutboxEntry { op_id, hlc });
        self.outbox_store.store(&self.outbox)?;

        self.metadata.operation_count = self.metadata.operation_count.saturating_add(1);
        self.metadata.last_nonce = nonce;
        self.metadata.last_wall = hlc.wall;
        self.metadata.last_counter = hlc.counter;

        let local_seq = self.metadata.seq.get(&self.device_id).copied().unwrap_or(0) + 1;
        self.metadata.seq.insert(self.device_id, local_seq);
        self.update_coverage(self.device_id, local_seq);
        self.metadata.applied.push(op_id);
        self.applied.insert(op_id);
        self.metadata_store.store(&self.metadata)?;

        Ok(op_id)
    }

    /// Apply a verified external envelope to the local materialized view.
    ///
    /// The caller supplies the origin's `seq` for this operation so the store
    /// can maintain per-origin coverage. Duplicate operation IDs are ignored
    /// (idempotent). If the sequence number conflicts with an already-known
    /// different operation, the mutation is quarantined rather than applied.
    pub fn apply(
        &mut self,
        envelope: crate::envelope::Envelope,
        owner_trust: &crate::identity::OwnerTrust,
        origin_seq: u64,
    ) -> Result<OperationId, StoreError> {
        let mutation = envelope
            .verify(owner_trust)
            .map_err(|e| StoreError::Encode(e.to_string()))?;
        let hlc = envelope.hlc;
        let origin = envelope.device_id;
        let op_id = operation_id(&hlc, &mutation)?;

        if self.applied.contains(&op_id) {
            return Ok(op_id);
        }

        // Sequence conflict detection: if this seq is already covered for this
        // origin and maps to a different op_id, quarantine it.
        if let Some(entry) = self.metadata.coverage.iter().find(|c| c.origin == origin) {
            if origin_seq <= entry.end && origin_seq >= entry.start {
                // Already covered range; if we reached here the op_id is new,
                // which means a different operation reused a sequence number.
                return Err(StoreError::Encode(
                    "sequence conflict detected; operation quarantined".into(),
                ));
            }
        }

        self.snapshot_manager.apply(mutation)?;
        self.snapshot_manager.save()?;

        self.update_coverage(origin, origin_seq);
        self.metadata.seq.insert(
            origin,
            origin_seq.max(self.metadata.seq.get(&origin).copied().unwrap_or(0)),
        );
        self.metadata.applied.push(op_id);
        self.applied.insert(op_id);
        self.metadata.operation_count = self.metadata.operation_count.saturating_add(1);
        self.metadata_store.store(&self.metadata)?;

        Ok(op_id)
    }

    /// Borrow the per-origin coverage ranges.
    pub fn coverage(&self) -> &[CoverageRange] {
        &self.metadata.coverage
    }

    fn update_coverage(&mut self, origin: DeviceId, seq: u64) {
        let mut merged = false;
        for entry in self.metadata.coverage.iter_mut() {
            if entry.origin == origin {
                if seq == entry.end + 1 {
                    entry.end = seq;
                } else if seq + 1 == entry.start {
                    entry.start = seq;
                } else if seq < entry.start || seq > entry.end {
                    // Non-contiguous; start a new range (will be added below).
                    continue;
                }
                merged = true;
                break;
            }
        }
        if !merged {
            self.metadata.coverage.push(CoverageRange {
                origin,
                start: seq,
                end: seq,
            });
        }
    }

    /// Borrow the materialized task store.
    pub fn store(&self) -> &TaskStore {
        self.snapshot_manager.store()
    }

    /// Borrow the local outbox.
    pub fn outbox(&self) -> &[OutboxEntry] {
        &self.outbox
    }

    /// Current store metadata.
    pub fn metadata(&self) -> &StoreMetadata {
        &self.metadata
    }

    /// Create a signed snapshot of the current local state.
    pub fn create_signed_snapshot(
        &mut self,
        owner_signing_key: &SigningKey,
        vault_id: VaultId,
    ) -> Result<SignedSnapshot, StoreError> {
        let snapshot =
            Snapshot::from_store(self.snapshot_manager.store(), self.snapshot_manager.log());
        let payload_bytes = snapshot.to_bytes()?;
        let payload_digest = OperationId(Sha256::digest(&payload_bytes).into());

        let (state_root, tombstone_root) = task_store_commitments(self.snapshot_manager.store());
        let checkpoint = Hlc {
            wall: self.metadata.last_wall,
            counter: self.metadata.last_counter,
            device_id: self.device_id,
        };

        let manifest = SnapshotManifest {
            protocol_version: 1,
            snapshot_version: 1,
            vault_id,
            checkpoint,
            coverage: self.metadata.coverage.clone(),
            state_root,
            tombstone_root,
            payload_digest,
        };

        let aead_snapshot = self.snapshot_manager.save()?;
        let signature = manifest.sign(owner_signing_key);

        Ok(SignedSnapshot {
            manifest,
            payload: aead_snapshot,
            signature,
        })
    }

    /// Install a verified signed snapshot, preserving local data not covered by
    /// the snapshot (no-silent-local-replacement).
    pub fn install_signed_snapshot(
        &mut self,
        signed: &SignedSnapshot,
        owner_trust: &OwnerTrust,
    ) -> Result<(), StoreError> {
        let snapshot = self.verify_and_decrypt_snapshot(signed, owner_trust)?;

        if !coverage_is_superset(&signed.manifest.coverage, &self.metadata.coverage) {
            return Err(StoreError::Decode(
                "snapshot does not cover current local state".into(),
            ));
        }

        self.install_snapshot(snapshot, signed.manifest.coverage.clone())?;
        self.metadata.last_wall = signed.manifest.checkpoint.wall;
        self.metadata.last_counter = signed.manifest.checkpoint.counter;
        self.metadata_store.store(&self.metadata)?;
        Ok(())
    }

    /// Repair local state from a verified snapshot and a list of subsequent
    /// operations. Local operations not covered by the snapshot are preserved
    /// and reconciled with the replayed subsequent operations.
    pub fn repair(
        &mut self,
        signed: &SignedSnapshot,
        owner_trust: &OwnerTrust,
        subsequent: &[(crate::envelope::Envelope, u64)],
    ) -> Result<(), StoreError> {
        let snapshot = self.verify_and_decrypt_snapshot(signed, owner_trust)?;

        // Preserve the local log, sequence counters, and coverage before replacing state.
        let local_log: Vec<Mutation> = self.snapshot_manager.log().to_vec();
        let saved_seq = self.metadata.seq.clone();
        let saved_coverage = self.metadata.coverage.clone();

        self.install_snapshot(snapshot, signed.manifest.coverage.clone())?;
        self.metadata.last_wall = signed.manifest.checkpoint.wall;
        self.metadata.last_counter = signed.manifest.checkpoint.counter;

        // Re-apply local mutations not already covered by the snapshot first,
        // so that later operations can reference them.
        let current = Hlc {
            wall: self.metadata.last_wall,
            counter: self.metadata.last_counter,
            device_id: self.device_id,
        };
        for mutation in local_log {
            let hlc = mutation.hlc().unwrap_or(current);
            let op_id = operation_id(&hlc, &mutation)?;
            if !self.applied.contains(&op_id) {
                self.snapshot_manager.apply(mutation)?;
                self.applied.insert(op_id);
                self.metadata.applied.push(op_id);
                self.metadata.operation_count = self.metadata.operation_count.saturating_add(1);
                if hlc.wall > self.metadata.last_wall
                    || (hlc.wall == self.metadata.last_wall
                        && hlc.counter > self.metadata.last_counter)
                {
                    self.metadata.last_wall = hlc.wall;
                    self.metadata.last_counter = hlc.counter;
                }
            }
        }

        // Replay subsequent operations in the order provided.
        for (envelope, seq) in subsequent {
            self.apply(envelope.clone(), owner_trust, *seq)?;
        }

        // Restore per-origin high-sequence counters and merge saved coverage ranges.
        for (origin, seq) in saved_seq {
            let high = seq.max(self.metadata.seq.get(&origin).copied().unwrap_or(0));
            self.metadata.seq.insert(origin, high);
        }
        for saved in saved_coverage {
            if let Some(entry) = self
                .metadata
                .coverage
                .iter_mut()
                .find(|c| c.origin == saved.origin)
            {
                entry.start = entry.start.min(saved.start);
                entry.end = entry.end.max(saved.end);
            } else {
                self.metadata.coverage.push(saved.clone());
            }
        }

        self.metadata_store.store(&self.metadata)?;
        self.snapshot_manager.save()?;
        Ok(())
    }

    /// Export the current local store state as an encrypted `VaultExport`.
    ///
    /// The caller supplies the owner trust anchor and manifest chain so the
    /// export is bound to the correct vault. The payload is encrypted under an
    /// export-purpose sub-key derived from the current epoch key.
    pub fn export(
        &self,
        owner_trust: &OwnerTrust,
        manifest_chain: &ManifestChain,
    ) -> Result<VaultExport, StoreError> {
        if manifest_chain.current().content.vault_id != owner_trust.vault_id {
            return Err(StoreError::Decode(
                "manifest chain vault id mismatch".into(),
            ));
        }
        let snapshot = Snapshot::from_store(self.store(), self.snapshot_manager.log());
        VaultExport::new(
            &self.epoch_key,
            owner_trust.vault_id,
            &snapshot,
            &self.metadata,
            &self.outbox,
            &manifest_chain.iter().cloned().collect::<Vec<_>>(),
            self.metadata.last_wall,
        )
        .map_err(|e: ExportError| StoreError::from(e))
    }

    /// Import an encrypted `VaultExport` into this store.
    ///
    /// The epoch key must match the export's key epoch. The manifest chain in
    /// the export is verified against the owner trust anchor before the snapshot,
    /// metadata, and outbox are installed. Local state not covered by the export
    /// is replaced; the no-silent-local-replacement rule is enforced because the
    /// export contains a complete snapshot.
    pub fn import(
        &mut self,
        export: &VaultExport,
        owner_trust: &OwnerTrust,
    ) -> Result<(), StoreError> {
        if export.vault_id != owner_trust.vault_id {
            return Err(StoreError::Decode("export vault id mismatch".into()));
        }
        let payload = export
            .decrypt(&self.epoch_key)
            .map_err(|e: ExportError| StoreError::from(e))?;
        if payload.vault_id != owner_trust.vault_id || payload.key_epoch != self.epoch_key.epoch {
            return Err(StoreError::Decode(
                "export payload identity mismatch".into(),
            ));
        }

        if payload.manifests.is_empty() {
            return Err(StoreError::Decode("export contains no manifests".into()));
        }
        let mut chain = ManifestChain::new(payload.manifests[0].clone())
            .map_err(|e| StoreError::Encode(e.to_string()))?;
        for m in &payload.manifests[1..] {
            chain
                .push(m.clone())
                .map_err(|e| StoreError::Encode(e.to_string()))?;
        }
        if chain.current().content.vault_id != owner_trust.vault_id {
            return Err(StoreError::Decode("manifest vault id mismatch".into()));
        }

        self.snapshot_manager
            .restore_from_snapshot(payload.snapshot)?;

        self.metadata = payload.metadata;
        self.applied = self.metadata.applied.iter().copied().collect();
        self.outbox = payload.outbox;
        self.wal.clear();

        self.metadata_store.store(&self.metadata)?;
        self.outbox_store.store(&self.outbox)?;
        self.wal_store.clear()?;
        self.nonce_counter.set_at_least(self.metadata.last_nonce)?;

        let last = Hlc {
            wall: self.metadata.last_wall,
            counter: self.metadata.last_counter,
            device_id: self.device_id,
        };
        self.clock
            .receive(self.metadata.last_wall, last)
            .map_err(StoreError::Clock)?;

        Ok(())
    }

    fn verify_and_decrypt_snapshot(
        &self,
        signed: &SignedSnapshot,
        owner_trust: &OwnerTrust,
    ) -> Result<Snapshot, StoreError> {
        signed
            .verify_signature(owner_trust)
            .map_err(|e| StoreError::Encode(e.to_string()))?;

        if signed.manifest.vault_id != owner_trust.vault_id {
            return Err(StoreError::Decode("snapshot vault id mismatch".into()));
        }
        if signed.manifest.protocol_version != 1 {
            return Err(StoreError::Decode(
                "unsupported snapshot protocol version".into(),
            ));
        }

        let snapshot_key = self.epoch_key.derive_purpose("snapshot")?;
        let payload_bytes = signed
            .payload
            .decrypt(&snapshot_key)
            .map_err(|e| StoreError::Crypto(e))?;
        let payload_digest = OperationId(Sha256::digest(&payload_bytes).into());
        if payload_digest != signed.manifest.payload_digest {
            return Err(StoreError::Decode(
                "snapshot payload digest mismatch".into(),
            ));
        }

        let snapshot = Snapshot::from_bytes(&payload_bytes)?;
        let replayed = snapshot.replay()?;
        let (state_root, tombstone_root) = task_store_commitments(&replayed);
        if state_root != signed.manifest.state_root {
            return Err(StoreError::Decode("snapshot state root mismatch".into()));
        }
        if tombstone_root != signed.manifest.tombstone_root {
            return Err(StoreError::Decode(
                "snapshot tombstone root mismatch".into(),
            ));
        }
        Ok(snapshot)
    }

    fn install_snapshot(
        &mut self,
        snapshot: Snapshot,
        coverage: Vec<CoverageRange>,
    ) -> Result<(), StoreError> {
        let current = Hlc {
            wall: self.metadata.last_wall,
            counter: self.metadata.last_counter,
            device_id: self.device_id,
        };
        self.snapshot_manager.restore_from_snapshot(snapshot)?;
        self.metadata.coverage = coverage;
        self.applied.clear();
        self.metadata.applied.clear();
        for mutation in self.snapshot_manager.log() {
            let hlc = mutation.hlc().unwrap_or(current);
            let op_id = operation_id(&hlc, mutation)?;
            self.applied.insert(op_id);
            self.metadata.applied.push(op_id);
        }
        Ok(())
    }

    /// Replay any pending WAL entries over the current snapshot state.
    fn replay_wal(&mut self) -> Result<(), StoreError> {
        if self.wal.is_empty() {
            return Ok(());
        }

        let log: Vec<Mutation> = self.snapshot_manager.log().to_vec();
        let mut changed = false;
        for entry in self.wal.drain(..) {
            if !log.contains(&entry.mutation) {
                self.snapshot_manager.apply(entry.mutation)?;
                changed = true;
            }
        }

        if changed {
            self.snapshot_manager.save()?;
        }
        self.wal_store.clear()?;
        Ok(())
    }
}

fn operation_id(hlc: &Hlc, mutation: &Mutation) -> Result<OperationId, StoreError> {
    let value = Value::Array(vec![
        hlc_to_value(hlc),
        mutation_to_value(mutation).map_err(|e| StoreError::Encode(e.to_string()))?,
    ]);
    let bytes = canonical::encode(&value)?;
    let digest = Sha256::digest(&bytes);
    Ok(OperationId(digest.into()))
}

fn coverage_is_superset(new: &[CoverageRange], old: &[CoverageRange]) -> bool {
    for old_range in old {
        let covered = new.iter().any(|new_range| {
            new_range.origin == old_range.origin
                && new_range.start <= old_range.start
                && new_range.end >= old_range.end
        });
        if !covered {
            return false;
        }
    }
    true
}

/// Encrypted clock storage backed by a `SnapshotStore`.
struct EncryptedClockStorage<'a> {
    store: SnapshotStore<'a>,
}

impl<'a> EncryptedClockStorage<'a> {
    fn new(storage: &'a dyn SecureStorage, epoch_key: EpochKey) -> Result<Self, EpochError> {
        let key = epoch_key.derive_purpose("clock")?;
        let store = SnapshotStore::with_keys(
            storage,
            key,
            "clock:counter".into(),
            "clock:ciphertext".into(),
        );
        Ok(Self { store })
    }
}

impl<'a> ClockStorage for EncryptedClockStorage<'a> {
    fn load(&self) -> Result<Option<(u64, u32)>, ClockError> {
        match self.store.load() {
            Ok(bytes) => {
                let value = canonical::parse(&bytes, &Limits::default())
                    .map_err(|e| ClockError::Storage(e.to_string()))?;
                match value {
                    Value::Array(arr) if arr.len() == 2 => {
                        let wall = u64_from_value(&arr[0])?;
                        let counter = u64_from_value(&arr[1])? as u32;
                        Ok(Some((wall, counter)))
                    }
                    _ => Err(ClockError::Storage("invalid clock state".into())),
                }
            }
            Err(EpochError::Storage(_)) => Ok(None),
            Err(e) => Err(ClockError::Storage(e.to_string())),
        }
    }

    fn save(&self, wall: u64, counter: u32) -> Result<(), ClockError> {
        let value = Value::Array(vec![
            Value::Integer(wall.into()),
            Value::Integer(counter.into()),
        ]);
        let bytes = canonical::encode(&value).map_err(|e| ClockError::Storage(e.to_string()))?;
        self.store
            .store(&bytes)
            .map_err(|e| ClockError::Storage(e.to_string()))?;
        Ok(())
    }
}

fn u64_from_value(v: &Value) -> Result<u64, ClockError> {
    match v {
        Value::Integer(i) => {
            let n: i128 = (*i).into();
            if n < 0 || n > u64::MAX as i128 {
                return Err(ClockError::Storage("integer out of u64 range".into()));
            }
            Ok(n as u64)
        }
        _ => Err(ClockError::Storage("expected integer".into())),
    }
}

/// Monotonic nonce counter with fail-closed uncertainty tracking.
struct NonceCounter<'a> {
    store: SnapshotStore<'a>,
    uncertain: bool,
}

impl<'a> NonceCounter<'a> {
    fn new(storage: &'a dyn SecureStorage, epoch_key: EpochKey) -> Result<Self, EpochError> {
        let key = epoch_key.derive_purpose("nonce")?;
        let store = SnapshotStore::with_keys(
            storage,
            key,
            "nonce:counter".into(),
            "nonce:ciphertext".into(),
        );
        Ok(Self {
            store,
            uncertain: false,
        })
    }

    fn is_uncertain(&self) -> bool {
        self.uncertain
    }

    fn current(&mut self) -> Result<u64, StoreError> {
        if self.uncertain {
            return Err(StoreError::CounterUncertain);
        }
        match self.store.load() {
            Ok(bytes) => {
                let value = canonical::parse(&bytes, &Limits::default())?;
                match value {
                    Value::Integer(i) => {
                        let n: i128 = i.into();
                        if n < 0 || n > u64::MAX as i128 {
                            self.uncertain = true;
                            return Err(StoreError::Decode("nonce counter out of range".into()));
                        }
                        Ok(n as u64)
                    }
                    _ => {
                        self.uncertain = true;
                        Err(StoreError::Decode("invalid nonce counter".into()))
                    }
                }
            }
            Err(EpochError::Storage(_)) => Ok(0),
            Err(e) => {
                self.uncertain = true;
                Err(e.into())
            }
        }
    }

    fn next(&mut self) -> Result<u64, StoreError> {
        let current = self.current()?;
        let next = current.checked_add(1).ok_or(StoreError::CounterExhausted)?;
        let value = Value::Integer(next.into());
        let bytes = canonical::encode(&value)?;
        self.store.store(&bytes)?;
        Ok(next)
    }

    /// Persist a counter value that is at least the supplied minimum.
    fn set_at_least(&mut self, minimum: u64) -> Result<(), StoreError> {
        let current = self.current()?;
        let target = minimum.max(current);
        let value = Value::Integer(target.into());
        let bytes = canonical::encode(&value)?;
        self.store.store(&bytes)?;
        Ok(())
    }
}

/// Encrypted metadata store.
struct MetadataStore<'a> {
    store: SnapshotStore<'a>,
}

impl<'a> MetadataStore<'a> {
    fn new(storage: &'a dyn SecureStorage, epoch_key: EpochKey) -> Result<Self, EpochError> {
        let key = epoch_key.derive_purpose("metadata")?;
        let store = SnapshotStore::with_keys(
            storage,
            key,
            "metadata:counter".into(),
            "metadata:ciphertext".into(),
        );
        Ok(Self { store })
    }

    fn load(&self) -> Result<StoreMetadata, StoreError> {
        match self.store.load() {
            Ok(bytes) => metadata_from_bytes(&bytes),
            Err(EpochError::Storage(_)) => Ok(StoreMetadata::default()),
            Err(e) => Err(e.into()),
        }
    }

    fn store(&self, metadata: &StoreMetadata) -> Result<(), StoreError> {
        let bytes = metadata_to_bytes(metadata)?;
        self.store.store(&bytes)?;
        Ok(())
    }
}

fn metadata_to_bytes(metadata: &StoreMetadata) -> Result<Vec<u8>, StoreError> {
    let applied = Value::Array(
        metadata
            .applied
            .iter()
            .map(|id| Value::Bytes(id.0.to_vec()))
            .collect(),
    );
    let coverage = Value::Array(
        metadata
            .coverage
            .iter()
            .map(|c| {
                Value::Map(vec![
                    (text("origin"), Value::Bytes(c.origin.0.to_vec())),
                    (text("start"), Value::Integer(c.start.into())),
                    (text("end"), Value::Integer(c.end.into())),
                ])
            })
            .collect(),
    );
    let seq = Value::Map(
        metadata
            .seq
            .iter()
            .map(|(device, s)| (Value::Bytes(device.0.to_vec()), Value::Integer((*s).into())))
            .collect(),
    );
    let value = Value::Map(vec![
        (
            text("operation_count"),
            Value::Integer(metadata.operation_count.into()),
        ),
        (
            text("last_nonce"),
            Value::Integer(metadata.last_nonce.into()),
        ),
        (text("last_wall"), Value::Integer(metadata.last_wall.into())),
        (
            text("last_counter"),
            Value::Integer(metadata.last_counter.into()),
        ),
        (text("applied"), applied),
        (text("coverage"), coverage),
        (text("seq"), seq),
    ]);
    Ok(canonical::encode(&value)?)
}

fn metadata_from_bytes(bytes: &[u8]) -> Result<StoreMetadata, StoreError> {
    let value = canonical::parse(bytes, &Limits::default())?;
    match value {
        Value::Map(map) => Ok(StoreMetadata {
            operation_count: u64_field(&map, "operation_count")?,
            last_nonce: u64_field(&map, "last_nonce")?,
            last_wall: u64_field(&map, "last_wall")?,
            last_counter: u64_field(&map, "last_counter")? as u32,
            applied: applied_from_value(&get_field(&map, "applied")?)?,
            coverage: coverage_from_value(&get_field(&map, "coverage")?)?,
            seq: seq_from_value(&get_field(&map, "seq")?)?,
        }),
        _ => Err(StoreError::Decode("invalid metadata".into())),
    }
}

/// Encrypted WAL store.
struct WalStore<'a> {
    store: SnapshotStore<'a>,
}

impl<'a> WalStore<'a> {
    fn new(storage: &'a dyn SecureStorage, epoch_key: EpochKey) -> Result<Self, EpochError> {
        let key = epoch_key.derive_purpose("wal")?;
        let store =
            SnapshotStore::with_keys(storage, key, "wal:counter".into(), "wal:ciphertext".into());
        Ok(Self { store })
    }

    fn load(&self) -> Result<Vec<WalEntry>, StoreError> {
        match self.store.load() {
            Ok(bytes) => wal_from_bytes(&bytes),
            Err(EpochError::Storage(_)) => Ok(Vec::new()),
            Err(e) => Err(e.into()),
        }
    }

    fn store(&self, entries: &[WalEntry]) -> Result<(), StoreError> {
        let bytes = wal_to_bytes(entries)?;
        self.store.store(&bytes)?;
        Ok(())
    }

    fn clear(&self) -> Result<(), StoreError> {
        self.store(&[])
    }
}

fn wal_to_bytes(entries: &[WalEntry]) -> Result<Vec<u8>, StoreError> {
    let values: Result<Vec<_>, _> = entries.iter().map(wal_entry_to_value).collect();
    let value = Value::Array(values?);
    Ok(canonical::encode(&value)?)
}

fn wal_from_bytes(bytes: &[u8]) -> Result<Vec<WalEntry>, StoreError> {
    let value = canonical::parse(bytes, &Limits::default())?;
    match value {
        Value::Array(arr) => arr.iter().map(wal_entry_from_value).collect(),
        _ => Err(StoreError::Decode("invalid wal".into())),
    }
}

fn wal_entry_to_value(entry: &WalEntry) -> Result<Value, StoreError> {
    Ok(Value::Map(vec![
        (text("op_id"), Value::Bytes(entry.op_id.0.to_vec())),
        (text("hlc"), hlc_to_value(&entry.hlc)),
        (text("nonce"), Value::Integer(entry.nonce.into())),
        (
            text("mutation"),
            mutation_to_value(&entry.mutation).map_err(|e| StoreError::Encode(e.to_string()))?,
        ),
    ]))
}

fn wal_entry_from_value(v: &Value) -> Result<WalEntry, StoreError> {
    match v {
        Value::Map(map) => Ok(WalEntry {
            op_id: op_id_field(map, "op_id")?,
            hlc: hlc_from_value(&get_field(map, "hlc")?)?,
            nonce: u64_field(map, "nonce")?,
            mutation: mutation_from_value(&get_field(map, "mutation")?)?,
        }),
        _ => Err(StoreError::Decode("invalid wal entry".into())),
    }
}

/// Encrypted outbox store.
struct OutboxStore<'a> {
    store: SnapshotStore<'a>,
}

impl<'a> OutboxStore<'a> {
    fn new(storage: &'a dyn SecureStorage, epoch_key: EpochKey) -> Result<Self, EpochError> {
        let key = epoch_key.derive_purpose("outbox")?;
        let store = SnapshotStore::with_keys(
            storage,
            key,
            "outbox:counter".into(),
            "outbox:ciphertext".into(),
        );
        Ok(Self { store })
    }

    fn load(&self) -> Result<Vec<OutboxEntry>, StoreError> {
        match self.store.load() {
            Ok(bytes) => outbox_from_bytes(&bytes),
            Err(EpochError::Storage(_)) => Ok(Vec::new()),
            Err(e) => Err(e.into()),
        }
    }

    fn store(&self, entries: &[OutboxEntry]) -> Result<(), StoreError> {
        let bytes = outbox_to_bytes(entries)?;
        self.store.store(&bytes)?;
        Ok(())
    }
}

pub(crate) fn outbox_to_bytes(entries: &[OutboxEntry]) -> Result<Vec<u8>, StoreError> {
    let values: Vec<Value> = entries
        .iter()
        .map(|e| {
            Value::Map(vec![
                (text("op_id"), Value::Bytes(e.op_id.0.to_vec())),
                (text("hlc"), hlc_to_value(&e.hlc)),
            ])
        })
        .collect();
    Ok(canonical::encode(&Value::Array(values))?)
}

pub(crate) fn outbox_from_bytes(bytes: &[u8]) -> Result<Vec<OutboxEntry>, StoreError> {
    let value = canonical::parse(bytes, &Limits::default())?;
    match value {
        Value::Array(arr) => arr
            .iter()
            .map(|v| match v {
                Value::Map(map) => Ok(OutboxEntry {
                    op_id: op_id_field(map, "op_id")?,
                    hlc: hlc_from_value(&get_field(map, "hlc")?)?,
                }),
                _ => Err(StoreError::Decode("invalid outbox entry".into())),
            })
            .collect(),
        _ => Err(StoreError::Decode("invalid outbox".into())),
    }
}

fn text(s: &str) -> Value {
    Value::Text(s.into())
}

fn get_field(map: &[(Value, Value)], key: &str) -> Result<Value, StoreError> {
    map.iter()
        .find(|(k, _)| matches!(k, Value::Text(t) if t == key))
        .map(|(_, v)| v.clone())
        .ok_or_else(|| StoreError::Decode(format!("missing field: {key}")))
}

fn u64_field(map: &[(Value, Value)], key: &str) -> Result<u64, StoreError> {
    match get_field(map, key)? {
        Value::Integer(i) => {
            let n: i128 = i.into();
            if n < 0 || n > u64::MAX as i128 {
                return Err(StoreError::Decode(format!("{key} out of u64 range")));
            }
            Ok(n as u64)
        }
        _ => Err(StoreError::Decode(format!("{key} not an integer"))),
    }
}

fn applied_from_value(v: &Value) -> Result<Vec<OperationId>, StoreError> {
    match v {
        Value::Array(arr) => arr
            .iter()
            .map(|v| match v {
                Value::Bytes(b) if b.len() == 32 => {
                    let arr: [u8; 32] = b.clone().try_into().unwrap();
                    Ok(OperationId(arr))
                }
                _ => Err(StoreError::Decode("invalid applied op_id".into())),
            })
            .collect(),
        _ => Err(StoreError::Decode("applied must be an array".into())),
    }
}

fn coverage_from_value(v: &Value) -> Result<Vec<CoverageRange>, StoreError> {
    match v {
        Value::Array(arr) => arr
            .iter()
            .map(|v| match v {
                Value::Map(map) => Ok(CoverageRange {
                    origin: device_id_from_value(&get_field(map, "origin")?)?,
                    start: u64_field(map, "start")?,
                    end: u64_field(map, "end")?,
                }),
                _ => Err(StoreError::Decode("invalid coverage entry".into())),
            })
            .collect(),
        _ => Err(StoreError::Decode("coverage must be an array".into())),
    }
}

fn seq_from_value(v: &Value) -> Result<std::collections::BTreeMap<DeviceId, u64>, StoreError> {
    match v {
        Value::Map(map) => map
            .iter()
            .map(|(k, v)| {
                let device = device_id_from_value(k)?;
                let seq = match v {
                    Value::Integer(i) => {
                        let n: i128 = (*i).into();
                        if n < 0 || n > u64::MAX as i128 {
                            return Err(StoreError::Decode("seq out of u64 range".into()));
                        }
                        n as u64
                    }
                    _ => return Err(StoreError::Decode("seq value not an integer".into())),
                };
                Ok((device, seq))
            })
            .collect(),
        _ => Err(StoreError::Decode("seq must be a map".into())),
    }
}

fn device_id_from_value(v: &Value) -> Result<DeviceId, StoreError> {
    match v {
        Value::Bytes(b) if b.len() == 16 => {
            let arr: [u8; 16] = b.clone().try_into().unwrap();
            Ok(DeviceId(arr))
        }
        _ => Err(StoreError::Decode("invalid device id".into())),
    }
}

fn op_id_field(map: &[(Value, Value)], key: &str) -> Result<OperationId, StoreError> {
    match get_field(map, key)? {
        Value::Bytes(b) if b.len() == 32 => {
            let arr: [u8; 32] = b.try_into().unwrap();
            Ok(OperationId(arr))
        }
        _ => Err(StoreError::Decode(format!("{key} not a 32-byte op_id"))),
    }
}

fn hlc_from_value(v: &Value) -> Result<Hlc, StoreError> {
    match v {
        Value::Array(arr) if arr.len() == 3 => {
            let wall = u64_from_value_direct(&arr[0])?;
            let counter = u64_from_value_direct(&arr[1])? as u32;
            let device_id = match &arr[2] {
                Value::Bytes(b) if b.len() == 16 => {
                    let arr: [u8; 16] = b.clone().try_into().unwrap();
                    DeviceId(arr)
                }
                _ => return Err(StoreError::Decode("invalid device_id in hlc".into())),
            };
            Ok(Hlc {
                wall,
                counter,
                device_id,
            })
        }
        _ => Err(StoreError::Decode("invalid hlc".into())),
    }
}

fn u64_from_value_direct(v: &Value) -> Result<u64, StoreError> {
    match v {
        Value::Integer(i) => {
            let n: i128 = (*i).into();
            if n < 0 || n > u64::MAX as i128 {
                return Err(StoreError::Decode("integer out of u64 range".into()));
            }
            Ok(n as u64)
        }
        _ => Err(StoreError::Decode("expected integer".into())),
    }
}

fn mutation_from_value(v: &Value) -> Result<Mutation, StoreError> {
    crate::envelope::value_to_mutation(v).map_err(|e| StoreError::Decode(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::envelope::Envelope;
    use crate::identity::{create_vault, IdentityError, InMemorySecureStorage, SecureStorage};
    use crate::{Hlc, TaskId};
    use std::collections::{HashMap, HashSet};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    fn dev(n: u8) -> DeviceId {
        let mut bytes = [0u8; 16];
        bytes[15] = n;
        DeviceId(bytes)
    }

    fn make_hlc(wall: u64, counter: u32, device: u8) -> Hlc {
        Hlc {
            wall,
            counter,
            device_id: dev(device),
        }
    }

    #[derive(Default)]
    struct FaultConfig {
        fail_total: Option<usize>,
        fail_key_any: HashSet<String>,
        fail_key_on: HashMap<String, usize>,
        corrupt_keys: HashSet<String>,
        fail_load: HashSet<String>,
    }

    struct FaultyStorage {
        inner: InMemorySecureStorage,
        total: AtomicUsize,
        per_key: Mutex<HashMap<String, usize>>,
        config: Mutex<FaultConfig>,
    }

    impl FaultyStorage {
        fn new(inner: InMemorySecureStorage) -> Self {
            Self {
                inner,
                total: AtomicUsize::new(0),
                per_key: Mutex::new(HashMap::new()),
                config: Mutex::new(FaultConfig::default()),
            }
        }

        fn fail_key_on(&self, key: &str, n: usize) -> &Self {
            self.config
                .lock()
                .unwrap()
                .fail_key_on
                .insert(key.to_string(), n);
            self
        }

        fn corrupt_key(&self, key: &str) -> &Self {
            self.config
                .lock()
                .unwrap()
                .corrupt_keys
                .insert(key.to_string());
            self
        }

        fn fail_load_key(&self, key: &str) -> &Self {
            self.config
                .lock()
                .unwrap()
                .fail_load
                .insert(key.to_string());
            self
        }
    }

    impl SecureStorage for FaultyStorage {
        fn store(&self, key: &str, value: &[u8]) -> Result<(), IdentityError> {
            let total = self.total.fetch_add(1, Ordering::SeqCst) + 1;
            let mut per_key = self.per_key.lock().unwrap();
            let key_n = per_key.entry(key.to_string()).or_insert(0);
            *key_n += 1;
            let key_n = *key_n;
            drop(per_key);

            let cfg = self.config.lock().unwrap();
            if cfg.fail_total == Some(total)
                || cfg.fail_key_any.contains(key)
                || cfg.fail_key_on.get(key) == Some(&key_n)
            {
                return Err(IdentityError::Storage("injected store fault".into()));
            }

            let value = if cfg.corrupt_keys.contains(key) {
                let mut v = value.to_vec();
                for b in v.iter_mut() {
                    *b ^= 0xff;
                }
                v
            } else {
                value.to_vec()
            };
            drop(cfg);

            self.inner.store(key, &value)
        }

        fn load(&self, key: &str) -> Result<Option<Vec<u8>>, IdentityError> {
            let cfg = self.config.lock().unwrap();
            if cfg.fail_load.contains(key) {
                return Err(IdentityError::Storage("injected load fault".into()));
            }
            drop(cfg);
            self.inner.load(key)
        }

        fn delete(&self, key: &str) -> Result<(), IdentityError> {
            self.inner.delete(key)
        }
    }

    fn make_store<'a>(
        storage: &'a InMemorySecureStorage,
        key: EpochKey,
        device_id: DeviceId,
    ) -> LocalStore<'a> {
        LocalStore::open(storage, key, device_id).unwrap()
    }

    #[test]
    fn fresh_store_opens_and_commits() {
        let storage = InMemorySecureStorage::default();
        let key = crate::epoch::EpochRoot::generate()
            .unwrap()
            .derive(0)
            .unwrap();
        let device_id = dev(1);

        let mut store = make_store(&storage, key, device_id);

        let id = TaskId([1; 16]);
        let op_id = store
            .commit(
                1,
                Mutation::Create {
                    hlc: make_hlc(1, 0, 1),
                    id,
                    title: "Buy milk".into(),
                    notes: None,
                    quadrant: 0,
                    due_date: None,
                },
            )
            .unwrap();

        assert!(!op_id.to_hex().is_empty());
        assert_eq!(store.store().len(), 1);
        assert_eq!(store.metadata().operation_count, 1);
        assert_eq!(store.outbox().len(), 1);
    }

    #[test]
    fn store_reloads_and_replays_wal() {
        let storage = InMemorySecureStorage::default();
        let key = crate::epoch::EpochRoot::generate()
            .unwrap()
            .derive(0)
            .unwrap();
        let device_id = dev(1);

        // Commit and drop store.
        {
            let mut store = make_store(&storage, key.clone(), device_id);
            let id = TaskId([2; 16]);
            store
                .commit(
                    1,
                    Mutation::Create {
                        hlc: make_hlc(1, 0, 1),
                        id,
                        title: "Task".into(),
                        notes: None,
                        quadrant: 1,
                        due_date: None,
                    },
                )
                .unwrap();
            store
                .commit(
                    2,
                    Mutation::Complete {
                        hlc: make_hlc(2, 0, 1),
                        id,
                    },
                )
                .unwrap();
        }

        // Reopen and verify persistence + WAL replay.
        let store = make_store(&storage, key, device_id);
        assert_eq!(store.store().len(), 1);
        assert!(store.store().get(TaskId([2; 16])).unwrap().is_completed());
        assert_eq!(store.metadata().operation_count, 2);
        assert_eq!(store.outbox().len(), 2);
    }

    #[test]
    fn counter_is_monotonic() {
        let storage = InMemorySecureStorage::default();
        let key = crate::epoch::EpochRoot::generate()
            .unwrap()
            .derive(0)
            .unwrap();
        let device_id = dev(1);

        let mut store = make_store(&storage, key, device_id);
        let id = TaskId([3; 16]);
        store
            .commit(
                1,
                Mutation::Create {
                    hlc: make_hlc(1, 0, 1),
                    id,
                    title: "A".into(),
                    notes: None,
                    quadrant: 0,
                    due_date: None,
                },
            )
            .unwrap();

        assert_eq!(store.metadata().last_nonce, 1);

        store
            .commit(
                2,
                Mutation::Update {
                    hlc: make_hlc(2, 0, 1),
                    id,
                    title: Some("B".into()),
                    notes: None,
                    quadrant: None,
                    due_date: None,
                },
            )
            .unwrap();

        assert_eq!(store.metadata().last_nonce, 2);
    }

    #[test]
    fn apply_is_idempotent_and_tracks_coverage() {
        let storage = InMemorySecureStorage::default();
        let key = crate::epoch::EpochRoot::generate()
            .unwrap()
            .derive(0)
            .unwrap();
        let hlc = make_hlc(1, 0, 1);
        let (owner_trust, device) = create_vault(&storage, hlc).unwrap();
        let mut store = make_store(&storage, key, device.device_id);

        let id = TaskId([4; 16]);
        let hlc = Hlc {
            wall: 2,
            counter: 0,
            device_id: device.device_id,
        };
        let mutation = Mutation::Create {
            hlc,
            id,
            title: "Remote".into(),
            notes: None,
            quadrant: 2,
            due_date: None,
        };
        let envelope = Envelope::sign(&mutation, hlc, &device.signing_key).unwrap();

        let op_id = store.apply(envelope.clone(), &owner_trust, 1).unwrap();
        assert_eq!(store.store().len(), 1);
        assert_eq!(store.metadata().operation_count, 1);
        assert_eq!(store.coverage().len(), 1);
        assert_eq!(store.coverage()[0].end, 1);

        // Applying the same envelope again must be a no-op.
        let op_id2 = store.apply(envelope, &owner_trust, 1).unwrap();
        assert_eq!(op_id, op_id2);
        assert_eq!(store.store().len(), 1);
        assert_eq!(store.metadata().operation_count, 1);
    }

    #[test]
    fn repair_preserves_local_ops_and_replays_subsequent() {
        let storage = InMemorySecureStorage::default();
        let empty_storage = InMemorySecureStorage::default();
        let key = crate::epoch::EpochRoot::generate()
            .unwrap()
            .derive(0)
            .unwrap();
        let hlc = make_hlc(1, 0, 1);
        let (owner_trust, device) = create_vault(&storage, hlc).unwrap();

        let mut store = LocalStore::open(&storage, key, device.device_id).unwrap();
        let id = TaskId([5; 16]);
        store
            .commit(
                2,
                Mutation::Create {
                    hlc: make_hlc(2, 0, 1),
                    id,
                    title: "Local".into(),
                    notes: None,
                    quadrant: 0,
                    due_date: None,
                },
            )
            .unwrap();

        // Create an empty signed snapshot from a fresh store.
        let mut empty_store = LocalStore::open(&empty_storage, key, device.device_id).unwrap();
        let signed = empty_store
            .create_signed_snapshot(&owner_trust.owner_signing_key, owner_trust.vault_id)
            .unwrap();

        // Subsequent update that depends on the preserved local create.
        let update_hlc = Hlc {
            wall: 3,
            counter: 0,
            device_id: device.device_id,
        };
        let update = Mutation::Update {
            hlc: update_hlc,
            id,
            title: Some("Updated".into()),
            notes: None,
            quadrant: None,
            due_date: None,
        };
        let envelope = Envelope::sign(&update, update_hlc, &device.signing_key).unwrap();

        store
            .repair(&signed, &owner_trust, &[(envelope, 2)])
            .unwrap();

        assert_eq!(store.store().len(), 1);
        assert_eq!(
            store.store().get(id).unwrap().title.value.as_ref().unwrap(),
            "Updated"
        );
        let local_cov = store
            .coverage()
            .iter()
            .find(|c| c.origin == device.device_id)
            .unwrap();
        assert_eq!(local_cov.start, 1);
        assert_eq!(local_cov.end, 2);
    }

    // Transaction-boundary fault injection tests (P1.16).

    #[test]
    fn rollback_before_any_durable_write_leaves_empty_store() {
        let inner = InMemorySecureStorage::default();
        let faulty = FaultyStorage::new(inner.clone());
        // Fail on the very first durable write (nonce counter).
        faulty.fail_key_on("nonce:counter", 1);

        let key = crate::epoch::EpochRoot::generate()
            .unwrap()
            .derive(0)
            .unwrap();
        let device_id = dev(1);
        {
            let mut store = LocalStore::open(&faulty, key, device_id).unwrap();
            let result = store.commit(
                1,
                Mutation::Create {
                    hlc: make_hlc(1, 0, 1),
                    id: TaskId([7; 16]),
                    title: "Rolled back".into(),
                    notes: None,
                    quadrant: 0,
                    due_date: None,
                },
            );
            assert!(result.is_err());
        }

        let recovered = LocalStore::open(&inner, key, device_id).unwrap();
        assert!(recovered.store().is_empty());
        assert_eq!(recovered.metadata().operation_count, 0);
    }

    #[test]
    fn crash_after_wal_replays_to_materialized_view() {
        let inner = InMemorySecureStorage::default();
        let faulty = FaultyStorage::new(inner.clone());
        // Fail on the first snapshot ciphertext write: WAL is durable, snapshot is not.
        faulty.fail_key_on("snapshot:ciphertext", 1);

        let key = crate::epoch::EpochRoot::generate()
            .unwrap()
            .derive(0)
            .unwrap();
        let device_id = dev(1);
        let id = TaskId([8; 16]);
        {
            let mut store = LocalStore::open(&faulty, key, device_id).unwrap();
            let result = store.commit(
                1,
                Mutation::Create {
                    hlc: make_hlc(1, 0, 1),
                    id,
                    title: "Replay me".into(),
                    notes: None,
                    quadrant: 0,
                    due_date: None,
                },
            );
            assert!(result.is_err());
        }

        let recovered = LocalStore::open(&inner, key, device_id).unwrap();
        assert_eq!(recovered.store().len(), 1);
        assert_eq!(
            recovered.store().get(id).unwrap().title.value,
            Some("Replay me".into())
        );
    }

    #[test]
    fn full_disk_during_outbox_does_not_corrupt_materialized_view() {
        let inner = InMemorySecureStorage::default();
        let faulty = FaultyStorage::new(inner.clone());
        // Fail on the first outbox ciphertext write, which occurs after WAL and snapshot.
        faulty.fail_key_on("outbox:ciphertext", 1);

        let key = crate::epoch::EpochRoot::generate()
            .unwrap()
            .derive(0)
            .unwrap();
        let device_id = dev(1);
        let id = TaskId([10; 16]);
        {
            let mut store = LocalStore::open(&faulty, key, device_id).unwrap();
            let result = store.commit(
                1,
                Mutation::Create {
                    hlc: make_hlc(1, 0, 1),
                    id,
                    title: "Full disk".into(),
                    notes: None,
                    quadrant: 0,
                    due_date: None,
                },
            );
            assert!(result.is_err());
        }

        let recovered = LocalStore::open(&inner, key, device_id).unwrap();
        assert_eq!(recovered.store().len(), 1);
        assert_eq!(
            recovered.store().get(id).unwrap().title.value,
            Some("Full disk".into())
        );
    }

    #[test]
    fn corrupted_snapshot_ciphertext_is_rejected() {
        let inner = InMemorySecureStorage::default();
        let faulty = FaultyStorage::new(inner.clone());
        // Corrupt every snapshot ciphertext write.
        faulty.corrupt_key("snapshot:ciphertext");

        let key = crate::epoch::EpochRoot::generate()
            .unwrap()
            .derive(0)
            .unwrap();
        let device_id = dev(1);
        {
            let mut store = LocalStore::open(&faulty, key, device_id).unwrap();
            store
                .commit(
                    1,
                    Mutation::Create {
                        hlc: make_hlc(1, 0, 1),
                        id: TaskId([11; 16]),
                        title: "Corrupt snapshot".into(),
                        notes: None,
                        quadrant: 0,
                        due_date: None,
                    },
                )
                .unwrap();
        }

        // An unauthentic snapshot must fail closed on reopen.
        let result = LocalStore::open(&inner, key, device_id);
        assert!(result.is_err());
    }

    #[test]
    fn secure_storage_unavailable_load_uses_defaults() {
        let inner = InMemorySecureStorage::default();
        let faulty = FaultyStorage::new(inner.clone());
        // Make the metadata ciphertext unavailable to load.
        faulty.fail_load_key("metadata:ciphertext");

        let key = crate::epoch::EpochRoot::generate()
            .unwrap()
            .derive(0)
            .unwrap();
        let device_id = dev(1);
        // Opening should succeed with default metadata rather than panic.
        let result = LocalStore::open(&faulty, key, device_id);
        assert!(result.is_ok());
    }
}
