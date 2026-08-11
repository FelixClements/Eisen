mod bridge;
use bridge::WorkerClient;
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
use eisen_core::{DeviceId, Hlc, Mutation, Task, TaskId};
use install::InstallPrompt;
use leptos::prelude::*;
use leptos_meta::*;
use matrix::Matrix;
use std::collections::BTreeMap;
use std::rc::Rc;
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

    let worker = Rc::new(WorkerClient::new().expect("Could not start secure worker"));
    let (tasks, set_tasks) = signal(Vec::<Task>::new());
    let (editing, set_editing) = signal(None::<Task>);

    view! {
        <Html attr:lang="en" attr:dir="ltr" />
        <Title text="Eisen" />
        <Meta charset="UTF-8" />
        <Meta name="viewport" content="width=device-width, initial-scale=1.0" />
        <Meta name="theme-color" content="#0f766e" />
        <Meta name="referrer" content="no-referrer" />
        <Meta name="apple-mobile-web-app-capable" content="yes" />
        <Meta name="apple-mobile-web-app-status-bar-style" content="black-translucent" />
        <meta
            http-equiv="Content-Security-Policy"
            content="default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; worker-src 'self'; style-src 'self' 'unsafe-inline'; connect-src 'self'; font-src 'self'; img-src 'self' data:; media-src 'self'; object-src 'none'; frame-ancestors 'none'; base-uri 'self'; form-action 'self';"
        />
        <Link rel="manifest" href="/manifest.json" />

        <main>
            <h1>"Eisen"</h1>
            <p>"A local-first, installable PWA for secure task management."</p>
            <InstallPrompt />
            <Vault worker=worker.clone() set_tasks=set_tasks />
            <TaskForm
                worker=worker.clone()
                editing=editing
                set_editing=set_editing
                set_tasks=set_tasks
            />
            <Matrix
                worker=worker.clone()
                tasks=tasks
                set_tasks=set_tasks
                set_editing=set_editing
            />
        </main>

        <footer class="status">
            <p class="local-only">
                "Local-only: your vault and tasks live on this device. "
                "There is no cloud sync or cross-device backup yet."
            </p>
            <p class="limitation">
                "Backups and recovery packages are local, encrypted files. "
                "If you lose the file, the passphrase, or this device, the vault cannot be recovered unless you have a separate copy."
            </p>
        </footer>
    }
}

const EPOCH_ROOT_KEY: &str = "epoch:root:0";
const META_VAULT_ID: &str = "meta:vault_id";

struct WorkerVault {
    secure: OpfsSecureStorage,
    owner: OwnerTrust,
    device: DeviceIdentity,
    epoch_key: EpochKey,
}

std::thread_local! {
    static VAULT: std::cell::RefCell<Option<WorkerVault>> = const { std::cell::RefCell::new(None) };
}

fn js_err<E: std::fmt::Display>(e: E) -> JsValue {
    JsValue::from_str(&e.to_string())
}

fn repair_err<E: std::fmt::Display>(e: E) -> JsValue {
    JsValue::from_str(&format!("Repair required: {e}"))
}

