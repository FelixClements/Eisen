//! Epoch-root key and local-store AEAD encryption (P1.05).
//!
//! Derives a per-epoch AES-256-GCM key from a 32-byte master root using
//! HKDF-SHA256, encrypts local snapshots with a deterministic nonce built from
//! a monotonic 64-bit counter, and enforces a fail-closed counter policy so
//! that snapshot encryption never reuses a counter within an epoch.

use crate::identity::SecureStorage;
use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use hkdf::Hkdf;
use sha2::Sha256;

const KEY_LEN: usize = 32;
const NONCE_LEN: usize = 12;
const COUNTER_KEY: &str = "snapshot:counter";
const CIPHERTEXT_KEY: &str = "snapshot:ciphertext";
const SNAPSHOT_INFO_PREFIX: &[u8] = b"eisen-snapshot-v1";

/// Errors from epoch-key and snapshot operations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EpochError {
    Entropy(String),
    Derivation(String),
    Encryption(String),
    Decryption(String),
    Storage(String),
    CounterViolation,
    CounterExhausted,
    InvalidCiphertext,
}

impl std::fmt::Display for EpochError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EpochError::Entropy(s) => write!(f, "entropy error: {s}"),
            EpochError::Derivation(s) => write!(f, "key derivation error: {s}"),
            EpochError::Encryption(s) => write!(f, "encryption error: {s}"),
            EpochError::Decryption(s) => write!(f, "decryption error: {s}"),
            EpochError::Storage(s) => write!(f, "storage error: {s}"),
            EpochError::CounterViolation => {
                write!(f, "snapshot counter is not monotonically increasing")
            }
            EpochError::CounterExhausted => write!(f, "snapshot counter exhausted"),
            EpochError::InvalidCiphertext => write!(f, "ciphertext encoding is invalid"),
        }
    }
}

impl std::error::Error for EpochError {}

impl From<crate::identity::IdentityError> for EpochError {
    fn from(e: crate::identity::IdentityError) -> Self {
        EpochError::Storage(e.to_string())
    }
}

/// A 32-byte master root from which per-epoch keys are derived.
pub struct EpochRoot([u8; KEY_LEN]);

impl EpochRoot {
    /// Generate a random master root from OS entropy.
    pub fn generate() -> Result<Self, EpochError> {
        let mut bytes = [0u8; KEY_LEN];
        getrandom::getrandom(&mut bytes).map_err(|e| EpochError::Entropy(e.to_string()))?;
        Ok(Self(bytes))
    }

    /// Derive the AES-256-GCM key for the given epoch.
    pub fn derive(&self, epoch: u64) -> Result<EpochKey, EpochError> {
        let hk = Hkdf::<Sha256>::new(None, &self.0);
        let mut info = Vec::with_capacity(SNAPSHOT_INFO_PREFIX.len() + 8);
        info.extend_from_slice(SNAPSHOT_INFO_PREFIX);
        info.extend_from_slice(&epoch.to_be_bytes());

        let mut okm = [0u8; KEY_LEN];
        hk.expand(&info, &mut okm)
            .map_err(|e| EpochError::Derivation(e.to_string()))?;

        Ok(EpochKey { epoch, key: okm })
    }

    /// Expose the raw root bytes for storage (platform secure storage only).
    pub fn as_bytes(&self) -> &[u8; KEY_LEN] {
        &self.0
    }

    /// Restore from raw bytes.
    pub fn from_bytes(bytes: [u8; KEY_LEN]) -> Self {
        Self(bytes)
    }
}

/// A per-epoch AES-256-GCM key.
pub struct EpochKey {
    pub epoch: u64,
    pub key: [u8; KEY_LEN],
}

impl EpochKey {}

/// Encrypted snapshot with its counter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AeadSnapshot {
    pub counter: u64,
    pub nonce: [u8; NONCE_LEN],
    pub payload: Vec<u8>,
}

