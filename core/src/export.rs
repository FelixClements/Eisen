//! Encrypted export/import staging (P1.15).
//!
//! A `VaultExport` is a portable, encrypted file that contains a snapshot of the
//! local vault state: task snapshot, store metadata, outbox, and the manifest
//! chain. It is encrypted under an export-purpose sub-key derived from the
//! epoch key, so it never writes plaintext task content to user-accessible disk
//! and cannot be decrypted without the same vault keys.

use crate::canonical;
use crate::epoch::{EpochError, EpochKey};
use crate::identity::{GenesisManifest, IdentityError, VaultId};
use crate::snapshot::{Snapshot, SnapshotError};
use crate::store::{outbox_from_bytes, outbox_to_bytes, OutboxEntry, StoreMetadata};
use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use cbor2::Value;
use sha2::{Digest, Sha256};

const EXPORT_MAGIC: &[u8] = b"EISEN-EXPORT";
const VERSION: u32 = 1;
const NONCE_LEN: usize = 12;
const CHECKSUM_LEN: usize = 32;
const U32_LEN: usize = 4;
const U64_LEN: usize = 8;
const VAULT_ID_LEN: usize = 16;

/// Errors returned by export/import staging operations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExportError {
    Encode(String),
    Decode(String),
    Crypto(String),
    Integrity(String),
    Entropy(String),
    Epoch(EpochError),
    Snapshot(SnapshotError),
    Identity(IdentityError),
    Version(u32),
}

impl std::fmt::Display for ExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExportError::Encode(s) => write!(f, "export encode error: {s}"),
            ExportError::Decode(s) => write!(f, "export decode error: {s}"),
            ExportError::Crypto(s) => write!(f, "export crypto error: {s}"),
            ExportError::Integrity(s) => write!(f, "export integrity error: {s}"),
            ExportError::Entropy(s) => write!(f, "export entropy error: {s}"),
            ExportError::Epoch(e) => write!(f, "export epoch error: {e}"),
            ExportError::Snapshot(e) => write!(f, "export snapshot error: {e}"),
            ExportError::Identity(e) => write!(f, "export identity error: {e}"),
            ExportError::Version(v) => write!(f, "unsupported export version: {v}"),
        }
    }
}

impl std::error::Error for ExportError {}

impl From<EpochError> for ExportError {
    fn from(e: EpochError) -> Self {
        ExportError::Epoch(e)
    }
}

impl From<SnapshotError> for ExportError {
    fn from(e: SnapshotError) -> Self {
        ExportError::Snapshot(e)
    }
}

impl From<IdentityError> for ExportError {
    fn from(e: IdentityError) -> Self {
        ExportError::Identity(e)
    }
}

/// Decrypted payload contained in a `VaultExport`.
#[derive(Debug)]
pub struct ExportPayload {
    pub version: u32,
    pub vault_id: VaultId,
    pub key_epoch: u64,
    pub timestamp: u64,
    pub snapshot: Snapshot,
    pub metadata: StoreMetadata,
    pub outbox: Vec<OutboxEntry>,
    pub manifests: Vec<GenesisManifest>,
}

/// A portable encrypted vault export.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VaultExport {
    pub vault_id: VaultId,
    pub key_epoch: u64,
    pub timestamp: u64,
    pub nonce: [u8; NONCE_LEN],
    pub payload: Vec<u8>,
    pub checksum: [u8; CHECKSUM_LEN],
}

