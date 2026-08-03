//! Encrypted, passphrase-protected OPFS storage for the browser (P2.01 / #97).
//!
//! Runs inside a Web Worker because `FileSystemSyncAccessHandle` is only
//! available there. The wrapping key is derived once from the user passphrase
//! with Argon2id and kept in worker memory while the vault is unlocked.
//!
//! Both `SecureStorage` and `ClockStorage` are backed by separate AES-256-GCM
//! encrypted files in the OPFS origin-private directory.

#![cfg(target_arch = "wasm32")]

use crate::clock::{ClockError, ClockStorage};
use crate::identity::{IdentityError, SecureStorage};
use crate::recovery::ArgonProfile;
use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use argon2::{Algorithm, Argon2, Params, Version};
use std::collections::HashMap;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    FileSystemDirectoryHandle, FileSystemFileHandle, FileSystemGetFileOptions,
    FileSystemSyncAccessHandle, StorageManager, WorkerGlobalScope,
};

const SALT_LEN: usize = 16;
const KEY_LEN: usize = 32;
const NONCE_LEN: usize = 12;
const SECURE_STORE_FILE: &str = "eisen-secure-store.bin";
const CLOCK_FILE: &str = "eisen-clock.bin";
const MAGIC: &[u8] = b"EISEN-OPFS-V1";

/// Single-file, encrypted OPFS key/value store.
pub struct OpfsSecureStorage {
    file: FileSystemSyncAccessHandle,
    wrapping_key: [u8; KEY_LEN],
    salt: [u8; SALT_LEN],
}

impl OpfsSecureStorage {
    /// Open or create the encrypted secure store.
    ///
    /// `passphrase` is used to derive the wrapping key with Argon2id.  If the
    /// file does not exist yet, a new random salt is generated and the empty
    /// store is written.  If it already exists, the salt is read and the
    /// passphrase is checked by decrypting the file header.
    pub async fn new(passphrase: &[u8], profile: ArgonProfile) -> Result<Self, IdentityError> {
        let root = opfs_root_async()
            .await
            .map_err(|e| IdentityError::Storage(js_err(e)))?;
        let file_handle = get_file_handle(&root, SECURE_STORE_FILE)
            .await
            .map_err(|e| IdentityError::Storage(js_err(e)))?;
        let file = create_sync_access_handle(&file_handle)
            .await
            .map_err(|e| IdentityError::Storage(js_err(e)))?;

        let size = file
            .get_size()
            .map_err(|e| IdentityError::Storage(js_err(e)))? as usize;
        let (salt, wrapping_key) = if size == 0 {
            let mut salt = [0u8; SALT_LEN];
            getrandom::getrandom(&mut salt).map_err(|e| IdentityError::Entropy(e.to_string()))?;
            let wrapping_key = derive_key(passphrase, &salt, &profile)?;

            let empty = HashMap::<String, Vec<u8>>::new();
            let plaintext = cbor2::to_canonical_vec(&empty)
                .map_err(|e| IdentityError::Encode(e.to_string()))?;
            let mut nonce = [0u8; NONCE_LEN];
            getrandom::getrandom(&mut nonce).map_err(|e| IdentityError::Entropy(e.to_string()))?;
            let ciphertext = aes_encrypt(&wrapping_key, &nonce, &plaintext, MAGIC)?;

            let mut blob = Vec::with_capacity(SALT_LEN + NONCE_LEN + ciphertext.len());
            blob.extend_from_slice(&salt);
            blob.extend_from_slice(&nonce);
            blob.extend_from_slice(&ciphertext);

            file.truncate_with_f64(0.0)
                .map_err(|e| IdentityError::Storage(js_err(e)))?;
            file.write_with_u8_array(&blob)
                .map_err(|e| IdentityError::Storage(js_err(e)))?;
            file.flush()
                .map_err(|e| IdentityError::Storage(js_err(e)))?;

            (salt, wrapping_key)
        } else {
            if size < SALT_LEN + NONCE_LEN {
                return Err(IdentityError::Storage(
                    "secure store file is truncated".into(),
                ));
            }
            let mut blob = vec![0u8; size];
            file.read_with_u8_array(&mut blob)
                .map_err(|e| IdentityError::Storage(js_err(e)))?;

            let salt: [u8; SALT_LEN] = blob[0..SALT_LEN]
                .try_into()
                .map_err(|_| IdentityError::Decode("invalid salt length".into()))?;
            let nonce: [u8; NONCE_LEN] = blob[SALT_LEN..SALT_LEN + NONCE_LEN]
                .try_into()
                .map_err(|_| IdentityError::Decode("invalid nonce length".into()))?;
            let ciphertext = &blob[SALT_LEN + NONCE_LEN..];

            let wrapping_key = derive_key(passphrase, &salt, &profile)?;
            let _plaintext =
                aes_decrypt(&wrapping_key, &nonce, ciphertext, MAGIC).map_err(|_| {
                    IdentityError::Crypto("wrong passphrase or tampered secure store".into())
                })?;

            (salt, wrapping_key)
        };

        Ok(Self {
            file,
            wrapping_key,
            salt,
        })
    }

