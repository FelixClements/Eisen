//! Device identity and owner trust anchor (P1.04).
//!
//! Generates per-device signing (Ed25519) and encryption (X25519) keys, a
//! stable random device ID, an owner signing key, and a signed genesis
//! manifest. Private key material is passed to a pluggable secure-storage
//! backend so the core remains platform-agnostic.

use crate::{canonical, DeviceId, Hlc};
use cbor2;
use ed25519_dalek::{Signature as EdSignature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{de, ser, Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use x25519_dalek::{PublicKey as X25519PublicKey, StaticSecret as X25519StaticSecret};

const SIGNING_KEY_LEN: usize = 32;
const ENCRYPTION_KEY_LEN: usize = 32;
const SIGNATURE_LEN: usize = 64;
const ID_LEN: usize = 16;

/// A vault identifier: 128 random bits.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VaultId(pub [u8; ID_LEN]);

impl VaultId {
    /// Generate a new random vault ID.
    pub fn generate() -> Result<Self, IdentityError> {
        let mut bytes = [0u8; ID_LEN];
        getrandom::getrandom(&mut bytes).map_err(|e| IdentityError::Entropy(e.to_string()))?;
        Ok(Self(bytes))
    }

    /// Convert to big-endian hex for diagnostics.
    pub fn to_hex(&self) -> String {
        self.0.iter().map(|b| format!("{b:02x}")).collect()
    }
}

impl Serialize for VaultId {
    fn serialize<S: ser::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_bytes(&self.0)
    }
}

impl<'de> Deserialize<'de> for VaultId {
    fn deserialize<D: de::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> de::Visitor<'de> for V {
            type Value = VaultId;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, "a 16-byte CBOR byte string")
            }
            fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
                let arr: [u8; ID_LEN] = v.try_into().map_err(|_| E::custom("expected 16 bytes"))?;
                Ok(VaultId(arr))
            }
        }
        d.deserialize_bytes(V)
    }
}

/// Ed25519 signing public key (32 bytes).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct SignPubKey(pub [u8; SIGNING_KEY_LEN]);

impl Serialize for SignPubKey {
    fn serialize<S: ser::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_bytes(&self.0)
    }
}

impl<'de> Deserialize<'de> for SignPubKey {
    fn deserialize<D: de::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> de::Visitor<'de> for V {
            type Value = SignPubKey;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, "a 32-byte CBOR byte string")
            }
            fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
                let arr: [u8; SIGNING_KEY_LEN] =
                    v.try_into().map_err(|_| E::custom("expected 32 bytes"))?;
                Ok(SignPubKey(arr))
            }
        }
        d.deserialize_bytes(V)
    }
}

/// X25519 encryption public key (32 bytes).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct EncPubKey(pub [u8; ENCRYPTION_KEY_LEN]);

impl Serialize for EncPubKey {
    fn serialize<S: ser::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_bytes(&self.0)
    }
}

impl<'de> Deserialize<'de> for EncPubKey {
    fn deserialize<D: de::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> de::Visitor<'de> for V {
            type Value = EncPubKey;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, "a 32-byte CBOR byte string")
            }
            fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
                let arr: [u8; ENCRYPTION_KEY_LEN] =
                    v.try_into().map_err(|_| E::custom("expected 32 bytes"))?;
                Ok(EncPubKey(arr))
            }
        }
        d.deserialize_bytes(V)
    }
}

/// Ed25519 signature (64 bytes).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct SignatureBytes(pub [u8; SIGNATURE_LEN]);

impl SignatureBytes {
    pub fn to_ed(&self) -> Result<EdSignature, IdentityError> {
        EdSignature::from_slice(&self.0).map_err(|e| IdentityError::Crypto(e.to_string()))
    }
}

impl Serialize for SignatureBytes {
    fn serialize<S: ser::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_bytes(&self.0)
    }
}

impl<'de> Deserialize<'de> for SignatureBytes {
    fn deserialize<D: de::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> de::Visitor<'de> for V {
            type Value = SignatureBytes;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, "a 64-byte CBOR byte string")
            }
            fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
                let arr: [u8; SIGNATURE_LEN] =
                    v.try_into().map_err(|_| E::custom("expected 64 bytes"))?;
                Ok(SignatureBytes(arr))
            }
        }
        d.deserialize_bytes(V)
    }
}

/// Device status in the membership manifest.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceStatus {
    Active,
    Revoked,
}

/// A device entry in a manifest.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceEntry {
    pub device_id: DeviceId,
    pub signing_pubkey: SignPubKey,
    pub encryption_pubkey: EncPubKey,
    pub status: DeviceStatus,
}

