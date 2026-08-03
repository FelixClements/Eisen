use leptos::prelude::*;
use wasm_bindgen::JsCast;

#[derive(Clone)]
pub enum VaultState {
    Locked,
    Creating,
    Unlocking,
    Unlocked { vault_id: String },
    Error(String),
    Recovery,
}

#[component]
pub fn Vault() -> impl IntoView {
    let (state, set_state) = signal(VaultState::Locked);
    let (passphrase, set_passphrase) = signal(String::new());
    let (confirm, set_confirm) = signal(String::new());
    let (tab, set_tab) = signal(0u8); // 0 = create, 1 = unlock, 2 = recovery

    let input = {
        let setter = set_passphrase.clone();
        move |ev: web_sys::Event| {
            if let Some(el) = ev.target() {
                if let Ok(input) = el.dyn_into::<web_sys::HtmlInputElement>() {
                    setter.set(input.value());
                }
            }
        }
    };

    let confirm_input = {
        move |ev: web_sys::Event| {
            if let Some(el) = ev.target() {
                if let Ok(input) = el.dyn_into::<web_sys::HtmlInputElement>() {
                    set_confirm.set(input.value());
                }
            }
        }
    };

    let on_create = {
        let confirm = confirm.clone();
        let passphrase = passphrase.clone();
        move |_| {
            let p = passphrase.get();
            let c = confirm.get();
            if p.is_empty() {
                set_state.set(VaultState::Error("Passphrase is required.".into()));
            } else if p != c {
                set_state.set(VaultState::Error("Passphrases do not match.".into()));
            } else {
                set_state.set(VaultState::Creating);
            }
        }
    };

    let on_unlock = {
        move |_| {
            if passphrase.get().is_empty() {
                set_state.set(VaultState::Error("Passphrase is required.".into()));
            } else {
                set_state.set(VaultState::Unlocking);
            }
        }
    };

    view! {
        <div class="vault">
            <h2>"Vault"</h2>

            <div class="tabs">
                <button on:click=move |_| set_tab.set(0) class:selected=move || tab.get() == 0>
                    "Create"
                </button>
                <button on:click=move |_| set_tab.set(1) class:selected=move || tab.get() == 1>
                    "Unlock"
                </button>
                <button on:click=move |_| set_tab.set(2) class:selected=move || tab.get() == 2>
                    "Recovery"
                </button>
            </div>

            {move || match state.get() {
                VaultState::Error(e) => view! { <p class="error">{e}</p> }.into_any(),
                VaultState::Creating => view! { <p>"Creating vault…"</p> }.into_any(),
                VaultState::Unlocking => view! { <p>"Unlocking vault…"</p> }.into_any(),
                VaultState::Unlocked { vault_id } => view! {
                    <p class="success">"Unlocked vault: " {vault_id}</p>
                }.into_any(),
                _ => view! {}.into_any(),
            }}

            <div class="panel" class:hidden=move || tab.get() != 0>
                <input type="password" placeholder="Passphrase" on:input=input />
                <input type="password" placeholder="Confirm passphrase" on:input=confirm_input />
                <button on:click=on_create>
                    "Create vault"
                </button>
            </div>

            <div class="panel" class:hidden=move || tab.get() != 1>
                <input type="password" placeholder="Passphrase" on:input=input />
                <button on:click=on_unlock>
                    "Unlock vault"
                </button>
            </div>

            <div class="panel" class:hidden=move || tab.get() != 2>
                <p>"Enter a recovery phrase to restore access."</p>
                <input type="text" placeholder="Recovery phrase" on:input=input />
                <button on:click=move |_| set_state.set(VaultState::Recovery)>
                    "Restore"
                </button>
            </div>
        </div>
    }
}