impl VaultExport {
    /// Create an encrypted export from a snapshot of local vault state.
    pub fn new(
        epoch_key: &EpochKey,
        vault_id: VaultId,
        snapshot: &Snapshot,
        metadata: &StoreMetadata,
        outbox: &[OutboxEntry],
        manifests: &[GenesisManifest],
        timestamp: u64,
    ) -> Result<Self, ExportError> {
        let export_key = epoch_key.derive_purpose("export")?;
        let mut nonce = [0u8; NONCE_LEN];
        getrandom::getrandom(&mut nonce).map_err(|e| ExportError::Entropy(e.to_string()))?;

        let payload_plaintext = payload_to_bytes(
            snapshot,
            metadata,
            outbox,
            manifests,
            vault_id,
            epoch_key.epoch,
            timestamp,
        )?;
        let aad = aad_value(vault_id, epoch_key.epoch, timestamp, &nonce)?;
        let aad_bytes = canonical::encode(&aad).map_err(|e| ExportError::Encode(e.to_string()))?;

        let cipher = Aes256Gcm::new_from_slice(&export_key.key)
            .map_err(|e| ExportError::Crypto(e.to_string()))?;
        let nonce_slice = Nonce::from_slice(&nonce);
        let payload = cipher
            .encrypt(
                nonce_slice,
                Payload {
                    msg: &payload_plaintext,
                    aad: &aad_bytes,
                },
            )
            .map_err(|e| ExportError::Crypto(e.to_string()))?;

        let mut prefix = Vec::new();
        prefix.extend_from_slice(EXPORT_MAGIC);
        prefix.extend_from_slice(&VERSION.to_be_bytes());
        prefix.extend_from_slice(&vault_id.0);
        prefix.extend_from_slice(&epoch_key.epoch.to_be_bytes());
        prefix.extend_from_slice(&timestamp.to_be_bytes());
        prefix.extend_from_slice(&nonce);
        prefix.extend_from_slice(&(payload.len() as u64).to_be_bytes());
        prefix.extend_from_slice(&payload);
        let checksum: [u8; CHECKSUM_LEN] = Sha256::digest(&prefix).into();

        Ok(Self {
            vault_id,
            key_epoch: epoch_key.epoch,
            timestamp,
            nonce,
            payload,
            checksum,
        })
    }

    /// Parse an export from its wire/file encoding and verify the checksum.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ExportError> {
        let mut offset = 0;
        let magic = read_bytes(bytes, &mut offset, EXPORT_MAGIC.len())?;
        if magic != EXPORT_MAGIC {
            return Err(ExportError::Integrity("bad magic".into()));
        }
        let version = read_u32(bytes, &mut offset)?;
        if version != VERSION {
            return Err(ExportError::Version(version));
        }
        let vault_id_bytes = read_bytes(bytes, &mut offset, VAULT_ID_LEN)?;
        let vault_id = VaultId(vault_id_bytes.try_into().unwrap());
        let key_epoch = read_u64(bytes, &mut offset)?;
        let timestamp = read_u64(bytes, &mut offset)?;
        let nonce = read_bytes(bytes, &mut offset, NONCE_LEN)?;
        let nonce: [u8; NONCE_LEN] = nonce.try_into().unwrap();
        let payload_len = read_u64(bytes, &mut offset)? as usize;
        let payload = read_bytes(bytes, &mut offset, payload_len)?.to_vec();

        if bytes.len() - offset != CHECKSUM_LEN {
            return Err(ExportError::Integrity(
                "trailing data or missing checksum".into(),
            ));
        }
        let checksum: [u8; CHECKSUM_LEN] = bytes[offset..].try_into().unwrap();
        let expected = Sha256::digest(&bytes[..offset]);
        if expected.as_slice() != checksum.as_slice() {
            return Err(ExportError::Integrity("checksum mismatch".into()));
        }

