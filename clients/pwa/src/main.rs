mod bridge;
mod install;
mod matrix;
mod task_form;
mod vault;

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine as _;
use eisen_core::clock::ClockStorage;
use eisen_core::epoch::{EpochKey, EpochRoot};
use eisen_core::export::VaultExport;
use eisen_core::identity::{create_vault, DeviceIdentity, OwnerTrust, SecureStorage, VaultId};
use eisen_core::manifest::ManifestChain;
use eisen_core::opfs_storage::{OpfsClockStorage, OpfsSecureStorage};
use eisen_core::recovery::{ArgonProfile, RecoveryPackage};
use eisen_core::store::LocalStore;
use eisen_core::{Hlc, Task};
use install::InstallPrompt;
use leptos::prelude::*;
use leptos_meta::*;
use matrix::{seed_store, Matrix};
use std::collections::BTreeMap;
use task_form::TaskForm;
use vault::Vault;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

#[wasm_bindgen(start)]
pub fn run() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();

    if js_sys::global().dyn_into::<web_sys::Window>().is_ok() {
        mount_to_body(App);
    }
}

fn main() {}

#[component]
fn App() -> impl IntoView {
    provide_meta_context();

    let (store, set_store) = signal(seed_store());
    let (editing, set_editing) = signal(None::<Task>);
    let (next_id, set_next_id) = signal(7u64);

    view! {
        <Html attr:lang="en" attr:dir="ltr" />
        <Title text="Eisen" />
        <Meta charset="UTF-8" />
        <Meta name="viewport" content="width=device-width, initial-scale=1.0" />
        <Meta name="theme-color" content="#0f766e" />
        <Link rel="manifest" href="/manifest.json" />

        <main>
            <h1>"Eisen"</h1>
            <p>"A local-first, installable PWA for secure task management."</p>
            <InstallPrompt />
            <Vault />
            <TaskForm
                store=store
                set_store=set_store
                editing=editing
                set_editing=set_editing
                next_id=next_id
                set_next_id=set_next_id
            />
            <Matrix
                store=store
                set_store=set_store
                set_editing=set_editing
                next_id=next_id
                set_next_id=set_next_id
            />
        </main>
    }
}

const EPOCH_ROOT_KEY: &str = "epoch:root:0";
const META_VAULT_ID: &str = "meta:vault_id";

fn js_err<E: std::fmt::Display>(e: E) -> JsValue {
    JsValue::from_str(&e.to_string())
}

async fn open_secure_storage(passphrase: &str) -> Result<OpfsSecureStorage, JsValue> {
    let profile = ArgonProfile::mobile();
    OpfsSecureStorage::new(passphrase.as_bytes(), profile)
        .await
        .map_err(js_err)
}

fn load_vault_id(secure: &OpfsSecureStorage) -> Result<VaultId, JsValue> {
    let bytes = secure
        .load(META_VAULT_ID)
        .map_err(js_err)?
        .ok_or_else(|| JsValue::from_str("no vault found"))?;
    Ok(VaultId(
        bytes
            .try_into()
            .map_err(|_| JsValue::from_str("corrupted vault id"))?,
    ))
}

fn load_epoch_root(secure: &OpfsSecureStorage) -> Result<EpochRoot, JsValue> {
    let bytes = secure
        .load(EPOCH_ROOT_KEY)
        .map_err(js_err)?
        .ok_or_else(|| JsValue::from_str("missing epoch root"))?;
    let root: [u8; 32] = bytes
        .try_into()
        .map_err(|_| JsValue::from_str("corrupted epoch root"))?;
    Ok(EpochRoot::from_bytes(root))
}

fn open_vault_state<'a>(
    secure: &'a OpfsSecureStorage,
) -> Result<
    (
        OwnerTrust,
        DeviceIdentity,
        EpochRoot,
        EpochKey,
        LocalStore<'a>,
    ),
    JsValue,
> {
    let vault_id = load_vault_id(secure)?;
    let owner = OwnerTrust::load(vault_id, secure).map_err(js_err)?;
    let management_device_id = owner.genesis_manifest.content.owner_management_device_id;
    let device = DeviceIdentity::load(management_device_id, secure).map_err(js_err)?;
    let epoch_root = load_epoch_root(secure)?;
    let epoch_key = epoch_root.derive(0).map_err(js_err)?;
    let store = LocalStore::open(secure, epoch_key, device.device_id).map_err(js_err)?;
    Ok((owner, device, epoch_root, epoch_key, store))
}

