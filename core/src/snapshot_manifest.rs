//! Signed snapshot primitives (P1.12).
//!
//! A signed snapshot packages an encrypted `Snapshot` payload with a manifest
//! that carries the checkpoint HLC, per-origin coverage, and cryptographic
//! commitments to the visible and tombstone task sets. The manifest is signed
//! by the vault owner and can be verified before installation.

use crate::canonical::{self, Limits};
use crate::epoch::{AeadSnapshot, EpochError, EpochKey};
use crate::identity::{OwnerTrust, SignPubKey, SignatureBytes, VaultId};
use crate::store::{CoverageRange, OperationId};
use crate::{DeviceId, Hlc, Task, TaskStore};
use cbor2::Value;
use ed25519_dalek::{Signer, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};

/// Errors returned by signed snapshot operations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SignedSnapshotError {
    Encode(String),
    Decode(String),
    Crypto(EpochError),
    BadSignature,
    Incompatible(String),
    MissingEpochKey,
}

impl std::fmt::Display for SignedSnapshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SignedSnapshotError::Encode(s) => write!(f, "signed snapshot encode error: {s}"),
            SignedSnapshotError::Decode(s) => write!(f, "signed snapshot decode error: {s}"),
            SignedSnapshotError::Crypto(e) => write!(f, "signed snapshot crypto error: {e}"),
            SignedSnapshotError::BadSignature => write!(f, "snapshot manifest signature invalid"),
            SignedSnapshotError::Incompatible(s) => write!(f, "snapshot incompatible: {s}"),
            SignedSnapshotError::MissingEpochKey => write!(f, "epoch key not available"),
        }
    }
}

impl std::error::Error for SignedSnapshotError {}

impl From<EpochError> for SignedSnapshotError {
    fn from(e: EpochError) -> Self {
        SignedSnapshotError::Crypto(e)
    }
}

impl From<crate::canonical::CanonError> for SignedSnapshotError {
    fn from(e: crate::canonical::CanonError) -> Self {
        SignedSnapshotError::Decode(e.to_string())
    }
}

/// Snapshot manifest content that is signed by the owner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SnapshotManifest {
    pub protocol_version: u64,
    pub snapshot_version: u64,
    pub vault_id: VaultId,
    pub checkpoint: Hlc,
    pub coverage: Vec<CoverageRange>,
    pub state_root: [u8; 32],
    pub tombstone_root: [u8; 32],
    pub payload_digest: OperationId,
}

/// A signed encrypted snapshot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignedSnapshot {
    pub manifest: SnapshotManifest,
    pub payload: AeadSnapshot,
    pub signature: SignatureBytes,
}

impl SnapshotManifest {
    /// Sign the canonical manifest content with the owner signing key.
    pub fn sign(&self, owner_signing_key: &ed25519_dalek::SigningKey) -> SignatureBytes {
        let bytes = self.canonical_bytes();
        SignatureBytes(owner_signing_key.sign(&bytes).to_bytes())
    }

    /// Verify the manifest signature against the owner public key.
    pub fn verify(
        &self,
        signature: &SignatureBytes,
        owner_pubkey: &SignPubKey,
    ) -> Result<(), SignedSnapshotError> {
        let bytes = self.canonical_bytes();
        let pubkey = VerifyingKey::from_bytes(&owner_pubkey.0)
            .map_err(|e| SignedSnapshotError::Encode(e.to_string()))?;
        let sig = signature
            .to_ed()
            .map_err(|e| SignedSnapshotError::Encode(e.to_string()))?;
        pubkey
            .verify(&bytes, &sig)
            .map_err(|_| SignedSnapshotError::BadSignature)
    }

    /// Canonical CBOR bytes of the manifest.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        canonical::encode(&self.to_value()).unwrap_or_default()
    }

    fn to_value(&self) -> Value {
        Value::Map(vec![
            (
                text("protocol_version"),
                Value::Integer(self.protocol_version.into()),
            ),
            (
                text("snapshot_version"),
                Value::Integer(self.snapshot_version.into()),
            ),
            (text("vault_id"), Value::Bytes(self.vault_id.0.to_vec())),
            (text("checkpoint"), hlc_to_value(&self.checkpoint)),
            (text("coverage"), coverage_to_value(&self.coverage)),
            (text("state_root"), Value::Bytes(self.state_root.to_vec())),
            (
                text("tombstone_root"),
                Value::Bytes(self.tombstone_root.to_vec()),
            ),
            (
                text("payload_digest"),
                Value::Bytes(self.payload_digest.0.to_vec()),
            ),
        ])
    }

    /// Parse a manifest from a canonical CBOR value.
    pub fn from_value(v: &Value) -> Result<Self, SignedSnapshotError> {
        match v {
            Value::Map(map) => Ok(Self {
                protocol_version: u64_field(map, "protocol_version")?,
                snapshot_version: u64_field(map, "snapshot_version")?,
                vault_id: vault_id_field(map, "vault_id")?,
                checkpoint: hlc_from_value(&get_field(map, "checkpoint")?)?,
                coverage: coverage_from_value(&get_field(map, "coverage")?)?,
                state_root: bytes32_field(map, "state_root")?,
                tombstone_root: bytes32_field(map, "tombstone_root")?,
                payload_digest: op_id_field(map, "payload_digest")?,
            }),
            _ => Err(SignedSnapshotError::Decode("invalid manifest".into())),
        }
    }
}

