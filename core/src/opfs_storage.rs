//! OPFS-backed storage for the browser (prototype).
//!
//! This module is only compiled for wasm32. It runs inside a Web Worker because
//! OPFS `FileSystemSyncAccessHandle` is only available there. The main thread
//! Leptos UI communicates with the worker; the core inside the worker keeps
//! synchronous `SecureStorage` / `ClockStorage` traits.

use std::collections::HashMap;

use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    FileSystemDirectoryHandle, FileSystemFileHandle, FileSystemGetFileOptions,
    FileSystemSyncAccessHandle, WorkerGlobalScope,
};

use crate::clock::{ClockError, ClockStorage};
use crate::identity::{IdentityError, SecureStorage};

const SECURE_STORE_FILE: &str = "eisen-secure-store.bin";
const CLOCK_FILE: &str = "eisen-clock.bin";

/// Single-file OPFS key/value store.
pub struct OpfsSecureStorage {
    file: FileSystemSyncAccessHandle,
}

impl OpfsSecureStorage {
    /// Async constructor — must be called from inside a Web Worker.
    pub async fn new() -> Result<Self, IdentityError> {
        let root = opfs_root_async().await.map_err(|e| IdentityError::Storage(js_err(e)))?;
        let file_handle = get_file_handle(&root, SECURE_STORE_FILE)
            .await
            .map_err(|e| IdentityError::Storage(js_err(e)))?;
        let access = create_sync_access_handle(&file_handle)
            .await
            .map_err(|e| IdentityError::Storage(js_err(e)))?;
        Ok(Self { file: access })
    }

    fn read_all(&self) -> Result<HashMap<String, Vec<u8>>, IdentityError> {
        let size = self
            .file
            .get_size()
            .map_err(|e| IdentityError::Storage(js_err(e)))? as usize;
        if size == 0 {
            return Ok(HashMap::new());
        }
        let mut buf = vec![0u8; size];
        self.file
            .read_with_u8_array(&mut buf)
            .map_err(|e| IdentityError::Storage(js_err(e)))?;
        cbor2::from_slice(&buf).map_err(|e| IdentityError::Decode(e.to_string()))
    }

    fn write_all(&self, map: &HashMap<String, Vec<u8>>) -> Result<(), IdentityError> {
        let bytes = cbor2::to_canonical_vec(map).map_err(|e| IdentityError::Encode(e.to_string()))?;
        self.file
            .truncate_with_f64(0.0)
            .map_err(|e| IdentityError::Storage(js_err(e)))?;
        self.file
            .write_with_u8_array(&bytes)
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

/// Single-file OPFS clock store.
pub struct OpfsClockStorage {
    file: FileSystemSyncAccessHandle,
}

impl OpfsClockStorage {
    /// Async constructor — must be called from inside a Web Worker.
    pub async fn new() -> Result<Self, ClockError> {
        let root = opfs_root_async().await.map_err(clock_err)?;
        let file_handle = get_file_handle(&root, CLOCK_FILE)
            .await
            .map_err(clock_err)?;
        let access = create_sync_access_handle(&file_handle)
            .await
            .map_err(clock_err)?;
        Ok(Self { file: access })
    }
}

impl ClockStorage for OpfsClockStorage {
    fn load(&self) -> Result<Option<(u64, u32)>, ClockError> {
        let size = self
            .file
            .get_size()
            .map_err(|e| ClockError::Storage(js_err(e)))? as usize;
        if size < 12 {
            return Ok(None);
        }
        let mut buf = [0u8; 12];
        self.file
            .read_with_u8_array(&mut buf)
            .map_err(|e| ClockError::Storage(js_err(e)))?;
        let wall = u64::from_le_bytes([buf[0], buf[1], buf[2], buf[3], buf[4], buf[5], buf[6], buf[7]]);
        let counter = u32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]);
        Ok(Some((wall, counter)))
    }

    fn save(&self, wall: u64, counter: u32) -> Result<(), ClockError> {
        let mut buf = [0u8; 12];
        buf[0..8].copy_from_slice(&wall.to_le_bytes());
        buf[8..12].copy_from_slice(&counter.to_le_bytes());
        self.file
            .truncate_with_f64(0.0)
            .map_err(|e| ClockError::Storage(js_err(e)))?;
        self.file
            .write_with_u8_array(&buf)
            .map_err(|e| ClockError::Storage(js_err(e)))?;
        self.file
            .flush()
            .map_err(|e| ClockError::Storage(js_err(e)))?;
        Ok(())
    }
}

async fn opfs_root_async() -> Result<FileSystemDirectoryHandle, JsValue> {
    let global = js_sys::global();
    let worker: WorkerGlobalScope = global.dyn_into()?;
    let navigator = worker.navigator();
    let storage = navigator.storage();
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