#[wasm_bindgen]
pub async fn worker_create_vault(passphrase: String) -> Result<String, JsValue> {
    let profile = ArgonProfile::mobile();
    let secure = open_secure_storage(&passphrase).await?;
    let _clock = OpfsClockStorage::new(passphrase.as_bytes(), profile)
        .await
        .map_err(js_err)?;
    let device = DeviceIdentity::generate().map_err(js_err)?;
    let hlc = Hlc {
        wall: 1,
        counter: 0,
        device_id: device.device_id,
    };
    let (owner, _device) = create_vault(&secure, hlc).map_err(js_err)?;

    let epoch_root = EpochRoot::generate().map_err(js_err)?;
    secure
        .store(EPOCH_ROOT_KEY, epoch_root.as_bytes())
        .map_err(js_err)?;
    let epoch_key = epoch_root.derive(0).map_err(js_err)?;
    _ = LocalStore::open(&secure, epoch_key, device.device_id).map_err(js_err)?;

    secure
        .store(META_VAULT_ID, &owner.vault_id.0)
        .map_err(js_err)?;
    _clock.save(1, 1).map_err(js_err)?;
    Ok(owner.vault_id.to_hex())
}

#[wasm_bindgen]
pub async fn worker_open_vault(passphrase: String) -> Result<String, JsValue> {
    let profile = ArgonProfile::mobile();
    let secure = open_secure_storage(&passphrase).await?;
    let _clock = OpfsClockStorage::new(passphrase.as_bytes(), profile)
        .await
        .map_err(js_err)?;
    let (owner, _device, _epoch_root, _epoch_key, _store) = open_vault_state(&secure)?;
    Ok(owner.vault_id.to_hex())
}

#[wasm_bindgen]
pub async fn worker_create_recovery_package(
    passphrase: String,
    locator: String,
) -> Result<String, JsValue> {
    let secure = open_secure_storage(&passphrase).await?;
    let (owner, _device, epoch_root, _epoch_key, _store) = open_vault_state(&secure)?;

    let chain = ManifestChain::new(owner.genesis_manifest.clone()).map_err(js_err)?;
    let devices = owner.genesis_manifest.content.devices.clone();
    let mut epoch_roots = BTreeMap::new();
    epoch_roots.insert(0, epoch_root);

    let locator = if locator.is_empty() {
        None
    } else {
        Some(locator)
    };

    let package = RecoveryPackage::create(
        passphrase.as_bytes(),
        &owner,
        &epoch_roots,
        &chain,
        &devices,
        locator,
        ArgonProfile::mobile(),
    )
    .map_err(js_err)?;
    Ok(B64.encode(package.to_bytes()))
}

#[wasm_bindgen]
pub async fn worker_restore_recovery_package(
    package_b64: String,
    passphrase: String,
) -> Result<String, JsValue> {
    let package_bytes = B64
        .decode(package_b64)
        .map_err(|_| JsValue::from_str("invalid base64"))?;
    let package = RecoveryPackage::from_bytes(&package_bytes).map_err(js_err)?;
    let state = package.restore(passphrase.as_bytes()).map_err(js_err)?;

    let secure = open_secure_storage(&passphrase).await?;
    state.owner_trust.persist(&secure).map_err(js_err)?;
    state.new_device.persist(&secure).map_err(js_err)?;
    for (epoch, root) in &state.epoch_roots {
        secure
            .store(&format!("epoch:root:{epoch}"), root.as_bytes())
            .map_err(js_err)?;
    }
    secure
        .store(META_VAULT_ID, &state.owner_trust.vault_id.0)
        .map_err(js_err)?;

    let epoch_key = state
        .epoch_roots
        .get(&0)
        .ok_or_else(|| JsValue::from_str("missing epoch 0 root"))?
        .derive(0)
        .map_err(js_err)?;
    _ = LocalStore::open(&secure, epoch_key, state.new_device.device_id).map_err(js_err)?;

    Ok(state.owner_trust.vault_id.to_hex())
}

#[wasm_bindgen]
pub async fn worker_export_vault(passphrase: String) -> Result<String, JsValue> {
    let secure = open_secure_storage(&passphrase).await?;
    let (owner, _device, _epoch_root, _epoch_key, store) = open_vault_state(&secure)?;
    let chain = ManifestChain::new(owner.genesis_manifest.clone()).map_err(js_err)?;
    let export = store.export(&owner, &chain).map_err(js_err)?;
    Ok(B64.encode(export.to_bytes()))
}

#[wasm_bindgen]
pub async fn worker_import_vault(
    passphrase: String,
    export_b64: String,
) -> Result<String, JsValue> {
    let secure = open_secure_storage(&passphrase).await?;
    let (owner, _device, _epoch_root, _epoch_key, mut store) = open_vault_state(&secure)?;
    let export_bytes = B64
        .decode(export_b64)
        .map_err(|_| JsValue::from_str("invalid base64"))?;
    let export = VaultExport::from_bytes(&export_bytes).map_err(js_err)?;
    store.import(&export, &owner).map_err(js_err)?;
    Ok("import completed".to_string())
}
