use crate::bridge::WorkerClient;
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine as _;
use js_sys::{ArrayBuffer, Uint8Array};
use leptos::prelude::*;
use std::rc::Rc;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::{FileReader, HtmlInputElement};

#[derive(Clone)]
pub enum VaultState {
    Locked,
    Creating,
    Unlocking,
    Working(String),
    Unlocked { vault_id: String },
    Error(String),
}

fn read_file(setter: WriteSignal<String>) -> impl Fn(web_sys::Event) {
    move |ev: web_sys::Event| {
        if let Some(el) = ev
            .target()
            .and_then(|t| t.dyn_into::<HtmlInputElement>().ok())
        {
            if let Some(file) = el.files().and_then(|list| list.get(0)) {
                let reader = FileReader::new().expect("FileReader");
                let reader_clone = reader.clone();
                let onload = Closure::wrap(Box::new(move |_: web_sys::ProgressEvent| {
                    if let Ok(buffer) = reader_clone.result() {
                        if let Ok(ab) = buffer.dyn_into::<ArrayBuffer>() {
                            let bytes = Uint8Array::new(&ab).to_vec();
                            setter.set(B64.encode(&bytes));
                        }
                    }
                }) as Box<dyn FnMut(_)>);
                reader.set_onload(Some(onload.as_ref().unchecked_ref()));
                onload.forget();
                let _ = reader.read_as_array_buffer(&file);
            }
        }
    }
}

