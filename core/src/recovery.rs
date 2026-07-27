//! User-held recovery package (P1.14).
//!
//! Creates and restores a passphrase-encrypted recovery package containing the
//! owner signing key, retained epoch roots, genesis manifest, manifest chain,
//! device membership, and a non-secret locator. Argon2id derives the wrapping
//! key; AES-256-GCM encrypts the keyring and trust sections with AAD binding
//! the package version, vault ID, Argon2id parameters, and section name.

use crate::canonical;
use crate::epoch::EpochRoot;
use crate::identity::{
    DeviceEntry, DeviceIdentity, GenesisManifest, IdentityError, OwnerTrust, VaultId,
};
use crate::manifest::ManifestChain;
use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use argon2::{Algorithm, Argon2, Params, Version};
use cbor2::Value;
use ed25519_dalek::SigningKey;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

const RECOVERY_MAGIC: &[u8] = b"EISEN-RECOVERY";
const VERSION: u32 = 1;
const SALT_LEN: usize = 16;
const KEY_LEN: usize = 32;
const NONCE_LEN: usize = 12;
const CHECKSUM_LEN: usize = 32;
const U32_LEN: usize = 4;
const U64_LEN: usize = 8;
const VAULT_ID_LEN: usize = 16;

/// Errors from recovery package creation or restore.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecoveryError {
    Argon(String),
    Crypto(String),
    Encode(String),
    Decode(String),
    Integrity(String),
    Identity(IdentityError),
    WrongPassphrase,
}

impl std::fmt::Display for RecoveryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RecoveryError::Argon(s) => write!(f, "argon2 error: {s}"),
            RecoveryError::Crypto(s) => write!(f, "recovery crypto error: {s}"),
            RecoveryError::Encode(s) => write!(f, "recovery encode error: {s}"),
            RecoveryError::Decode(s) => write!(f, "recovery decode error: {s}"),
            RecoveryError::Integrity(s) => write!(f, "recovery integrity error: {s}"),
            RecoveryError::Identity(e) => write!(f, "recovery identity error: {e}"),
            RecoveryError::WrongPassphrase => write!(f, "wrong passphrase or tampered package"),
        }
    }
}

impl std::error::Error for RecoveryError {}

impl From<IdentityError> for RecoveryError {
    fn from(e: IdentityError) -> Self {
        RecoveryError::Identity(e)
    }
}

impl From<crate::canonical::CanonError> for RecoveryError {
    fn from(e: crate::canonical::CanonError) -> Self {
        RecoveryError::Encode(e.to_string())
    }
}

impl From<cbor2::ser::Error> for RecoveryError {
    fn from(e: cbor2::ser::Error) -> Self {
        RecoveryError::Encode(e.to_string())
    }
}

impl From<cbor2::de::Error> for RecoveryError {
    fn from(e: cbor2::de::Error) -> Self {
        RecoveryError::Decode(e.to_string())
    }
}

impl From<cbor2::value::Error> for RecoveryError {
    fn from(e: cbor2::value::Error) -> Self {
        RecoveryError::Encode(e.to_string())
    }
}

impl From<argon2::Error> for RecoveryError {
    fn from(e: argon2::Error) -> Self {
        RecoveryError::Argon(e.to_string())
    }
}

/// Argon2id cost profile for key derivation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArgonProfile {
    /// Memory cost in kibibytes.
    pub m_cost: u32,
    /// Number of iterations.
    pub t_cost: u32,
    /// Degree of parallelism.
    pub p_cost: u32,
}

impl ArgonProfile {
    /// Mobile profile: 19 MiB, t=2, p=1.
    pub const fn mobile() -> Self {
        Self {
            m_cost: 19 * 1024,
            t_cost: 2,
            p_cost: 1,
        }
    }

    /// Desktop profile: 64 MiB, t=3, p=4.
    pub const fn desktop() -> Self {
        Self {
            m_cost: 64 * 1024,
            t_cost: 3,
            p_cost: 4,
        }
    }