        Ok(Self {
            vault_id,
            key_epoch,
            timestamp,
            nonce,
            payload,
            checksum,
        })
    }

    /// Serialize the export to its wire/file encoding.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(
            EXPORT_MAGIC.len()
                + U32_LEN
                + VAULT_ID_LEN
                + 3 * U64_LEN
                + NONCE_LEN
                + self.payload.len()
                + CHECKSUM_LEN,
        );
        out.extend_from_slice(EXPORT_MAGIC);
        out.extend_from_slice(&VERSION.to_be_bytes());
        out.extend_from_slice(&self.vault_id.0);
        out.extend_from_slice(&self.key_epoch.to_be_bytes());
        out.extend_from_slice(&self.timestamp.to_be_bytes());
        out.extend_from_slice(&self.nonce);
        out.extend_from_slice(&(self.payload.len() as u64).to_be_bytes());
        out.extend_from_slice(&self.payload);
        out.extend_from_slice(&self.checksum);
        out
    }

    /// Decrypt and verify the export payload using the matching epoch key.
    pub fn decrypt(&self, epoch_key: &EpochKey) -> Result<ExportPayload, ExportError> {
        if epoch_key.epoch != self.key_epoch {
            return Err(ExportError::Crypto("export key epoch mismatch".into()));
        }
        let export_key = epoch_key.derive_purpose("export")?;
        let aad = aad_value(self.vault_id, self.key_epoch, self.timestamp, &self.nonce)?;
        let aad_bytes = canonical::encode(&aad).map_err(|e| ExportError::Encode(e.to_string()))?;

        let cipher = Aes256Gcm::new_from_slice(&export_key.key)
            .map_err(|e| ExportError::Crypto(e.to_string()))?;
        let nonce_slice = Nonce::from_slice(&self.nonce);
        let plaintext = cipher
            .decrypt(
                nonce_slice,
                Payload {
                    msg: &self.payload,
                    aad: &aad_bytes,
                },
            )
            .map_err(|_| {
                ExportError::Crypto("export decryption or authentication failed".into())
            })?;

        let value = canonical::parse(&plaintext, &canonical::Limits::default())
            .map_err(|e| ExportError::Decode(e.to_string()))?;
        payload_from_value(&value)
    }
}

fn aad_value(
    vault_id: VaultId,
    key_epoch: u64,
    timestamp: u64,
    nonce: &[u8; NONCE_LEN],
) -> Result<Value, ExportError> {
    Ok(Value::Map(vec![
        (text("key_epoch"), u64_value(key_epoch)),
        (text("nonce"), Value::Bytes(nonce.to_vec())),
        (text("timestamp"), u64_value(timestamp)),
        (text("vault_id"), Value::Bytes(vault_id.0.to_vec())),
        (text("version"), u64_value(VERSION as u64)),
    ]))
}

fn payload_to_bytes(
    snapshot: &Snapshot,
    metadata: &StoreMetadata,
    outbox: &[OutboxEntry],
    manifests: &[GenesisManifest],
    vault_id: VaultId,
    key_epoch: u64,
    timestamp: u64,
) -> Result<Vec<u8>, ExportError> {
    let manifests: Result<Vec<Value>, ExportError> = manifests
        .iter()
        .map(|m| Ok(Value::Bytes(m.to_bytes()?)))
        .collect();
    let value = Value::Map(vec![
        (text("version"), u64_value(VERSION as u64)),
        (text("vault_id"), Value::Bytes(vault_id.0.to_vec())),
        (text("key_epoch"), u64_value(key_epoch)),
        (text("timestamp"), u64_value(timestamp)),
        (text("snapshot"), Value::Bytes(snapshot.to_bytes()?)),
        (
            text("metadata"),
            Value::Bytes(
                metadata
                    .to_bytes()
                    .map_err(|e| ExportError::Encode(e.to_string()))?,
            ),
        ),
        (
            text("outbox"),
            Value::Bytes(outbox_to_bytes(outbox).map_err(|e| ExportError::Encode(e.to_string()))?),
        ),
        (text("manifests"), Value::Array(manifests?)),
    ]);
    canonical::encode(&value).map_err(|e| ExportError::Encode(e.to_string()))
}