/// The signed payload of a membership manifest.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestContent {
    pub protocol_version: u64,
    pub membership_version: u64,
    pub key_epoch: u64,
    pub vault_id: VaultId,
    pub owner_pubkey: SignPubKey,
    pub owner_management_device_id: DeviceId,
    pub created_at: Hlc,
    pub devices: Vec<DeviceEntry>,
}

/// A signed genesis / membership manifest.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenesisManifest {
    pub content: ManifestContent,
    pub signature: SignatureBytes,
}

impl GenesisManifest {
    /// Verify the owner signature over the canonical content.
    pub fn verify(&self) -> Result<(), IdentityError> {
        let content_bytes = canonical_content_bytes(&self.content)?;
        let pubkey = VerifyingKey::from_bytes(&self.content.owner_pubkey.0)
            .map_err(|e| IdentityError::Crypto(e.to_string()))?;
        let sig = self.signature.to_ed()?;
        pubkey
            .verify(&content_bytes, &sig)
            .map_err(|e| IdentityError::Crypto(e.to_string()))
    }

    /// Encode the whole manifest (content + signature) to canonical CBOR.
    pub fn to_bytes(&self) -> Result<Vec<u8>, IdentityError> {
        cbor2::to_canonical_vec(self).map_err(|e| IdentityError::Encode(e.to_string()))
    }

    /// Decode and validate a manifest from canonical CBOR bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, IdentityError> {
        canonical::parse(bytes, &canonical::Limits::default())?;
        cbor2::from_slice(bytes).map_err(|e| IdentityError::Encode(e.to_string()))
    }
}

fn canonical_content_bytes(content: &ManifestContent) -> Result<Vec<u8>, IdentityError> {
    cbor2::to_canonical_vec(content).map_err(|e| IdentityError::Encode(e.to_string()))
}

/// Errors from identity operations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IdentityError {
    Entropy(String),
    Encode(String),
    Decode(String),
    Crypto(String),
    Storage(String),
    MissingKey(String),
    InvalidManifest,
}

impl std::fmt::Display for IdentityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IdentityError::Entropy(s) => write!(f, "entropy error: {s}"),
            IdentityError::Encode(s) => write!(f, "encode error: {s}"),
            IdentityError::Decode(s) => write!(f, "decode error: {s}"),
            IdentityError::Crypto(s) => write!(f, "crypto error: {s}"),
            IdentityError::Storage(s) => write!(f, "storage error: {s}"),
            IdentityError::MissingKey(s) => write!(f, "missing storage key: {s}"),
            IdentityError::InvalidManifest => write!(f, "invalid manifest"),
        }
    }
}

impl std::error::Error for IdentityError {}

impl From<canonical::CanonError> for IdentityError {
    fn from(e: canonical::CanonError) -> Self {
        IdentityError::Decode(e.to_string())
    }
}

/// Pluggable secure key storage backend.
pub trait SecureStorage: Send + Sync {
    /// Store a key-value pair.
    fn store(&self, key: &str, value: &[u8]) -> Result<(), IdentityError>;
    /// Load a value if it exists.
    fn load(&self, key: &str) -> Result<Option<Vec<u8>>, IdentityError>;
    /// Delete a value.
    fn delete(&self, key: &str) -> Result<(), IdentityError>;
}

/// In-memory secure storage for tests.
#[derive(Clone, Default)]
pub struct InMemorySecureStorage {
    inner: Arc<Mutex<HashMap<String, Vec<u8>>>>,
}

impl SecureStorage for InMemorySecureStorage {
    fn store(&self, key: &str, value: &[u8]) -> Result<(), IdentityError> {
        self.inner
            .lock()
            .map_err(|e| IdentityError::Storage(e.to_string()))?
            .insert(key.to_string(), value.to_vec());
        Ok(())
    }

    fn load(&self, key: &str) -> Result<Option<Vec<u8>>, IdentityError> {
        Ok(self
            .inner
            .lock()
            .map_err(|e| IdentityError::Storage(e.to_string()))?
            .get(key)
            .cloned())
    }

    fn delete(&self, key: &str) -> Result<(), IdentityError> {
        self.inner
            .lock()
            .map_err(|e| IdentityError::Storage(e.to_string()))?
            .remove(key);
        Ok(())
    }
}

/// Device identity: stable ID plus signing and encryption secrets.
pub struct DeviceIdentity {
    pub device_id: DeviceId,
    pub signing_key: SigningKey,
    pub encryption_key: X25519StaticSecret,
}