    /// Low-cost profile for tests.
    pub const fn test() -> Self {
        Self {
            m_cost: 64,
            t_cost: 1,
            p_cost: 1,
        }
    }
}

/// Recovered vault state plus a fresh device identity.
pub struct RecoveryState {
    /// Recovered owner trust anchor.
    pub owner_trust: OwnerTrust,
    /// Retained epoch root keys (epoch -> root).
    pub epoch_roots: BTreeMap<u64, EpochRoot>,
    /// Verified manifest chain.
    pub manifest_chain: ManifestChain,
    /// Device membership list from the current epoch.
    pub devices: Vec<DeviceEntry>,
    /// Non-secret locator.
    pub locator: String,
    /// Freshly generated device identity for the new device.
    pub new_device: DeviceIdentity,
}

/// A user-held passphrase-encrypted recovery package.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecoveryPackage {
    /// Vault identifier (non-secret locator binding).
    pub vault_id: VaultId,
    /// Argon2id parameters used to derive the wrapping key.
    pub profile: ArgonProfile,
    /// Random KDF salt.
    pub salt: [u8; SALT_LEN],
    /// Encrypted keyring ciphertext.
    pub keyring_ciphertext: Vec<u8>,
    /// Encrypted trust contents ciphertext.
    pub trust_ciphertext: Vec<u8>,
    /// Non-secret locator hint.
    pub locator: String,
    /// SHA-256 checksum over the serialized preceding sections.
    pub checksum: [u8; CHECKSUM_LEN],
}

impl RecoveryPackage {
    /// Create a new recovery package.
    ///
    /// `locator` is an optional user-visible hint; if `None`, the first 8 hex
    /// characters of the vault ID are used.
    pub fn create(
        passphrase: &[u8],
        owner_trust: &OwnerTrust,
        epoch_roots: &BTreeMap<u64, EpochRoot>,
        manifest_chain: &ManifestChain,
        devices: &[DeviceEntry],
        locator: Option<String>,
        profile: ArgonProfile,
    ) -> Result<Self, RecoveryError> {
        let mut salt = [0u8; SALT_LEN];
        getrandom::getrandom(&mut salt).map_err(|e| RecoveryError::Crypto(e.to_string()))?;
        let wrapping_key = derive_key(passphrase, &salt, &profile)?;

        let locator = locator.unwrap_or_else(|| {
            let hex = owner_trust.vault_id.to_hex();
            hex.chars().take(8).collect()
        });

        let keyring_value = keyring_to_value(owner_trust, epoch_roots)?;
        let keyring_plaintext = canonical::encode(&keyring_value)?;
        let aad_keyring = aad_value(&owner_trust.vault_id, &profile, "keyring")?;
        let aad_keyring_bytes = canonical::encode(&aad_keyring)?;
        let keyring_ciphertext =
            aes_encrypt(&wrapping_key, 0, 1, &keyring_plaintext, &aad_keyring_bytes)?;

        let trust_value = trust_to_value(manifest_chain, devices, &locator)?;
        let trust_plaintext = canonical::encode(&trust_value)?;
        let aad_trust = aad_value(&owner_trust.vault_id, &profile, "trust")?;
        let aad_trust_bytes = canonical::encode(&aad_trust)?;
        let trust_ciphertext =
            aes_encrypt(&wrapping_key, 1, 1, &trust_plaintext, &aad_trust_bytes)?;

        let mut prefix = Vec::new();
        prefix.extend_from_slice(RECOVERY_MAGIC);
        prefix.extend_from_slice(&VERSION.to_be_bytes());
        prefix.extend_from_slice(&owner_trust.vault_id.0);
        prefix.extend_from_slice(&profile.m_cost.to_be_bytes());
        prefix.extend_from_slice(&profile.t_cost.to_be_bytes());
        prefix.extend_from_slice(&profile.p_cost.to_be_bytes());
        prefix.extend_from_slice(&salt);
        prefix.extend_from_slice(&(keyring_ciphertext.len() as u64).to_be_bytes());
        prefix.extend_from_slice(&keyring_ciphertext);
        prefix.extend_from_slice(&(trust_ciphertext.len() as u64).to_be_bytes());
        prefix.extend_from_slice(&trust_ciphertext);
        prefix.extend_from_slice(&(locator.len() as u64).to_be_bytes());
        prefix.extend_from_slice(locator.as_bytes());
        let checksum: [u8; CHECKSUM_LEN] = Sha256::digest(&prefix).into();

        Ok(Self {
            vault_id: owner_trust.vault_id,
            profile,
            salt,
            keyring_ciphertext,
            trust_ciphertext,
            locator,
            checksum,
        })
    }