async fn open_secure_storage(passphrase: &str) -> Result<OpfsSecureStorage, JsValue> {
    let profile = ArgonProfile::mobile();
    OpfsSecureStorage::new(passphrase.as_bytes(), profile)
        .await
        .map_err(|e| match e {
            eisen_core::identity::IdentityError::Storage(msg) => {
                JsValue::from_str(&format!("Encrypted storage (OPFS) cannot be opened: {msg}"))
            }
            eisen_core::identity::IdentityError::Crypto(msg) => {
                JsValue::from_str(&format!("Cannot unlock vault: {msg}"))
            }
            e => js_err(e),
        })
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

fn load_vault_components(
    secure: &OpfsSecureStorage,
) -> Result<(OwnerTrust, DeviceIdentity, EpochRoot, EpochKey), JsValue> {
    let vault_id = load_vault_id(secure)?;
    let owner = OwnerTrust::load(vault_id, secure).map_err(repair_err)?;
    let management_device_id = owner.genesis_manifest.content.owner_management_device_id;
    let device = DeviceIdentity::load(management_device_id, secure).map_err(repair_err)?;
    let epoch_root = load_epoch_root(secure)?;
    let epoch_key = epoch_root.derive(0).map_err(repair_err)?;
    Ok((owner, device, epoch_root, epoch_key))
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
    let (owner, device, epoch_root, epoch_key) = load_vault_components(secure)?;
    let store = LocalStore::open(secure, epoch_key, device.device_id).map_err(repair_err)?;
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
    let (owner, mgmt) = create_vault(&secure, hlc).map_err(js_err)?;

    let epoch_root = EpochRoot::generate().map_err(js_err)?;
    secure
        .store(EPOCH_ROOT_KEY, epoch_root.as_bytes())
        .map_err(js_err)?;
    let epoch_key = epoch_root.derive(0).map_err(js_err)?;
    _ = LocalStore::open(&secure, epoch_key, mgmt.device_id).map_err(js_err)?;

    secure
        .store(META_VAULT_ID, &owner.vault_id.0)
        .map_err(js_err)?;
    _clock.save(1, 1).map_err(js_err)?;

    let vault_id = owner.vault_id.to_hex();
    VAULT.with(|v| {
        *v.borrow_mut() = Some(WorkerVault {
            secure,
            owner,
            device: mgmt,
            epoch_key,
        });
    });
    Ok(vault_id)
}

#[wasm_bindgen]
pub async fn worker_open_vault(passphrase: String) -> Result<String, JsValue> {
    let profile = ArgonProfile::mobile();
    let secure = open_secure_storage(&passphrase).await?;
    let _clock = OpfsClockStorage::new(passphrase.as_bytes(), profile)
        .await
        .map_err(js_err)?;
    let (owner, device, _epoch_root, epoch_key) = load_vault_components(&secure)?;
    let _store = LocalStore::open(&secure, epoch_key, device.device_id).map_err(repair_err)?;

    let vault_id = owner.vault_id.to_hex();
    VAULT.with(|v| {
        *v.borrow_mut() = Some(WorkerVault {
            secure,
            owner,
            device,
            epoch_key,
        });
    });
    Ok(vault_id)
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

    let vault_id = state.owner_trust.vault_id;
    VAULT.with(|v| {
        *v.borrow_mut() = Some(WorkerVault {
            secure,
            owner: state.owner_trust,
            device: state.new_device,
            epoch_key,
        });
    });
    Ok(vault_id.to_hex())
}

#[wasm_bindgen]
pub fn worker_persist() -> Result<String, JsValue> {
    VAULT.with(|v| {
        let v = v.borrow();
        let vault = v
            .as_ref()
            .ok_or_else(|| JsValue::from_str("vault not open"))?;
        let mut store = LocalStore::open(&vault.secure, vault.epoch_key, vault.device.device_id)
            .map_err(js_err)?;
        let _ = store
            .create_signed_snapshot(&vault.owner.owner_signing_key, vault.owner.vault_id)
            .map_err(js_err)?;
        Ok("persisted".to_string())
    })
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

fn next_hlc(store: &LocalStore, device_id: DeviceId) -> Hlc {
    let last_wall = store.metadata().last_wall;
    let last_counter = store.metadata().last_counter;
    let wall = (js_sys::Date::now() as u64).max(last_wall);
    let counter = if wall == last_wall {
        last_counter.saturating_add(1)
    } else {
        0
    };
    Hlc {
        wall,
        counter,
        device_id,
    }
}

fn random_task_id() -> Result<TaskId, JsValue> {
    let mut bytes = [0u8; 16];
    getrandom::getrandom(&mut bytes)
        .map_err(|e| JsValue::from_str(&format!("getrandom failed: {e}")))?;
    Ok(TaskId(bytes))
}

fn task_id_from_b64(id_b64: &str) -> Result<TaskId, JsValue> {
    let bytes = B64
        .decode(id_b64)
        .map_err(|_| JsValue::from_str("invalid id"))?;
    let arr: [u8; 16] = bytes
        .try_into()
        .map_err(|_| JsValue::from_str("corrupted id length"))?;
    Ok(TaskId(arr))
}

fn apply_local_mutation(
    store: &mut LocalStore,
    owner: &OwnerTrust,
    device: &DeviceIdentity,
    mutation: Mutation,
) -> Result<(), JsValue> {
    let hlc = mutation
        .hlc()
        .ok_or_else(|| JsValue::from_str("mutation has no hlc"))?;
    let envelope = eisen_core::envelope::Envelope::sign(&mutation, hlc, &device.signing_key)
        .map_err(js_err)?;
    let seq = store
        .metadata()
        .seq
        .get(&device.device_id)
        .copied()
        .unwrap_or(0)
        .saturating_add(1);
    store.apply(envelope, owner, seq).map_err(js_err)?;
    Ok(())
}

fn list_tasks_base64(store: &LocalStore) -> Result<String, JsValue> {
    let tasks: Vec<Task> = store.store().values().cloned().collect();
    let bytes = cbor2::to_canonical_vec(&tasks).map_err(js_err)?;
    Ok(B64.encode(bytes))
}

fn with_open_store<T, F: FnOnce(&mut LocalStore, &WorkerVault) -> Result<T, JsValue>>(
    f: F,
) -> Result<T, JsValue> {
    VAULT.with(|v| {
        let v = v.borrow();
        let vault = v
            .as_ref()
            .ok_or_else(|| JsValue::from_str("vault not open"))?;
        let mut store = LocalStore::open(&vault.secure, vault.epoch_key, vault.device.device_id)
            .map_err(js_err)?;
        f(&mut store, vault)
    })
}

#[wasm_bindgen]
pub fn worker_list_tasks() -> Result<String, JsValue> {
    with_open_store(|store, _| list_tasks_base64(store))
}

#[wasm_bindgen]
pub fn worker_create_task(title: String, notes: String, quadrant: u8) -> Result<String, JsValue> {
    with_open_store(|store, vault| {
        let hlc = next_hlc(store, vault.device.device_id);
        let id = random_task_id()?;
        let notes = if notes.is_empty() { None } else { Some(notes) };
        let mutation = Mutation::Create {
            hlc,
            id,
            title,
            notes,
            quadrant,
            due_date: None,
        };
        apply_local_mutation(store, &vault.owner, &vault.device, mutation)?;
        list_tasks_base64(store)
    })
}

#[wasm_bindgen]
pub fn worker_update_task(
    id_b64: String,
    title: String,
    notes: String,
    quadrant: u8,
) -> Result<String, JsValue> {
    with_open_store(|store, vault| {
        let hlc = next_hlc(store, vault.device.device_id);
        let id = task_id_from_b64(&id_b64)?;
        let title = Some(title);
        let notes = if notes.is_empty() {
            Some(None)
        } else {
            Some(Some(notes))
        };
        let quadrant = Some(quadrant);
        let mutation = Mutation::Update {
            hlc,
            id,
            title,
            notes,
            quadrant,
            due_date: None,
        };
        apply_local_mutation(store, &vault.owner, &vault.device, mutation)?;
        list_tasks_base64(store)
    })
}

#[wasm_bindgen]
pub fn worker_complete_task(id_b64: String) -> Result<String, JsValue> {
    with_open_store(|store, vault| {
        let hlc = next_hlc(store, vault.device.device_id);
        let id = task_id_from_b64(&id_b64)?;
        let mutation = Mutation::Complete { hlc, id };
        apply_local_mutation(store, &vault.owner, &vault.device, mutation)?;
        list_tasks_base64(store)
    })
}

#[wasm_bindgen]
pub fn worker_delete_task(id_b64: String) -> Result<String, JsValue> {
    with_open_store(|store, vault| {
        let hlc = next_hlc(store, vault.device.device_id);
        let id = task_id_from_b64(&id_b64)?;
        let mutation = Mutation::Delete { hlc, id };
        apply_local_mutation(store, &vault.owner, &vault.device, mutation)?;
        list_tasks_base64(store)
    })
}

#[wasm_bindgen]
pub fn worker_restore_task(id_b64: String) -> Result<String, JsValue> {
    with_open_store(|store, vault| {
        let hlc = next_hlc(store, vault.device.device_id);
        let id = task_id_from_b64(&id_b64)?;
        let mutation = Mutation::Restore { hlc, id };
        apply_local_mutation(store, &vault.owner, &vault.device, mutation)?;
        list_tasks_base64(store)
    })
}

#[wasm_bindgen]
pub fn worker_move_task(id_b64: String, quadrant: u8) -> Result<String, JsValue> {
    with_open_store(|store, vault| {
        let hlc = next_hlc(store, vault.device.device_id);
        let id = task_id_from_b64(&id_b64)?;
        let mutation = Mutation::Update {
            hlc,
            id,
            title: None,
            notes: None,
            quadrant: Some(quadrant),
            due_date: None,
        };
        apply_local_mutation(store, &vault.owner, &vault.device, mutation)?;
        list_tasks_base64(store)
    })
}