impl SignedSnapshot {
    /// Encode to canonical CBOR bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>, SignedSnapshotError> {
        let value = Value::Map(vec![
            (text("manifest"), self.manifest.to_value()),
            (text("payload"), aead_snapshot_to_value(&self.payload)),
            (text("signature"), Value::Bytes(self.signature.0.to_vec())),
        ]);
        Ok(canonical::encode(&value)?)
    }

    /// Decode from canonical CBOR bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SignedSnapshotError> {
        let value = canonical::parse(bytes, &Limits::default())?;
        match value {
            Value::Map(map) => Ok(Self {
                manifest: SnapshotManifest::from_value(&get_field(&map, "manifest")?)?,
                payload: aead_snapshot_from_value(&get_field(&map, "payload")?)?,
                signature: signature_field(&get_field(&map, "signature")?)?,
            }),
            _ => Err(SignedSnapshotError::Decode(
                "invalid signed snapshot".into(),
            )),
        }
    }

    /// Verify the owner signature over the manifest.
    pub fn verify_signature(&self, owner_trust: &OwnerTrust) -> Result<(), SignedSnapshotError> {
        self.manifest
            .verify(&self.signature, &owner_trust.owner_pubkey())
    }

    /// Decrypt the payload using the snapshot-purpose epoch key.
    pub fn decrypt_payload(&self, epoch_key: &EpochKey) -> Result<Vec<u8>, SignedSnapshotError> {
        let key = epoch_key.derive_purpose("snapshot")?;
        self.payload.decrypt(&key).map_err(|e| e.into())
    }
}

/// Compute the state commitment (active tasks) and tombstone commitment
/// (deleted tasks) for a `TaskStore`.
pub fn task_store_commitments(store: &TaskStore) -> ([u8; 32], [u8; 32]) {
    let mut active: Vec<&Task> = Vec::new();
    let mut tombstones: Vec<&Task> = Vec::new();
    for (_, task) in store.all_iter() {
        if task.is_deleted() {
            tombstones.push(task);
        } else {
            active.push(task);
        }
    }
    (task_list_root(&active), task_list_root(&tombstones))
}

fn task_list_root(tasks: &[&Task]) -> [u8; 32] {
    let mut items: Vec<Value> = tasks
        .iter()
        .map(|t| {
            let bytes = cbor2::to_canonical_vec(t).unwrap_or_default();
            Value::Bytes(bytes)
        })
        .collect();
    // Sort by task id to make the commitment deterministic and order-independent.
    items.sort_by(|a, b| match (a, b) {
        (Value::Bytes(a), Value::Bytes(b)) => a.cmp(b),
        _ => std::cmp::Ordering::Equal,
    });
    let value = Value::Array(items);
    let bytes = canonical::encode(&value).unwrap_or_default();
    Sha256::digest(&bytes).into()
}

fn hlc_to_value(hlc: &Hlc) -> Value {
    crate::envelope::hlc_to_value(hlc)
}

fn hlc_from_value(v: &Value) -> Result<Hlc, SignedSnapshotError> {
    crate::envelope::value_to_hlc(v).map_err(|e| SignedSnapshotError::Decode(e.to_string()))
}

fn coverage_to_value(coverage: &[CoverageRange]) -> Value {
    Value::Array(
        coverage
            .iter()
            .map(|c| {
                Value::Map(vec![
                    (text("origin"), Value::Bytes(c.origin.0.to_vec())),
                    (text("start"), Value::Integer(c.start.into())),
                    (text("end"), Value::Integer(c.end.into())),
                ])
            })
            .collect(),
    )
}

fn coverage_from_value(v: &Value) -> Result<Vec<CoverageRange>, SignedSnapshotError> {
    match v {
        Value::Array(arr) => arr
            .iter()
            .map(|v| match v {
                Value::Map(map) => Ok(CoverageRange {
                    origin: device_id_field(map, "origin")?,
                    start: u64_field(map, "start")?,
                    end: u64_field(map, "end")?,
                }),
                _ => Err(SignedSnapshotError::Decode("invalid coverage entry".into())),
            })
            .collect(),
        _ => Err(SignedSnapshotError::Decode(
            "coverage must be an array".into(),
        )),
    }
}

fn aead_snapshot_to_value(s: &AeadSnapshot) -> Value {
    Value::Map(vec![
        (text("counter"), Value::Integer(s.counter.into())),
        (text("nonce"), Value::Bytes(s.nonce.to_vec())),
        (text("payload"), Value::Bytes(s.payload.clone())),
    ])
}

fn aead_snapshot_from_value(v: &Value) -> Result<AeadSnapshot, SignedSnapshotError> {
    match v {
        Value::Map(map) => Ok(AeadSnapshot {
            counter: u64_field(map, "counter")?,
            nonce: bytes16_field(map, "nonce")?,
            payload: bytes_field(map, "payload")?,
        }),
        _ => Err(SignedSnapshotError::Decode("invalid aead snapshot".into())),
    }
}