    /// Parse a recovery package from its wire/file encoding and verify the
    /// structural checksum. This does not require the passphrase.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, RecoveryError> {
        let mut offset = 0;
        let magic = read_bytes(bytes, &mut offset, RECOVERY_MAGIC.len())?;
        if magic != RECOVERY_MAGIC {
            return Err(RecoveryError::Integrity("bad magic".into()));
        }
        let version = read_u32(bytes, &mut offset)?;
        if version != VERSION {
            return Err(RecoveryError::Integrity(
                "unsupported recovery version".into(),
            ));
        }
        let vault_id_bytes = read_bytes(bytes, &mut offset, VAULT_ID_LEN)?;
        let vault_id = VaultId(vault_id_bytes.try_into().unwrap());
        let m_cost = read_u32(bytes, &mut offset)?;
        let t_cost = read_u32(bytes, &mut offset)?;
        let p_cost = read_u32(bytes, &mut offset)?;
        let profile = ArgonProfile {
            m_cost,
            t_cost,
            p_cost,
        };
        let salt = read_bytes(bytes, &mut offset, SALT_LEN)?;
        let salt: [u8; SALT_LEN] = salt.try_into().unwrap();
        let keyring_len = read_u64(bytes, &mut offset)? as usize;
        let keyring_ciphertext = read_bytes(bytes, &mut offset, keyring_len)?.to_vec();
        let trust_len = read_u64(bytes, &mut offset)? as usize;
        let trust_ciphertext = read_bytes(bytes, &mut offset, trust_len)?.to_vec();
        let locator_len = read_u64(bytes, &mut offset)? as usize;
        let locator_bytes = read_bytes(bytes, &mut offset, locator_len)?;
        let locator = String::from_utf8(locator_bytes.to_vec())
            .map_err(|_| RecoveryError::Decode("locator is not valid UTF-8".into()))?;

        if bytes.len() - offset != CHECKSUM_LEN {
            return Err(RecoveryError::Integrity(
                "trailing data or missing checksum".into(),
            ));
        }
        let checksum: [u8; CHECKSUM_LEN] = bytes[offset..].try_into().unwrap();
        let expected = Sha256::digest(&bytes[..offset]);
        if expected.as_slice() != checksum.as_slice() {
            return Err(RecoveryError::Integrity("checksum mismatch".into()));
        }