fn payload_from_value(v: &Value) -> Result<ExportPayload, ExportError> {
    let map = match v {
        Value::Map(m) => m,
        _ => return Err(ExportError::Decode("export payload is not a map".into())),
    };

    let version = u32_from_value(&get_field(map, "version")?)?;
    if version != VERSION {
        return Err(ExportError::Version(version));
    }
    let vault_id = vault_id_field(map, "vault_id")?;
    let key_epoch = u64_from_value(&get_field(map, "key_epoch")?)?;
    let timestamp = u64_from_value(&get_field(map, "timestamp")?)?;

    let snapshot_bytes = bytes_field(map, "snapshot")?;
    let snapshot = Snapshot::from_bytes(&snapshot_bytes)?;

    let metadata_bytes = bytes_field(map, "metadata")?;
    let metadata = StoreMetadata::from_bytes(&metadata_bytes)
        .map_err(|e| ExportError::Decode(e.to_string()))?;

    let outbox_bytes = bytes_field(map, "outbox")?;
    let outbox =
        outbox_from_bytes(&outbox_bytes).map_err(|e| ExportError::Decode(e.to_string()))?;

    let manifests_value = get_field(map, "manifests")?;
    let mut manifests = Vec::new();
    match manifests_value {
        Value::Array(arr) => {
            for v in arr {
                let bytes = match v {
                    Value::Bytes(b) => b,
                    _ => return Err(ExportError::Decode("manifest must be bytes".into())),
                };
                manifests.push(GenesisManifest::from_bytes(&bytes)?);
            }
        }
        _ => return Err(ExportError::Decode("manifests is not an array".into())),
    }

    Ok(ExportPayload {
        version,
        vault_id,
        key_epoch,
        timestamp,
        snapshot,
        metadata,
        outbox,
        manifests,
    })
}

fn read_u32(bytes: &[u8], offset: &mut usize) -> Result<u32, ExportError> {
    let b = read_bytes(bytes, offset, U32_LEN)?;
    Ok(u32::from_be_bytes(b.try_into().unwrap()))
}

fn read_u64(bytes: &[u8], offset: &mut usize) -> Result<u64, ExportError> {
    let b = read_bytes(bytes, offset, U64_LEN)?;
    Ok(u64::from_be_bytes(b.try_into().unwrap()))
}

fn read_bytes<'a>(
    bytes: &'a [u8],
    offset: &mut usize,
    len: usize,
) -> Result<&'a [u8], ExportError> {
    if bytes.len() < *offset + len {
        return Err(ExportError::Integrity("truncated export".into()));
    }
    let out = &bytes[*offset..*offset + len];
    *offset += len;
    Ok(out)
}

fn text(s: &str) -> Value {
    Value::Text(s.into())
}

fn u64_value(n: u64) -> Value {
    Value::Integer(cbor2::value::Integer::from(n))
}

fn u32_from_value(v: &Value) -> Result<u32, ExportError> {
    let n = u64_from_value(v)?;
    u32::try_from(n).map_err(|_| ExportError::Decode("integer out of u32 range".into()))
}

fn u64_from_value(v: &Value) -> Result<u64, ExportError> {
    match v {
        Value::Integer(i) => {
            let n: i128 = (*i).into();
            if n < 0 {
                return Err(ExportError::Decode("negative integer".into()));
            }
            u64::try_from(n).map_err(|_| ExportError::Decode("integer too large for u64".into()))
        }
        _ => Err(ExportError::Decode("expected integer".into())),
    }
}

fn get_field(map: &[(Value, Value)], key: &str) -> Result<Value, ExportError> {
    map.iter()
        .find(|(k, _)| matches!(k, Value::Text(t) if t == key))
        .map(|(_, v)| v.clone())
        .ok_or_else(|| ExportError::Decode(format!("missing field {key}")))
}

fn bytes_field(map: &[(Value, Value)], key: &str) -> Result<Vec<u8>, ExportError> {
    match get_field(map, key)? {
        Value::Bytes(b) => Ok(b),
        _ => Err(ExportError::Decode(format!("{key}: expected bytes"))),
    }
}