impl DeviceIdentity {
    /// Generate a fresh device identity from OS entropy.
    pub fn generate() -> Result<Self, IdentityError> {
        let device_id = generate_id()?;
        let signing_key = generate_signing_key()?;
        let encryption_key = generate_encryption_key()?;
        Ok(Self {
            device_id,
            signing_key,
            encryption_key,
        })
    }

    /// Return the Ed25519 verifying key bytes.
    pub fn signing_pubkey(&self) -> SignPubKey {
        SignPubKey(self.signing_key.verifying_key().to_bytes())
    }

    /// Return the X25519 public key bytes.
    pub fn encryption_pubkey(&self) -> EncPubKey {
        EncPubKey(X25519PublicKey::from(&self.encryption_key).to_bytes())
    }

    /// Return a device entry for inclusion in a manifest.
    pub fn manifest_entry(&self, status: DeviceStatus) -> DeviceEntry {
        DeviceEntry {
            device_id: self.device_id,
            signing_pubkey: self.signing_pubkey(),
            encryption_pubkey: self.encryption_pubkey(),
            status,
        }
    }

    /// Persist raw secret key material to secure storage.
    pub fn persist(&self, storage: &dyn SecureStorage) -> Result<(), IdentityError> {
        let hex = self.device_id.to_hex();
        storage.store(
            &format!("device:{hex}:signing"),
            &self.signing_key.to_bytes(),
        )?;
        storage.store(
            &format!("device:{hex}:encryption"),
            self.encryption_key.as_bytes(),
        )?;
        storage.store(&format!("device:{hex}:id"), &self.device_id.0)?;
        Ok(())
    }

    /// Load a device identity from secure storage.
    pub fn load(device_id: DeviceId, storage: &dyn SecureStorage) -> Result<Self, IdentityError> {
        let hex = device_id.to_hex();
        let signing_bytes = storage
            .load(&format!("device:{hex}:signing"))?
            .ok_or_else(|| IdentityError::MissingKey(format!("device:{hex}:signing")))?;
        let encryption_bytes = storage
            .load(&format!("device:{hex}:encryption"))?
            .ok_or_else(|| IdentityError::MissingKey(format!("device:{hex}:encryption")))?;

        let signing_arr: [u8; SIGNING_KEY_LEN] = signing_bytes
            .try_into()
            .map_err(|_| IdentityError::Crypto("invalid signing key length".into()))?;
        let encryption_arr: [u8; ENCRYPTION_KEY_LEN] = encryption_bytes
            .try_into()
            .map_err(|_| IdentityError::Crypto("invalid encryption key length".into()))?;

        Ok(Self {
            device_id,
            signing_key: SigningKey::from_bytes(&signing_arr),
            encryption_key: X25519StaticSecret::from(encryption_arr),
        })
    }
}

/// Owner trust anchor for a vault.
pub struct OwnerTrust {
    pub vault_id: VaultId,
    pub owner_signing_key: SigningKey,
    pub genesis_manifest: GenesisManifest,
}

impl OwnerTrust {
    /// Return the owner verifying key bytes.
    pub fn owner_pubkey(&self) -> SignPubKey {
        SignPubKey(self.owner_signing_key.verifying_key().to_bytes())
    }

    /// Persist the owner signing key and genesis manifest to secure storage.
    pub fn persist(&self, storage: &dyn SecureStorage) -> Result<(), IdentityError> {
        let hex = self.vault_id.to_hex();
        storage.store(
            &format!("vault:{hex}:owner_signing"),
            &self.owner_signing_key.to_bytes(),
        )?;
        storage.store(
            &format!("vault:{hex}:genesis"),
            &self.genesis_manifest.to_bytes()?,
        )?;
        Ok(())
    }

    /// Load owner trust from secure storage.
    pub fn load(vault_id: VaultId, storage: &dyn SecureStorage) -> Result<Self, IdentityError> {
        let hex = vault_id.to_hex();
        let owner_signing_bytes = storage
            .load(&format!("vault:{hex}:owner_signing"))?
            .ok_or_else(|| IdentityError::MissingKey(format!("vault:{hex}:owner_signing")))?;
        let genesis_bytes = storage
            .load(&format!("vault:{hex}:genesis"))?
            .ok_or_else(|| IdentityError::MissingKey(format!("vault:{hex}:genesis")))?;

        let owner_arr: [u8; SIGNING_KEY_LEN] = owner_signing_bytes
            .try_into()
            .map_err(|_| IdentityError::Crypto("invalid owner signing key length".into()))?;
        let genesis_manifest = GenesisManifest::from_bytes(&genesis_bytes)?;

        Ok(Self {
            vault_id,
            owner_signing_key: SigningKey::from_bytes(&owner_arr),
            genesis_manifest,
        })
    }
}