        Ok(Self {
            vault_id,
            profile,
            salt,
            keyring_ciphertext,
            trust_ciphertext,
            locator,
            checksum,
        })
    }

    /// Serialize the package to its wire/file encoding.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(
            RECOVERY_MAGIC.len()
                + 4 * U32_LEN
                + VAULT_ID_LEN
                + SALT_LEN
                + 3 * U64_LEN
                + self.keyring_ciphertext.len()
                + self.trust_ciphertext.len()
                + self.locator.len()
                + CHECKSUM_LEN,
        );
        out.extend_from_slice(RECOVERY_MAGIC);
        out.extend_from_slice(&VERSION.to_be_bytes());
        out.extend_from_slice(&self.vault_id.0);
        out.extend_from_slice(&self.profile.m_cost.to_be_bytes());
        out.extend_from_slice(&self.profile.t_cost.to_be_bytes());
        out.extend_from_slice(&self.profile.p_cost.to_be_bytes());
        out.extend_from_slice(&self.salt);
        out.extend_from_slice(&(self.keyring_ciphertext.len() as u64).to_be_bytes());
        out.extend_from_slice(&self.keyring_ciphertext);
        out.extend_from_slice(&(self.trust_ciphertext.len() as u64).to_be_bytes());
        out.extend_from_slice(&self.trust_ciphertext);
        out.extend_from_slice(&(self.locator.len() as u64).to_be_bytes());
        out.extend_from_slice(self.locator.as_bytes());
        out.extend_from_slice(&self.checksum);
        out
    }

    /// Restore the vault from this package using the user's passphrase.
    ///
    /// Generates a fresh device identity and nonce domain as required for a
    /// new device.
    pub fn restore(&self, passphrase: &[u8]) -> Result<RecoveryState, RecoveryError> {
        let wrapping_key = derive_key(passphrase, &self.salt, &self.profile)?;

        let aad_keyring = aad_value(&self.vault_id, &self.profile, "keyring")?;
        let keyring_plaintext = aes_decrypt(
            &wrapping_key,
            0,
            1,
            &self.keyring_ciphertext,
            &canonical::encode(&aad_keyring)?,
        )
        .map_err(|_| RecoveryError::WrongPassphrase)?;
        let keyring_value = canonical::parse(&keyring_plaintext, &canonical::Limits::default())?;
        let (owner_trust, epoch_roots) = keyring_from_value(&keyring_value)?;

        let aad_trust = aad_value(&self.vault_id, &self.profile, "trust")?;
        let trust_plaintext = aes_decrypt(
            &wrapping_key,
            1,
            1,
            &self.trust_ciphertext,
            &canonical::encode(&aad_trust)?,
        )
        .map_err(|_| RecoveryError::WrongPassphrase)?;
        let trust_value = canonical::parse(&trust_plaintext, &canonical::Limits::default())?;
        let (manifest_chain, devices, locator) = trust_from_value(&trust_value)?;

        let new_device =
            DeviceIdentity::generate().map_err(|e| RecoveryError::Crypto(e.to_string()))?;

        Ok(RecoveryState {
            owner_trust,
            epoch_roots,
            manifest_chain,
            devices,
            locator,
            new_device,
        })
    }
}

fn derive_key(
    passphrase: &[u8],
    salt: &[u8; SALT_LEN],
    profile: &ArgonProfile,
) -> Result<[u8; KEY_LEN], RecoveryError> {
    let params = Params::new(
        profile.m_cost,
        profile.t_cost,
        profile.p_cost,
        Some(KEY_LEN),
    )?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut out = [0u8; KEY_LEN];
    argon2.hash_password_into(passphrase, salt, &mut out)?;
    Ok(out)
}

fn section_nonce(section_index: u32, counter: u32) -> [u8; NONCE_LEN] {
    let mut n = [0u8; NONCE_LEN];
    n[0..4].copy_from_slice(&VERSION.to_be_bytes());
    n[4..8].copy_from_slice(&section_index.to_be_bytes());
    n[8..12].copy_from_slice(&counter.to_be_bytes());
    n
}

fn aes_encrypt(
    key: &[u8; KEY_LEN],
    section_index: u32,
    counter: u32,
    plaintext: &[u8],
    aad: &[u8],
) -> Result<Vec<u8>, RecoveryError> {
    let cipher =
        Aes256Gcm::new_from_slice(key).map_err(|e| RecoveryError::Crypto(e.to_string()))?;
    let nonce = section_nonce(section_index, counter);
    let nonce = Nonce::from_slice(&nonce);
    cipher
        .encrypt(
            nonce,
            Payload {
                msg: plaintext,
                aad,
            },
        )
        .map_err(|e| RecoveryError::Crypto(e.to_string()))
}

fn aes_decrypt(
    key: &[u8; KEY_LEN],
    section_index: u32,
    counter: u32,
    ciphertext: &[u8],
    aad: &[u8],
) -> Result<Vec<u8>, RecoveryError> {
    let cipher =
        Aes256Gcm::new_from_slice(key).map_err(|e| RecoveryError::Crypto(e.to_string()))?;
    let nonce = section_nonce(section_index, counter);
    let nonce = Nonce::from_slice(&nonce);
    cipher
        .decrypt(
            nonce,
            Payload {
                msg: ciphertext,
                aad,
            },
        )
        .map_err(|e| RecoveryError::Crypto(e.to_string()))
}