fn text(s: &str) -> Value {
    Value::Text(s.into())
}

fn get_field(map: &[(Value, Value)], key: &str) -> Result<Value, SignedSnapshotError> {
    map.iter()
        .find(|(k, _)| matches!(k, Value::Text(t) if t == key))
        .map(|(_, v)| v.clone())
        .ok_or_else(|| SignedSnapshotError::Decode(format!("missing field: {key}")))
}

fn u64_field(map: &[(Value, Value)], key: &str) -> Result<u64, SignedSnapshotError> {
    match get_field(map, key)? {
        Value::Integer(i) => {
            let n: i128 = i.into();
            if n < 0 || n > u64::MAX as i128 {
                return Err(SignedSnapshotError::Decode(format!(
                    "{key} out of u64 range"
                )));
            }
            Ok(n as u64)
        }
        _ => Err(SignedSnapshotError::Decode(format!("{key} not an integer"))),
    }
}

fn vault_id_field(map: &[(Value, Value)], key: &str) -> Result<VaultId, SignedSnapshotError> {
    match get_field(map, key)? {
        Value::Bytes(b) if b.len() == 16 => {
            let arr: [u8; 16] = b.try_into().unwrap();
            Ok(VaultId(arr))
        }
        _ => Err(SignedSnapshotError::Decode(format!("{key} not a vault id"))),
    }
}

fn device_id_field(map: &[(Value, Value)], key: &str) -> Result<DeviceId, SignedSnapshotError> {
    match get_field(map, key)? {
        Value::Bytes(b) if b.len() == 16 => {
            let arr: [u8; 16] = b.try_into().unwrap();
            Ok(DeviceId(arr))
        }
        _ => Err(SignedSnapshotError::Decode(format!(
            "{key} not a device id"
        ))),
    }
}

fn bytes32_field(map: &[(Value, Value)], key: &str) -> Result<[u8; 32], SignedSnapshotError> {
    match get_field(map, key)? {
        Value::Bytes(b) if b.len() == 32 => {
            let arr: [u8; 32] = b.try_into().unwrap();
            Ok(arr)
        }
        _ => Err(SignedSnapshotError::Decode(format!("{key} not 32 bytes"))),
    }
}

fn bytes16_field(map: &[(Value, Value)], key: &str) -> Result<[u8; 12], SignedSnapshotError> {
    match get_field(map, key)? {
        Value::Bytes(b) if b.len() == 12 => {
            let arr: [u8; 12] = b.try_into().unwrap();
            Ok(arr)
        }
        _ => Err(SignedSnapshotError::Decode(format!("{key} not 12 bytes"))),
    }
}

fn bytes_field(map: &[(Value, Value)], key: &str) -> Result<Vec<u8>, SignedSnapshotError> {
    match get_field(map, key)? {
        Value::Bytes(b) => Ok(b),
        _ => Err(SignedSnapshotError::Decode(format!("{key} not bytes"))),
    }
}

fn op_id_field(map: &[(Value, Value)], key: &str) -> Result<OperationId, SignedSnapshotError> {
    match get_field(map, key)? {
        Value::Bytes(b) if b.len() == 32 => {
            let arr: [u8; 32] = b.try_into().unwrap();
            Ok(OperationId(arr))
        }
        _ => Err(SignedSnapshotError::Decode(format!("{key} not an op_id"))),
    }
}

fn signature_field(v: &Value) -> Result<SignatureBytes, SignedSnapshotError> {
    match v {
        Value::Bytes(b) if b.len() == 64 => {
            let arr: [u8; 64] = b.clone().try_into().unwrap();
            Ok(SignatureBytes(arr))
        }
        _ => Err(SignedSnapshotError::Decode("invalid signature".into())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::{create_vault, InMemorySecureStorage};
    use crate::store::LocalStore;
    use crate::{Hlc, Mutation, TaskId};

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

    #[test]
    fn signed_snapshot_round_trips_and_verifies() {
        let storage = InMemorySecureStorage::default();
        let hlc = make_hlc(1, 0, 1);
        let (owner_trust, device) = create_vault(&storage, hlc).unwrap();

        let epoch_root = crate::epoch::EpochRoot::generate().unwrap();
        let epoch_key = epoch_root.derive(0).unwrap();
        let mut store = LocalStore::open(&storage, epoch_key, device.device_id).unwrap();

        let id = TaskId([1; 16]);
        store
            .commit(
                1,
                Mutation::Create {
                    hlc: make_hlc(2, 0, 1),
                    id,
                    title: "A".into(),
                    notes: None,
                    quadrant: 0,
                    due_date: None,
                },
            )
            .unwrap();

        let signed = store
            .create_signed_snapshot(&owner_trust.owner_signing_key, owner_trust.vault_id)
            .unwrap();

        let bytes = signed.to_bytes().unwrap();
        let parsed = SignedSnapshot::from_bytes(&bytes).unwrap();
        parsed.verify_signature(&owner_trust).unwrap();
    }
}