#[component]
pub fn Vault() -> impl IntoView {
    let (state, set_state) = signal(VaultState::Locked);
    let (passphrase, set_passphrase) = signal(String::new());
    let (confirm, set_confirm) = signal(String::new());
    let (tab, set_tab) = signal(0u8); // 0 = create, 1 = unlock, 2 = backup
    let (locator, set_locator) = signal(String::new());
    let (export_b64, set_export_b64) = signal(String::new());
    let (recovery_b64, set_recovery_b64) = signal(String::new());
    let (import_b64, set_import_b64) = signal(String::new());
    let (restore_b64, set_restore_b64) = signal(String::new());

    let worker = match WorkerClient::new() {
        Ok(w) => Some(Rc::new(w)),
        Err(_) => {
            set_state.set(VaultState::Error("Could not start secure worker.".into()));
            None
        }
    };
    let worker = StoredValue::new_local(worker);

    let input = {
        let setter = set_passphrase.clone();
        move |ev: web_sys::Event| {
            if let Some(el) = ev.target() {
                if let Ok(input) = el.dyn_into::<HtmlInputElement>() {
                    setter.set(input.value());
                }
            }
        }
    };

    let confirm_input = {
        move |ev: web_sys::Event| {
            if let Some(el) = ev.target() {
                if let Ok(input) = el.dyn_into::<HtmlInputElement>() {
                    set_confirm.set(input.value());
                }
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
                    "Backup & Recovery"
                </button>
            </div>

            {move || match state.get() {
                VaultState::Error(e) => view! { <p class="error">{e}</p> }.into_any(),
                VaultState::Creating => view! { <p>"Creating vault…"</p> }.into_any(),
                VaultState::Unlocking => view! { <p>"Unlocking vault…"</p> }.into_any(),
                VaultState::Working(msg) => view! { <p>{msg}</p> }.into_any(),
                VaultState::Unlocked { vault_id } => view! {
                    <p class="success">"Unlocked vault: " {vault_id}</p>
                }.into_any(),
                _ => view! {}.into_any(),
            }}

            <div class="panel" class:hidden=move || tab.get() != 0>
                <input type="password" placeholder="Passphrase" on:input=input />
                <input type="password" placeholder="Confirm passphrase" on:input=confirm_input />
                <button on:click=move |_| worker.with_value(|worker| {
                    let p = passphrase.get();
                    let c = confirm.get();
                    if p.is_empty() {
                        set_state.set(VaultState::Error("Passphrase is required.".into()));
                        return;
                    }
                    if p != c {
                        set_state.set(VaultState::Error("Passphrases do not match.".into()));
                        return;
                    }
                    if let Some(w) = worker {
                        set_state.set(VaultState::Creating);
                        w.send(
                            "create",
                            &p,
                            "",
                            Box::new(move |res| match res {
                                Ok(id) => set_state.set(VaultState::Unlocked { vault_id: id }),
                                Err(e) => set_state.set(VaultState::Error(e)),
                            }),
                        );
                    } else {
                        set_state.set(VaultState::Error(
                            "Secure worker is not available.".into(),
                        ));
                    }
                })>
                    "Create vault"
                </button>
            </div>

            <div class="panel" class:hidden=move || tab.get() != 1>
                <input type="password" placeholder="Passphrase" on:input=input />
                <button on:click=move |_| worker.with_value(|worker| {
                    let p = passphrase.get();
                    if p.is_empty() {
                        set_state.set(VaultState::Error("Passphrase is required.".into()));
                        return;
                    }
                    if let Some(w) = worker {
                        set_state.set(VaultState::Unlocking);
                        w.send(
                            "open",
                            &p,
                            "",
                            Box::new(move |res| match res {
                                Ok(id) => set_state.set(VaultState::Unlocked { vault_id: id }),
                                Err(e) => set_state.set(VaultState::Error(e)),
                            }),
                        );
                    } else {
                        set_state.set(VaultState::Error(
                            "Secure worker is not available.".into(),
                        ));
                    }
                })>
                    "Unlock vault"
                </button>
            </div>

            <div class="panel" class:hidden=move || tab.get() != 2>
                <p class="notice">
                    "Backups and recovery require the current vault passphrase. "
                    "The vault passphrase is never sent off this device."
                </p>

                <input
                    type="password"
                    placeholder="Current vault passphrase"
                    on:input=input
                />

                <h3>"Export"</h3>
                <p>"Download an encrypted file containing the local task state."</p>
                <button on:click=move |_| worker.with_value(|worker| {
                    let p = passphrase.get();
                    if p.is_empty() {
                        set_state.set(VaultState::Error("Passphrase is required.".into()));
                        return;
                    }
                    if let Some(w) = worker {
                        set_state.set(VaultState::Working("Creating export…".into()));
                        w.send(
                            "export",
                            &p,
                            "",
                            Box::new(move |res| match res {
                                Ok(b64) => {
                                    set_state.set(VaultState::Locked);
                                    set_export_b64.set(b64);
                                }
                                Err(e) => set_state.set(VaultState::Error(e)),
                            }),
                        );
                    } else {
                        set_state.set(VaultState::Error(
                            "Secure worker is not available.".into(),
                        ));
                    }
                })>
                    "Export vault"
                </button>

                {move || if !export_b64.get().is_empty() {
                    view! {
                        <a
                            class="download-link"
                            download="eisen-export.bin"
                            href=move || format!("data:application/octet-stream;base64,{}", export_b64.get())
                        >
                            "Download export file"
                        </a>
                    }.into_any()
                } else {
                    view! {}.into_any()
                }}

                <h3>"Import"</h3>
                <p>"Replace the local state with an encrypted export file."</p>
                <input type="file" on:change=read_file(set_import_b64.clone()) />
                <button on:click=move |_| worker.with_value(|worker| {
                    let p = passphrase.get();
                    let b64 = import_b64.get();
                    if p.is_empty() || b64.is_empty() {
                        set_state.set(VaultState::Error("Passphrase and export file are required.".into()));
                        return;
                    }
                    if let Some(w) = worker {
                        set_state.set(VaultState::Working("Importing…".into()));
                        w.send(
                            "import",
                            &p,
                            &b64,
                            Box::new(move |res| match res {
                                Ok(_) => set_state.set(VaultState::Working("Import completed.".into())),
                                Err(e) => set_state.set(VaultState::Error(e)),
                            }),
                        );
                    } else {
                        set_state.set(VaultState::Error(
                            "Secure worker is not available.".into(),
                        ));
                    }
                })>
                    "Import vault"
                </button>

                <h3>"Recovery package"</h3>
                <p>"Create a passphrase-encrypted package for vault recovery."</p>
                <input
                    type="text"
                    placeholder="Optional locator (e.g. a hint or name)"
                    prop:value=locator
                    on:input=move |ev| {
                        if let Some(el) = ev.target() {
                            if let Ok(input) = el.dyn_into::<HtmlInputElement>() {
                                set_locator.set(input.value());
                            }
                        }
                    }
                />
                <button on:click=move |_| worker.with_value(|worker| {
                    let p = passphrase.get();
                    let loc = locator.get();
                    if p.is_empty() {
                        set_state.set(VaultState::Error("Passphrase is required.".into()));
                        return;
                    }
                    if let Some(w) = worker {
                        set_state.set(VaultState::Working("Creating recovery package…".into()));
                        w.send(
                            "recovery",
                            &p,
                            &loc,
                            Box::new(move |res| match res {
                                Ok(b64) => {
                                    set_state.set(VaultState::Locked);
                                    set_recovery_b64.set(b64);
                                }
                                Err(e) => set_state.set(VaultState::Error(e)),
                            }),
                        );
                    } else {
                        set_state.set(VaultState::Error(
                            "Secure worker is not available.".into(),
                        ));
                    }
                })>
                    "Create recovery package"
                </button>

                {move || if !recovery_b64.get().is_empty() {
                    let name = if locator.get().is_empty() {
                        "eisen-recovery.bin".into()
                    } else {
                        format!("eisen-recovery-{}.bin", locator.get())
                    };
                    view! {
                        <a
                            class="download-link"
                            download=name
                            href=move || format!("data:application/octet-stream;base64,{}", recovery_b64.get())
                        >
                            "Download recovery package"
                        </a>
                    }.into_any()
                } else {
                    view! {}.into_any()
                }}

                <h3>"Restore from recovery"</h3>
                <p>"Restore an existing vault from a recovery package."</p>
                <input type="file" on:change=read_file(set_restore_b64.clone()) />
                <button on:click=move |_| worker.with_value(|worker| {
                    let p = passphrase.get();
                    let b64 = restore_b64.get();
                    if p.is_empty() || b64.is_empty() {
                        set_state.set(VaultState::Error("Passphrase and recovery package are required.".into()));
                        return;
                    }
                    if let Some(w) = worker {
                        set_state.set(VaultState::Working("Restoring…".into()));
                        w.send(
                            "restore",
                            &p,
                            &b64,
                            Box::new(move |res| match res {
                                Ok(id) => set_state.set(VaultState::Unlocked { vault_id: id }),
                                Err(e) => set_state.set(VaultState::Error(e)),
                            }),
                        );
                    } else {
                        set_state.set(VaultState::Error(
                            "Secure worker is not available.".into(),
                        ));
                    }
                })>
                    "Restore vault"
                </button>
            </div>
        </div>
    }
}