fn aad_value(
    vault_id: &VaultId,
    profile: &ArgonProfile,
    section: &str,
) -> Result<Value, RecoveryError> {
    Ok(Value::Map(vec![
        (text("m_cost"), u64_value(profile.m_cost as u64)),
        (text("p_cost"), u64_value(profile.p_cost as u64)),
        (text("section"), text(section)),
        (text("t_cost"), u64_value(profile.t_cost as u64)),
        (text("vault_id"), bytes_value(&vault_id.0)),
        (text("version"), u64_value(VERSION as u64)),
    ]))
}

fn keyring_to_value(
    owner_trust: &OwnerTrust,
    epoch_roots: &BTreeMap<u64, EpochRoot>,
) -> Result<Value, RecoveryError> {
    let mut roots = Vec::with_capacity(epoch_roots.len());
    for (epoch, root) in epoch_roots {
        roots.push((u64_value(*epoch), bytes_value(root.as_bytes().as_ref())));
    }
    Ok(Value::Map(vec![
        (text("epoch_roots"), Value::Map(roots)),
        (
            text("genesis_manifest"),
            bytes_value(&owner_trust.genesis_manifest.to_bytes()?),
        ),
        (
            text("owner_signing_key"),
            bytes_value(&owner_trust.owner_signing_key.to_bytes()),
        ),
    ]))
}

fn keyring_from_value(v: &Value) -> Result<(OwnerTrust, BTreeMap<u64, EpochRoot>), RecoveryError> {
    let map = match v {
        Value::Map(m) => m,
        _ => return Err(RecoveryError::Decode("keyring is not a map".into())),
    };
    let owner_key = bytes32_field(map, "owner_signing_key")?;
    let genesis_bytes = bytes_field(map, "genesis_manifest")?;
    let genesis = GenesisManifest::from_bytes(&genesis_bytes)?;
    let owner_trust = OwnerTrust {
        vault_id: genesis.content.vault_id,
        owner_signing_key: SigningKey::from_bytes(&owner_key),
        genesis_manifest: genesis,
    };

    let mut epoch_roots = BTreeMap::new();
    match get_field(map, "epoch_roots")? {
        Value::Map(items) => {
            for (k, v) in items {
                let epoch = u64_from_value(&k)?;
                let root_bytes = match v {
                    Value::Bytes(b) if b.len() == KEY_LEN => b,
                    _ => return Err(RecoveryError::Decode("epoch root must be 32 bytes".into())),
                };
                let arr: [u8; KEY_LEN] = root_bytes.as_slice().try_into().unwrap();
                epoch_roots.insert(epoch, EpochRoot::from_bytes(arr));
            }
        }
        _ => return Err(RecoveryError::Decode("epoch_roots is not a map".into())),
    }

    Ok((owner_trust, epoch_roots))
}

fn trust_to_value(
    manifest_chain: &ManifestChain,
    devices: &[DeviceEntry],
    locator: &str,
) -> Result<Value, RecoveryError> {
    let manifests: Result<Vec<Value>, RecoveryError> = manifest_chain
        .iter()
        .map(|m| Ok(bytes_value(&m.to_bytes()?)))
        .collect();
    let devices: Result<Vec<Value>, RecoveryError> = devices
        .iter()
        .map(|d| Ok(bytes_value(&cbor2::to_canonical_vec(d)?)))
        .collect();
    Ok(Value::Map(vec![
        (text("devices"), Value::Array(devices?)),
        (text("locator"), text(locator)),
        (text("manifests"), Value::Array(manifests?)),
    ]))
}