    fn read_all(&self) -> Result<HashMap<String, Vec<u8>>, IdentityError> {
        let size = self
            .file
            .get_size()
            .map_err(|e| IdentityError::Storage(js_err(e)))? as usize;
        if size < SALT_LEN + NONCE_LEN {
            return Ok(HashMap::new());
        }
        let mut blob = vec![0u8; size];
        self.file
            .read_with_u8_array(&mut blob)
            .map_err(|e| IdentityError::Storage(js_err(e)))?;

        let salt: [u8; SALT_LEN] = blob[0..SALT_LEN]
            .try_into()
            .map_err(|_| IdentityError::Decode("invalid salt length".into()))?;
        if salt != self.salt {
            return Err(IdentityError::Storage("secure store salt changed".into()));
        }
        let nonce: [u8; NONCE_LEN] = blob[SALT_LEN..SALT_LEN + NONCE_LEN]
            .try_into()
            .map_err(|_| IdentityError::Decode("invalid nonce length".into()))?;
        let ciphertext = &blob[SALT_LEN + NONCE_LEN..];

        let plaintext = aes_decrypt(&self.wrapping_key, &nonce, ciphertext, MAGIC)?;
        cbor2::from_slice(&plaintext).map_err(|e| IdentityError::Decode(e.to_string()))
    }

    fn write_all(&self, map: &HashMap<String, Vec<u8>>) -> Result<(), IdentityError> {
        let plaintext =
            cbor2::to_canonical_vec(map).map_err(|e| IdentityError::Encode(e.to_string()))?;
        let mut nonce = [0u8; NONCE_LEN];
        getrandom::getrandom(&mut nonce).map_err(|e| IdentityError::Entropy(e.to_string()))?;
        let ciphertext = aes_encrypt(&self.wrapping_key, &nonce, &plaintext, MAGIC)?;

        let mut blob = Vec::with_capacity(SALT_LEN + NONCE_LEN + ciphertext.len());
        blob.extend_from_slice(&self.salt);
        blob.extend_from_slice(&nonce);
        blob.extend_from_slice(&ciphertext);

        self.file
            .truncate_with_f64(0.0)
            .map_err(|e| IdentityError::Storage(js_err(e)))?;
        self.file
            .write_with_u8_array(&blob)
            .map_err(|e| IdentityError::Storage(js_err(e)))?;
        self.file
            .flush()
            .map_err(|e| IdentityError::Storage(js_err(e)))?;

        Ok(())
    }
}

impl SecureStorage for OpfsSecureStorage {
    fn store(&self, key: &str, value: &[u8]) -> Result<(), IdentityError> {
        let mut map = self.read_all()?;
        map.insert(key.to_string(), value.to_vec());
        self.write_all(&map)
    }

    fn load(&self, key: &str) -> Result<Option<Vec<u8>>, IdentityError> {
        let map = self.read_all()?;
        Ok(map.get(key).cloned())
    }

    fn delete(&self, key: &str) -> Result<(), IdentityError> {
        let mut map = self.read_all()?;
        map.remove(key);
        self.write_all(&map)
    }
}

/// Single-file, encrypted OPFS clock store.
pub struct OpfsClockStorage {
    file: FileSystemSyncAccessHandle,
    wrapping_key: [u8; KEY_LEN],
    salt: [u8; SALT_LEN],
}

