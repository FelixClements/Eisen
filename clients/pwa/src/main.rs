mod bridge;
mod install;
mod matrix;
mod task_form;
mod vault;

use eisen_core::clock::ClockStorage;
use eisen_core::identity::SecureStorage;
use eisen_core::{Hlc, Task};
use install::InstallPrompt;
use leptos::prelude::*;
use leptos_meta::*;
use matrix::{seed_store, Matrix};
use task_form::TaskForm;
use vault::Vault;
use wasm_bindgen::prelude::*;

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
            <TaskForm store=store set_store=set_store editing=editing set_editing=set_editing />
            <Matrix store=store set_editing=set_editing />
        </main>
    }
}

#[wasm_bindgen]
pub async fn worker_create_vault(passphrase: String) -> Result<String, JsValue> {
    use eisen_core::identity::{create_vault, DeviceIdentity};
    use eisen_core::opfs_storage::{OpfsClockStorage, OpfsSecureStorage};
    use eisen_core::recovery::ArgonProfile;

    let profile = ArgonProfile::mobile();
    let secure = OpfsSecureStorage::new(passphrase.as_bytes(), profile)
        .await
        .map_err(js_err)?;
    let clock = OpfsClockStorage::new(passphrase.as_bytes(), profile)
        .await
        .map_err(js_err)?;
    let device = DeviceIdentity::generate().map_err(js_err)?;
    let hlc = Hlc {
        wall: 1,
        counter: 0,
        device_id: device.device_id,
    };
    let (owner, _device) = create_vault(&secure, hlc).map_err(js_err)?;
    secure
        .store("meta:vault_id", &owner.vault_id.0)
        .map_err(js_err)?;
    clock.save(1, 1).map_err(js_err)?;
    Ok(owner.vault_id.to_hex())
}

#[wasm_bindgen]
pub async fn worker_open_vault(passphrase: String) -> Result<String, JsValue> {
    use eisen_core::identity::{OwnerTrust, VaultId};
    use eisen_core::opfs_storage::{OpfsClockStorage, OpfsSecureStorage};
    use eisen_core::recovery::ArgonProfile;

    let profile = ArgonProfile::mobile();
    let secure = OpfsSecureStorage::new(passphrase.as_bytes(), profile)
        .await
        .map_err(js_err)?;
    let _clock = OpfsClockStorage::new(passphrase.as_bytes(), profile)
        .await
        .map_err(js_err)?;
    let bytes = secure
        .load("meta:vault_id")
        .map_err(js_err)?
        .ok_or_else(|| JsValue::from_str("no vault found"))?;
    let vault_id = VaultId(
        bytes
            .try_into()
            .map_err(|_| JsValue::from_str("corrupted vault id"))?,
    );
    let _owner = OwnerTrust::load(vault_id, &secure).map_err(js_err)?;
    Ok(vault_id.to_hex())
}

fn js_err<E: std::fmt::Display>(e: E) -> JsValue {
    JsValue::from_str(&e.to_string())
}