fn trust_from_value(v: &Value) -> Result<(ManifestChain, Vec<DeviceEntry>, String), RecoveryError> {
    let map = match v {
        Value::Map(m) => m,
        _ => return Err(RecoveryError::Decode("trust is not a map".into())),
    };
    let locator = text_field(map, "locator")?;

    let manifests_value = get_field(map, "manifests")?;
    let mut manifests = Vec::new();
    match manifests_value {
        Value::Array(arr) => {
            for v in arr {
                let bytes = match v {
                    Value::Bytes(b) => b,
                    _ => return Err(RecoveryError::Decode("manifest must be bytes".into())),
                };
                manifests.push(GenesisManifest::from_bytes(&bytes)?);
            }
        }
        _ => return Err(RecoveryError::Decode("manifests is not an array".into())),
    }
    if manifests.is_empty() {
        return Err(RecoveryError::Decode("manifest chain is empty".into()));
    }
    let mut chain = ManifestChain::new(manifests[0].clone())?;
    for m in &manifests[1..] {
        chain.push(m.clone())?;
    }

    let devices_value = get_field(map, "devices")?;
    let mut devices = Vec::new();
    match devices_value {
        Value::Array(arr) => {
            for v in arr {
                let bytes = match v {
                    Value::Bytes(b) => b,
                    _ => return Err(RecoveryError::Decode("device must be bytes".into())),
                };
                devices.push(cbor2::from_slice::<DeviceEntry>(&bytes)?);
            }
        }
        _ => return Err(RecoveryError::Decode("devices is not an array".into())),
    }

    Ok((chain, devices, locator))
}

fn read_u32(bytes: &[u8], offset: &mut usize) -> Result<u32, RecoveryError> {
    let b = read_bytes(bytes, offset, U32_LEN)?;
    Ok(u32::from_be_bytes(b.try_into().unwrap()))
}

fn read_u64(bytes: &[u8], offset: &mut usize) -> Result<u64, RecoveryError> {
    let b = read_bytes(bytes, offset, U64_LEN)?;
    Ok(u64::from_be_bytes(b.try_into().unwrap()))
}

fn read_bytes<'a>(
    bytes: &'a [u8],
    offset: &mut usize,
    len: usize,
) -> Result<&'a [u8], RecoveryError> {
    if bytes.len() < *offset + len {
        return Err(RecoveryError::Integrity("truncated package".into()));
    }
    let out = &bytes[*offset..*offset + len];
    *offset += len;
    Ok(out)
}

fn text(s: &str) -> Value {
    Value::Text(s.into())
}

fn bytes_value(b: &[u8]) -> Value {
    Value::Bytes(b.to_vec())
}

fn u64_value(n: u64) -> Value {
    Value::Integer(cbor2::value::Integer::from(n))
}

fn get_field(map: &[(Value, Value)], key: &str) -> Result<Value, RecoveryError> {
    map.iter()
        .find(|(k, _)| matches!(k, Value::Text(t) if t == key))
        .map(|(_, v)| v.clone())
        .ok_or_else(|| RecoveryError::Decode(format!("missing field {key}")))
}

fn bytes_field(map: &[(Value, Value)], key: &str) -> Result<Vec<u8>, RecoveryError> {
    match get_field(map, key)? {
        Value::Bytes(b) => Ok(b),
        _ => Err(RecoveryError::Decode(format!("{key}: expected bytes"))),
    }
}

fn bytes32_field(map: &[(Value, Value)], key: &str) -> Result<[u8; 32], RecoveryError> {
    match get_field(map, key)? {
        Value::Bytes(b) if b.len() == 32 => {
            let arr: [u8; 32] = b.as_slice().try_into().unwrap();
            Ok(arr)
        }
        _ => Err(RecoveryError::Decode(format!("{key}: expected 32 bytes"))),
    }
}

fn text_field(map: &[(Value, Value)], key: &str) -> Result<String, RecoveryError> {
    match get_field(map, key)? {
        Value::Text(s) => Ok(s),
        _ => Err(RecoveryError::Decode(format!("{key}: expected text"))),
    }
}