impl OpfsClockStorage {
    /// Open or create the encrypted clock store.
    pub async fn new(passphrase: &[u8], profile: ArgonProfile) -> Result<Self, ClockError> {
        let root = opfs_root_async().await.map_err(clock_err)?;
        let file_handle = get_file_handle(&root, CLOCK_FILE)
            .await
            .map_err(clock_err)?;
        let file = create_sync_access_handle(&file_handle)
            .await
            .map_err(clock_err)?;

        let size = file.get_size().map_err(clock_err)? as usize;
        let (salt, wrapping_key) = if size == 0 {
            let mut salt = [0u8; SALT_LEN];
            getrandom::getrandom(&mut salt).map_err(|e| ClockError::Storage(e.to_string()))?;
            let wrapping_key = derive_key(passphrase, &salt, &profile)
                .map_err(|e| ClockError::Storage(e.to_string()))?;

            let mut nonce = [0u8; NONCE_LEN];
            getrandom::getrandom(&mut nonce).map_err(|e| ClockError::Storage(e.to_string()))?;
            let plaintext = [0u8; U64_LEN + U32_LEN];
            let ciphertext = aes_encrypt(&wrapping_key, &nonce, &plaintext, MAGIC)
                .map_err(|e| ClockError::Storage(e.to_string()))?;

            let mut blob = Vec::with_capacity(SALT_LEN + NONCE_LEN + ciphertext.len());
            blob.extend_from_slice(&salt);
            blob.extend_from_slice(&nonce);
            blob.extend_from_slice(&ciphertext);

            file.truncate_with_f64(0.0).map_err(clock_err)?;
            file.write_with_u8_array(&blob).map_err(clock_err)?;
            file.flush().map_err(clock_err)?;

            (salt, wrapping_key)
        } else {
            if size < SALT_LEN + NONCE_LEN {
                return Err(ClockError::Storage("clock file is truncated".into()));
            }
            let mut blob = vec![0u8; size];
            file.read_with_u8_array(&mut blob).map_err(clock_err)?;

            let salt: [u8; SALT_LEN] = blob[0..SALT_LEN]
                .try_into()
                .map_err(|_| ClockError::Storage("invalid salt length".into()))?;
            let nonce: [u8; NONCE_LEN] = blob[SALT_LEN..SALT_LEN + NONCE_LEN]
                .try_into()
                .map_err(|_| ClockError::Storage("invalid nonce length".into()))?;
            let ciphertext = &blob[SALT_LEN + NONCE_LEN..];

            let wrapping_key = derive_key(passphrase, &salt, &profile)
                .map_err(|e| ClockError::Storage(e.to_string()))?;
            aes_decrypt(&wrapping_key, &nonce, ciphertext, MAGIC).map_err(|_| {
                ClockError::Storage("wrong passphrase or tampered clock file".into())
            })?;

            (salt, wrapping_key)
        };

        Ok(Self {
            file,
            wrapping_key,
            salt,
        })
    }

    fn read(&self) -> Result<Option<(u64, u32)>, ClockError> {
        let size = self.file.get_size().map_err(clock_err)? as usize;
        if size < SALT_LEN + NONCE_LEN {
            return Ok(None);
        }
        let mut blob = vec![0u8; size];
        self.file.read_with_u8_array(&mut blob).map_err(clock_err)?;

        let salt: [u8; SALT_LEN] = blob[0..SALT_LEN]
            .try_into()
            .map_err(|_| ClockError::Storage("invalid salt length".into()))?;
        if salt != self.salt {
            return Err(ClockError::Storage("clock salt changed".into()));
        }
        let nonce: [u8; NONCE_LEN] = blob[SALT_LEN..SALT_LEN + NONCE_LEN]
            .try_into()
            .map_err(|_| ClockError::Storage("invalid nonce length".into()))?;
        let ciphertext = &blob[SALT_LEN + NONCE_LEN..];

        let plaintext = aes_decrypt(&self.wrapping_key, &nonce, ciphertext, MAGIC)
            .map_err(|e| ClockError::Storage(e.to_string()))?;
        if plaintext.len() != U64_LEN + U32_LEN {
            return Err(ClockError::Storage("clock plaintext length".into()));
        }

        let wall = u64::from_le_bytes(plaintext[0..U64_LEN].try_into().unwrap());
        let counter = u32::from_le_bytes(plaintext[U64_LEN..U64_LEN + U32_LEN].try_into().unwrap());
        Ok(Some((wall, counter)))
    }