fn vault_id_field(map: &[(Value, Value)], key: &str) -> Result<VaultId, ExportError> {
    match get_field(map, key)? {
        Value::Bytes(b) if b.len() == VAULT_ID_LEN => {
            let arr: [u8; VAULT_ID_LEN] = b.as_slice().try_into().unwrap();
            Ok(VaultId(arr))
        }
        _ => Err(ExportError::Decode(format!("{key}: expected 16 bytes"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::epoch::EpochRoot;
    use crate::identity::{create_vault, InMemorySecureStorage};
    use crate::manifest::ManifestChain;
    use crate::store::LocalStore;
    use crate::{DeviceId, Hlc, Mutation, TaskId};

    fn dev(n: u8) -> DeviceId {
        DeviceId([n; 16])
    }

    fn make_hlc(wall: u64, counter: u32, device_id: DeviceId) -> Hlc {
        Hlc {
            wall,
            counter,
            device_id,
        }
    }

    fn make_store(
        storage: &InMemorySecureStorage,
        key: crate::epoch::EpochKey,
        device_id: DeviceId,
    ) -> LocalStore<'_> {
        LocalStore::open(storage, key, device_id).unwrap()
    }

    #[test]
    fn export_and_import_round_trip() {
        let storage = InMemorySecureStorage::default();
        let hlc = make_hlc(1, 0, dev(1));
        let (owner_trust, device) = create_vault(&storage, hlc).unwrap();
        let epoch_key = EpochRoot::generate().unwrap().derive(0).unwrap();
        let mut store = make_store(&storage, epoch_key, device.device_id);

        let id = TaskId([1; 16]);
        let mutation = Mutation::Create {
            hlc: make_hlc(1, 0, device.device_id),
            id,
            title: "Buy milk".into(),
            notes: None,
            quadrant: 0,
            due_date: None,
        };
        store.commit(1, mutation).unwrap();

        let chain = ManifestChain::new(owner_trust.genesis_manifest.clone()).unwrap();
        let export = store.export(&owner_trust, &chain).unwrap();
        let bytes = export.to_bytes();
        let parsed = VaultExport::from_bytes(&bytes).unwrap();

        let storage2 = InMemorySecureStorage::default();
        let mut store2 = make_store(&storage2, epoch_key, device.device_id);
        store2.import(&parsed, &owner_trust).unwrap();

        assert_eq!(store2.store().len(), 1);
        let task = store2.store().values().next().unwrap();
        assert_eq!(task.title.value.as_ref().unwrap(), "Buy milk");
        assert_eq!(store2.metadata().operation_count, 1);
        assert_eq!(store2.outbox().len(), 1);
    }

    #[test]
    fn tampered_export_checksum_rejected() {
        let storage = InMemorySecureStorage::default();
        let hlc = make_hlc(1, 0, dev(1));
        let (owner_trust, device) = create_vault(&storage, hlc).unwrap();
        let epoch_key = EpochRoot::generate().unwrap().derive(0).unwrap();
        let mut store = make_store(&storage, epoch_key, device.device_id);

        let id = TaskId([1; 16]);
        let mutation = Mutation::Create {
            hlc: make_hlc(1, 0, device.device_id),
            id,
            title: "A".into(),
            notes: None,
            quadrant: 0,
            due_date: None,
        };
        store.commit(1, mutation).unwrap();

        let chain = ManifestChain::new(owner_trust.genesis_manifest.clone()).unwrap();
        let mut export = store.export(&owner_trust, &chain).unwrap();
        export.payload[0] ^= 0xff;
        let bytes = export.to_bytes();
        let err = VaultExport::from_bytes(&bytes).unwrap_err();
        assert!(matches!(err, ExportError::Integrity(_)));
    }

    #[test]
    fn wrong_epoch_import_fails() {
        let storage = InMemorySecureStorage::default();
        let hlc = make_hlc(1, 0, dev(1));
        let (owner_trust, device) = create_vault(&storage, hlc).unwrap();
        let epoch_key = EpochRoot::generate().unwrap().derive(0).unwrap();
        let mut store = make_store(&storage, epoch_key, device.device_id);

        let id = TaskId([1; 16]);
        let mutation = Mutation::Create {
            hlc: make_hlc(1, 0, device.device_id),
            id,
            title: "A".into(),
            notes: None,
            quadrant: 0,
            due_date: None,
        };
        store.commit(1, mutation).unwrap();

        let chain = ManifestChain::new(owner_trust.genesis_manifest.clone()).unwrap();
        let export = store.export(&owner_trust, &chain).unwrap();

        let wrong_key = EpochRoot::generate().unwrap().derive(1).unwrap();
        let err = export.decrypt(&wrong_key).unwrap_err();
        assert!(matches!(err, ExportError::Crypto(_)));
    }
}