fn u64_from_value(v: &Value) -> Result<u64, RecoveryError> {
    match v {
        Value::Integer(i) => {
            let n: i128 = (*i).into();
            if n < 0 {
                return Err(RecoveryError::Decode("negative integer".into()));
            }
            u64::try_from(n).map_err(|_| RecoveryError::Decode("integer too large for u64".into()))
        }
        _ => Err(RecoveryError::Decode("expected integer".into())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::InMemorySecureStorage;
    use crate::Hlc;

    fn make_fixture() -> (OwnerTrust, DeviceIdentity, ManifestChain) {
        let storage = InMemorySecureStorage::default();
        let hlc = Hlc {
            wall: 1,
            counter: 0,
            device_id: crate::DeviceId([0u8; 16]),
        };
        let (owner_trust, device) = crate::identity::create_vault(&storage, hlc).unwrap();
        let chain = ManifestChain::new(owner_trust.genesis_manifest.clone()).unwrap();
        (owner_trust, device, chain)
    }

    #[test]
    fn recovery_round_trip() {
        let (owner_trust, _device, chain) = make_fixture();
        let mut roots = BTreeMap::new();
        let root = crate::epoch::EpochRoot::generate().unwrap();
        roots.insert(0, root);

        let package = RecoveryPackage::create(
            b"correct horse battery staple",
            &owner_trust,
            &roots,
            &chain,
            &owner_trust.genesis_manifest.content.devices,
            None,
            ArgonProfile::test(),
        )
        .unwrap();

        let bytes = package.to_bytes();
        let parsed = RecoveryPackage::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.vault_id, owner_trust.vault_id);
        assert_eq!(parsed.locator, owner_trust.vault_id.to_hex()[..8]);

        let state = parsed.restore(b"correct horse battery staple").unwrap();
        assert_eq!(state.owner_trust.vault_id, owner_trust.vault_id);
        assert_eq!(state.owner_trust.owner_pubkey(), owner_trust.owner_pubkey());
        assert_eq!(state.manifest_chain.len(), 1);
        assert_eq!(state.devices.len(), 1);
        assert!(state.epoch_roots.contains_key(&0));
    }

    #[test]
    fn wrong_passphrase_fails() {
        let (owner_trust, _device, chain) = make_fixture();
        let mut roots = BTreeMap::new();
        let root = crate::epoch::EpochRoot::generate().unwrap();
        roots.insert(0, root);

        let package = RecoveryPackage::create(
            b"right",
            &owner_trust,
            &roots,
            &chain,
            &owner_trust.genesis_manifest.content.devices,
            None,
            ArgonProfile::test(),
        )
        .unwrap();

        let result = package.restore(b"wrong");
        assert!(matches!(result, Err(RecoveryError::WrongPassphrase)));
    }

    #[test]
    fn checksum_failure_rejected() {
        let (owner_trust, _device, chain) = make_fixture();
        let mut roots = BTreeMap::new();
        let root = crate::epoch::EpochRoot::generate().unwrap();
        roots.insert(0, root);

        let mut package = RecoveryPackage::create(
            b"secret",
            &owner_trust,
            &roots,
            &chain,
            &owner_trust.genesis_manifest.content.devices,
            None,
            ArgonProfile::test(),
        )
        .unwrap();

        package.keyring_ciphertext[0] ^= 0xff;
        let bytes = package.to_bytes();
        let err = RecoveryPackage::from_bytes(&bytes).unwrap_err();
        assert!(matches!(err, RecoveryError::Integrity(_)));
    }

    #[test]
    fn custom_locator_preserved() {
        let (owner_trust, _device, chain) = make_fixture();
        let mut roots = BTreeMap::new();
        let root = crate::epoch::EpochRoot::generate().unwrap();
        roots.insert(0, root);

        let package = RecoveryPackage::create(
            b"secret",
            &owner_trust,
            &roots,
            &chain,
            &owner_trust.genesis_manifest.content.devices,
            Some("my backup".into()),
            ArgonProfile::test(),
        )
        .unwrap();

        let state = package.restore(b"secret").unwrap();
        assert_eq!(state.locator, "my backup");
    }
}