    fn write(&self, wall: u64, counter: u32) -> Result<(), ClockError> {
        let mut plaintext = [0u8; U64_LEN + U32_LEN];
        plaintext[0..U64_LEN].copy_from_slice(&wall.to_le_bytes());
        plaintext[U64_LEN..U64_LEN + U32_LEN].copy_from_slice(&counter.to_le_bytes());

        let mut nonce = [0u8; NONCE_LEN];
        getrandom::getrandom(&mut nonce).map_err(|e| ClockError::Storage(e.to_string()))?;
        let ciphertext = aes_encrypt(&self.wrapping_key, &nonce, &plaintext, MAGIC)
            .map_err(|e| ClockError::Storage(e.to_string()))?;

        let mut blob = Vec::with_capacity(SALT_LEN + NONCE_LEN + ciphertext.len());
        blob.extend_from_slice(&self.salt);
        blob.extend_from_slice(&nonce);
        blob.extend_from_slice(&ciphertext);

        self.file.truncate_with_f64(0.0).map_err(clock_err)?;
        self.file.write_with_u8_array(&blob).map_err(clock_err)?;
        self.file.flush().map_err(clock_err)?;

        Ok(())
    }
}

impl ClockStorage for OpfsClockStorage {
    fn load(&self) -> Result<Option<(u64, u32)>, ClockError> {
        self.read()
    }

    fn save(&self, wall: u64, counter: u32) -> Result<(), ClockError> {
        self.write(wall, counter)
    }
}

const U64_LEN: usize = 8;
const U32_LEN: usize = 4;

fn derive_key(
    passphrase: &[u8],
    salt: &[u8; SALT_LEN],
    profile: &ArgonProfile,
) -> Result<[u8; KEY_LEN], IdentityError> {
    let params = Params::new(
        profile.m_cost,
        profile.t_cost,
        profile.p_cost,
        Some(KEY_LEN),
    )
    .map_err(|e| IdentityError::Crypto(e.to_string()))?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut out = [0u8; KEY_LEN];
    argon2
        .hash_password_into(passphrase, salt, &mut out)
        .map_err(|e| IdentityError::Crypto(e.to_string()))?;
    Ok(out)
}

fn aes_encrypt(
    key: &[u8; KEY_LEN],
    nonce: &[u8; NONCE_LEN],
    plaintext: &[u8],
    aad: &[u8],
) -> Result<Vec<u8>, IdentityError> {
    let cipher =
        Aes256Gcm::new_from_slice(key).map_err(|e| IdentityError::Crypto(e.to_string()))?;
    let nonce = Nonce::from_slice(nonce);
    cipher
        .encrypt(
            nonce,
            Payload {
                msg: plaintext,
                aad,
            },
        )
        .map_err(|e| IdentityError::Crypto(e.to_string()))
}

fn aes_decrypt(
    key: &[u8; KEY_LEN],
    nonce: &[u8; NONCE_LEN],
    ciphertext: &[u8],
    aad: &[u8],
) -> Result<Vec<u8>, IdentityError> {
    let cipher =
        Aes256Gcm::new_from_slice(key).map_err(|e| IdentityError::Crypto(e.to_string()))?;
    let nonce = Nonce::from_slice(nonce);
    cipher
        .decrypt(
            nonce,
            Payload {
                msg: ciphertext,
                aad,
            },
        )
        .map_err(|e| IdentityError::Crypto(e.to_string()))
}

async fn opfs_root_async() -> Result<FileSystemDirectoryHandle, JsValue> {
    let global = js_sys::global();
    let worker: WorkerGlobalScope = global.dyn_into()?;
    let navigator = worker.navigator();
    let storage: StorageManager = navigator.storage();
    let promise = storage.get_directory();
    let result = JsFuture::from(promise).await?;
    result.dyn_into()
}

async fn get_file_handle(
    root: &FileSystemDirectoryHandle,
    name: &str,
) -> Result<FileSystemFileHandle, JsValue> {
    let options = FileSystemGetFileOptions::new();
    options.set_create(true);
    let promise = root.get_file_handle_with_options(name, &options);
    let result = JsFuture::from(promise).await?;
    result.dyn_into()
}

async fn create_sync_access_handle(
    file: &FileSystemFileHandle,
) -> Result<FileSystemSyncAccessHandle, JsValue> {
    let promise = file.create_sync_access_handle();
    let result = JsFuture::from(promise).await?;
    result.dyn_into()
}

fn js_err(value: JsValue) -> String {
    value
        .as_string()
        .unwrap_or_else(|| "unknown JS error".to_string())
}

fn clock_err(value: JsValue) -> ClockError {
    ClockError::Storage(js_err(value))
}

// wasm32 is single-threaded; these JS handles are only accessed from the worker
// thread that created them. Marking them Send + Sync lets them satisfy the core
// storage traits. This is safe only because the wasm32 target has no threads.
unsafe impl Send for OpfsSecureStorage {}
unsafe impl Sync for OpfsSecureStorage {}
unsafe impl Send for OpfsClockStorage {}
unsafe impl Sync for OpfsClockStorage {}