impl AeadSnapshot {
    /// Wire/storage encoding: `counter (8) || nonce (12) || payload`.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(8 + NONCE_LEN + self.payload.len());
        out.extend_from_slice(&self.counter.to_be_bytes());
        out.extend_from_slice(&self.nonce);
        out.extend_from_slice(&self.payload);
        out
    }

    /// Parse from the wire/storage encoding.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, EpochError> {
        if bytes.len() < 8 + NONCE_LEN {
            return Err(EpochError::InvalidCiphertext);
        }
        let counter = u64::from_be_bytes(bytes[0..8].try_into().unwrap());
        let mut nonce = [0u8; NONCE_LEN];
        nonce.copy_from_slice(&bytes[8..8 + NONCE_LEN]);
        let payload = bytes[8 + NONCE_LEN..].to_vec();
        Ok(Self {
            counter,
            nonce,
            payload,
        })
    }

    /// Encrypt a plaintext snapshot under the epoch key with the given counter.
    ///
    /// The counter is part of the AEAD nonce; callers must ensure it is unique
    /// per key. `SnapshotStore` enforces this.
    pub fn encrypt(key: &EpochKey, counter: u64, plaintext: &[u8]) -> Result<Self, EpochError> {
        let cipher = Aes256Gcm::new_from_slice(&key.key)
            .map_err(|e| EpochError::Encryption(e.to_string()))?;
        let nonce = Self::nonce(key.epoch, counter);
        let nonce = Nonce::from_slice(&nonce);
        let payload = cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| EpochError::Encryption(e.to_string()))?;
        Ok(Self {
            counter,
            nonce: Self::nonce(key.epoch, counter),
            payload,
        })
    }

    /// Decrypt a snapshot under the epoch key. The counter/nonce are carried in
    /// the snapshot itself.
    pub fn decrypt(&self, key: &EpochKey) -> Result<Vec<u8>, EpochError> {
        let cipher = Aes256Gcm::new_from_slice(&key.key)
            .map_err(|e| EpochError::Decryption(e.to_string()))?;
        let nonce = Nonce::from_slice(&self.nonce);
        cipher
            .decrypt(nonce, self.payload.as_ref())
            .map_err(|_| EpochError::Decryption("authentication failed".into()))
    }

    fn nonce(epoch: u64, counter: u64) -> [u8; NONCE_LEN] {
        let mut n = [0u8; NONCE_LEN];
        // Include the epoch in the upper 4 bytes and the upper 32 bits of the
        // counter in the lower 8 bytes. Since the epoch key is already unique to
        // the epoch, this is defensive in depth.
        n[0..4].copy_from_slice(&epoch.to_be_bytes()[4..8]);
        n[4..].copy_from_slice(&counter.to_be_bytes());
        n
    }
}

/// Local snapshot store that enforces a monotonic counter.
pub struct SnapshotStore<'a> {
    storage: &'a dyn SecureStorage,
    epoch_key: EpochKey,
    counter_key: String,
    ciphertext_key: String,
}

impl<'a> SnapshotStore<'a> {
    /// Create a store bound to an epoch key and default snapshot keys.
    pub fn new(storage: &'a dyn SecureStorage, epoch_key: EpochKey) -> Self {
        Self {
            storage,
            epoch_key,
            counter_key: COUNTER_KEY.into(),
            ciphertext_key: CIPHERTEXT_KEY.into(),
        }
    }

    /// Create a store with custom keys (useful for tests/profiles).
    pub fn with_keys(
        storage: &'a dyn SecureStorage,
        epoch_key: EpochKey,
        counter_key: String,
        ciphertext_key: String,
    ) -> Self {
        Self {
            storage,
            epoch_key,
            counter_key,
            ciphertext_key,
        }
    }