/// Create a new vault on this device.
///
/// Generates an owner signing key, a device identity, and a genesis manifest
/// signed by the owner. Both the device identity and owner trust are persisted
/// using the supplied secure storage backend.
pub fn create_vault(
    storage: &dyn SecureStorage,
    created_at: Hlc,
) -> Result<(OwnerTrust, DeviceIdentity), IdentityError> {
    let vault_id = VaultId::generate()?;
    let owner_signing_key = generate_signing_key()?;
    let device = DeviceIdentity::generate()?;

    let content = ManifestContent {
        protocol_version: 0,
        membership_version: 0,
        key_epoch: 0,
        vault_id,
        owner_pubkey: SignPubKey(owner_signing_key.verifying_key().to_bytes()),
        owner_management_device_id: device.device_id,
        created_at,
        devices: vec![device.manifest_entry(DeviceStatus::Active)],
    };

    let content_bytes = canonical_content_bytes(&content)?;
    let signature = owner_signing_key.sign(&content_bytes);
    let genesis_manifest = GenesisManifest {
        content,
        signature: SignatureBytes(signature.to_bytes()),
    };

    let owner_trust = OwnerTrust {
        vault_id,
        owner_signing_key,
        genesis_manifest,
    };

    device.persist(storage)?;
    owner_trust.persist(storage)?;

    Ok((owner_trust, device))
}

fn generate_id() -> Result<DeviceId, IdentityError> {
    let mut bytes = [0u8; ID_LEN];
    getrandom::getrandom(&mut bytes).map_err(|e| IdentityError::Entropy(e.to_string()))?;
    Ok(DeviceId(bytes))
}

fn generate_signing_key() -> Result<SigningKey, IdentityError> {
    let mut bytes = [0u8; SIGNING_KEY_LEN];
    getrandom::getrandom(&mut bytes).map_err(|e| IdentityError::Entropy(e.to_string()))?;
    Ok(SigningKey::from_bytes(&bytes))
}

fn generate_encryption_key() -> Result<X25519StaticSecret, IdentityError> {
    let mut bytes = [0u8; ENCRYPTION_KEY_LEN];
    getrandom::getrandom(&mut bytes).map_err(|e| IdentityError::Entropy(e.to_string()))?;
    Ok(X25519StaticSecret::from(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_verify_vault() {
        let storage = InMemorySecureStorage::default();
        let hlc = Hlc {
            wall: 1,
            counter: 0,
            device_id: DeviceId([1; 16]),
        };
        let (owner_trust, device) = create_vault(&storage, hlc).unwrap();

        assert_eq!(
            device.device_id,
            owner_trust
                .genesis_manifest
                .content
                .owner_management_device_id
        );
        assert_eq!(
            owner_trust.owner_pubkey().0,
            owner_trust.genesis_manifest.content.owner_pubkey.0
        );
        owner_trust.genesis_manifest.verify().unwrap();

        // Round-trip through storage.
        let loaded_owner = OwnerTrust::load(owner_trust.vault_id, &storage).unwrap();
        let loaded_device = DeviceIdentity::load(device.device_id, &storage).unwrap();

        assert_eq!(loaded_owner.owner_pubkey().0, owner_trust.owner_pubkey().0);
        assert_eq!(loaded_device.signing_pubkey().0, device.signing_pubkey().0);
        loaded_owner.genesis_manifest.verify().unwrap();
    }

    #[test]
    fn detect_tampered_manifest() {
        let storage = InMemorySecureStorage::default();
        let hlc = Hlc {
            wall: 2,
            counter: 0,
            device_id: DeviceId([2; 16]),
        };
        let (mut owner_trust, _) = create_vault(&storage, hlc).unwrap();

        // Mutate membership version after signing.
        owner_trust.genesis_manifest.content.membership_version = 99;
        assert!(owner_trust.genesis_manifest.verify().is_err());
    }

    #[test]
    fn genesis_manifest_round_trip() {
        let storage = InMemorySecureStorage::default();
        let hlc = Hlc {
            wall: 3,
            counter: 0,
            device_id: DeviceId([3; 16]),
        };
        let (owner_trust, _) = create_vault(&storage, hlc).unwrap();

        let bytes = owner_trust.genesis_manifest.to_bytes().unwrap();
        let parsed = GenesisManifest::from_bytes(&bytes).unwrap();
        assert_eq!(parsed, owner_trust.genesis_manifest);
        parsed.verify().unwrap();
    }
}