    /// Read the persisted counter, defaulting to 1 less than `counter` when none.
    fn current_counter(&self) -> Result<u64, EpochError> {
        match self.storage.load(&self.counter_key)? {
            Some(bytes) if bytes.len() == 8 => Ok(u64::from_be_bytes(bytes.try_into().unwrap())),
            Some(_) => Err(EpochError::Storage("corrupt counter".into())),
            None => Ok(1u64.saturating_sub(1)),
        }
    }

    /// Encrypt and persist a new snapshot. Fails unless the new counter is
    /// strictly greater than the persisted counter (fail-closed).
    pub fn store(&self, plaintext: &[u8]) -> Result<AeadSnapshot, EpochError> {
        let current = self.current_counter()?;
        let next = current.checked_add(1).ok_or(EpochError::CounterExhausted)?;

        if current != u64::MAX && current.saturating_add(1) != next {
            // Defensive: checked_add already handles u64::MAX.
        }

        let snapshot = AeadSnapshot::encrypt(&self.epoch_key, next, plaintext)?;
        self.storage.store(&self.counter_key, &next.to_be_bytes())?;
        self.storage
            .store(&self.ciphertext_key, &snapshot.to_bytes())?;
        Ok(snapshot)
    }

    /// Load and decrypt the persisted snapshot.
    pub fn load(&self) -> Result<Vec<u8>, EpochError> {
        let bytes = self
            .storage
            .load(&self.ciphertext_key)?
            .ok_or_else(|| EpochError::Storage("missing snapshot".into()))?;
        let snapshot = AeadSnapshot::from_bytes(&bytes)?;
        snapshot.decrypt(&self.epoch_key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::InMemorySecureStorage;

    #[test]
    fn derive_and_encrypt_round_trip() {
        let root = EpochRoot::generate().unwrap();
        let key = root.derive(7).unwrap();
        let pt = b"local task snapshot";
        let snapshot = AeadSnapshot::encrypt(&key, 1, pt).unwrap();
        let decrypted = snapshot.decrypt(&key).unwrap();
        assert_eq!(decrypted, pt);
        assert_eq!(snapshot.counter, 1);
        assert_eq!(snapshot.nonce, AeadSnapshot::nonce(7, 1));
    }

    #[test]
    fn snapshot_store_enforces_monotonic_counter() {
        let storage = InMemorySecureStorage::default();
        let root = EpochRoot::generate().unwrap();
        let key = root.derive(0).unwrap();
        let store = SnapshotStore::new(&storage, key);

        let pt1 = b"first";
        let snap1 = store.store(pt1).unwrap();
        assert_eq!(snap1.counter, 1);

        let pt2 = b"second";
        let snap2 = store.store(pt2).unwrap();
        assert_eq!(snap2.counter, 2);

        assert_eq!(store.load().unwrap(), pt2);
    }

    #[test]
    fn tampered_ciphertext_fails_authentication() {
        let root = EpochRoot::generate().unwrap();
        let key = root.derive(1).unwrap();
        let mut snapshot = AeadSnapshot::encrypt(&key, 1, b"hello").unwrap();
        snapshot.payload[0] ^= 0xff;
        assert!(snapshot.decrypt(&key).is_err());
    }

    #[test]
    fn different_epochs_derive_different_keys() {
        let root = EpochRoot::generate().unwrap();
        let k1 = root.derive(0).unwrap();
        let k2 = root.derive(1).unwrap();
        assert_ne!(k1.key, k2.key);
    }

    #[test]
    fn aead_snapshot_round_trips_bytes() {
        let root = EpochRoot::generate().unwrap();
        let key = root.derive(2).unwrap();
        let snapshot = AeadSnapshot::encrypt(&key, 42, b"payload").unwrap();
        let bytes = snapshot.to_bytes();
        let parsed = AeadSnapshot::from_bytes(&bytes).unwrap();
        assert_eq!(parsed, snapshot);
        assert_eq!(parsed.decrypt(&key).unwrap(), b"payload");
    }
}
