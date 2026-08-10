use crate::bridge::WorkerClient;
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine as _;
use cbor2;
use eisen_core::Task;
use js_sys::{ArrayBuffer, Uint8Array};
use leptos::prelude::*;
use std::rc::Rc;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::{FileReader, HtmlInputElement};

#[derive(Clone)]
pub enum VaultState {
    Idle,
    Creating,
    Unlocking,
    Working(String),
    Locked { message: String },
    RepairRequired { message: String },
    Unlocked { vault_id: String },
    Error(String),
}

fn set_error_state(set_state: &WriteSignal<VaultState>, e: String) {
    if e.starts_with("Repair required") {
        set_state.set(VaultState::RepairRequired { message: e });
    } else if e.starts_with("Cannot unlock vault") {
        set_state.set(VaultState::Locked { message: e });
    } else {
        set_state.set(VaultState::Error(e));
    }
}

fn decode_task_list(b64: &str) -> Option<Vec<Task>> {
    let bytes = B64.decode(b64).ok()?;
    cbor2::from_slice::<Vec<Task>>(&bytes).ok()
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

async fn request_persistent_storage() -> Option<bool> {
    let window = web_sys::window()?;
    let storage = window.navigator().storage();
    let promise = storage.persist().ok()?;
    let result = wasm_bindgen_futures::JsFuture::from(promise).await.ok()?;
    result.as_bool()
}

#[component]
pub fn Vault(worker: Rc<WorkerClient>, set_tasks: WriteSignal<Vec<Task>>) -> impl IntoView {
    let (state, set_state) = signal(VaultState::Idle);
    let (last_vault_id, set_last_vault_id) = signal(String::new());
    let (passphrase, set_passphrase) = signal(String::new());
    let (confirm, set_confirm) = signal(String::new());
    let (tab, set_tab) = signal(0u8); // 0 = create, 1 = unlock, 2 = backup
    let (locator, set_locator) = signal(String::new());
    let (export_b64, set_export_b64) = signal(String::new());
    let (recovery_b64, set_recovery_b64) = signal(String::new());
    let (import_b64, set_import_b64) = signal(String::new());
    let (restore_b64, set_restore_b64) = signal(String::new());
    let (persistence_hint, set_persistence_hint) = signal(None::<String>);

    let worker = StoredValue::new_local(Some(worker));

    let load_tasks = {
        let set_tasks = set_tasks;
        move || {
            worker.with_value(|w| {
                if let Some(w) = w {
                    w.send(
                        "list",
                        "",
                        "",
                        Box::new(move |res| match res {
                            Ok(b64) => {
                                if let Some(list) = decode_task_list(&b64) {
                                    set_tasks.set(list);
                                }
                            }
                            Err(e) => log::error!("failed to load tasks: {e}"),
                        }),
                    );
                }
            })
        }
    };

    wasm_bindgen_futures::spawn_local(async move {
        if request_persistent_storage().await == Some(false) {
            set_persistence_hint.set(Some(
                "Persistent storage not granted; the browser may evict vault data when resources are low.".into(),
            ));
        }
    });

    let persist = Rc::new({
        let worker = worker;
        let state = state;
        move || {
            if state.with(|s| matches!(s, VaultState::Unlocked { .. })) {
                worker.with_value(|w| {
                    if let Some(w) = w {
                        w.send("persist", "", "", Box::new(|_| ()));
                    }
                });
            }
        }
    });

    let on_visibility = {
        let persist = persist.clone();
        Closure::wrap(Box::new(move |_: web_sys::Event| {
            if let Some(document) = web_sys::window().and_then(|w| w.document()) {
                if document.hidden() {
                    persist();
                }
            }
        }) as Box<dyn FnMut(_)>)
    };
    if let Some(window) = web_sys::window() {
        let _ = window.add_event_listener_with_callback(
            "visibilitychange",
            on_visibility.as_ref().unchecked_ref(),
        );
    }
    on_visibility.forget();

    let on_pagehide = {
        let persist = persist.clone();
        Closure::wrap(Box::new(move |_: web_sys::PageTransitionEvent| {
            persist();
        }) as Box<dyn FnMut(_)>)
    };
    if let Some(window) = web_sys::window() {
        let _ = window
            .add_event_listener_with_callback("pagehide", on_pagehide.as_ref().unchecked_ref());
    }
    on_pagehide.forget();

    let on_beforeunload = {
        let persist = persist.clone();
        Closure::wrap(Box::new(move |_: web_sys::BeforeUnloadEvent| {
            persist();
        }) as Box<dyn FnMut(_)>)
    };
    if let Some(window) = web_sys::window() {
        let _ = window.add_event_listener_with_callback(
            "beforeunload",
            on_beforeunload.as_ref().unchecked_ref(),
        );
    }
    on_beforeunload.forget();

    let on_freeze = {
        let persist = persist.clone();
        Closure::wrap(Box::new(move |_: web_sys::Event| {
            persist();
        }) as Box<dyn FnMut(_)>)
    };
    if let Some(document) = web_sys::window().and_then(|w| w.document()) {
        let _ =
            document.add_event_listener_with_callback("freeze", on_freeze.as_ref().unchecked_ref());
    }
    on_freeze.forget();

    let on_resume = {
        let persist = persist.clone();
        Closure::wrap(Box::new(move |_: web_sys::Event| {
            persist();
        }) as Box<dyn FnMut(_)>)
    };
    if let Some(document) = web_sys::window().and_then(|w| w.document()) {
        let _ =
            document.add_event_listener_with_callback("resume", on_resume.as_ref().unchecked_ref());
    }
    on_resume.forget();

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
                VaultState::Idle => view! {}.into_any(),
                VaultState::Creating => view! { <p>"Creating vault…"</p> }.into_any(),
                VaultState::Unlocking => view! { <p>"Unlocking vault…"</p> }.into_any(),
                VaultState::Working(msg) => view! { <p>{msg}</p> }.into_any(),
                VaultState::Unlocked { vault_id } => view! {
                    <p class="success">"Unlocked vault: " {vault_id}</p>
                }.into_any(),
                VaultState::Locked { message } => view! {
                    <div class="locked">
                        <p class="error">{message}</p>
                        <p>"If you have a recovery package, switch to Backup & Recovery to restore access."</p>
                        <button on:click=move |_| set_tab.set(2)>"Restore from recovery"</button>
                    </div>
                }.into_any(),
                VaultState::RepairRequired { message } => view! {
                    <div class="repair">
                        <p class="error">{message}</p>
                        <p>"The vault will not be overwritten automatically. You can try to recover from an encrypted export or a recovery package."</p>
                        <button on:click=move |_| set_tab.set(2)>"Recover from backup"</button>
                    </div>
                }.into_any(),
            }}

            {move || if let Some(h) = persistence_hint.get() {
                view! { <p class="hint">{h}</p> }.into_any()
            } else {
                view! {}.into_any()
            }}

            <div class="panel" class:hidden=move || tab.get() != 0>
                <input type="password" placeholder="Passphrase" autocomplete="off" on:input=input />
                <input type="password" placeholder="Confirm passphrase" autocomplete="off" on:input=confirm_input />
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
                        let load_tasks = load_tasks.clone();
                        w.send(
                            "create",
                            &p,
                            "",
                            Box::new(move |res| match res {
                                Ok(id) => {
                                    set_last_vault_id.set(id.clone());
                                    set_state.set(VaultState::Unlocked { vault_id: id });
                                    load_tasks();
                                }
                                Err(e) => set_error_state(&set_state, e),
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
                <input type="password" placeholder="Passphrase" autocomplete="off" on:input=input />
                <button on:click=move |_| worker.with_value(|worker| {
                    let p = passphrase.get();
                    if p.is_empty() {
                        set_state.set(VaultState::Error("Passphrase is required.".into()));
                        return;
                    }
                    if let Some(w) = worker {
                        set_state.set(VaultState::Unlocking);
                        let load_tasks = load_tasks.clone();
                        w.send(
                            "open",
                            &p,
                            "",
                            Box::new(move |res| match res {
                                Ok(id) => {
                                    set_last_vault_id.set(id.clone());
                                    set_state.set(VaultState::Unlocked { vault_id: id });
                                    load_tasks();
                                }
                                Err(e) => set_error_state(&set_state, e),
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
                    autocomplete="off"


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
                                    let v = last_vault_id.get_untracked();
                                    if !v.is_empty() {
                                        set_state.set(VaultState::Unlocked { vault_id: v });
                                    } else {
                                        set_state.set(VaultState::Idle);
                                    }
                                    set_export_b64.set(b64);
                                }
                                Err(e) => set_error_state(&set_state, e),
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
                        let load_tasks = load_tasks.clone();
                        w.send(
                            "import",
                            &p,
                            &b64,
                            Box::new(move |res| match res {
                                Ok(_) => {
                                    let v = last_vault_id.get_untracked();
                                    if !v.is_empty() {
                                        set_state.set(VaultState::Unlocked { vault_id: v });
                                    } else {
                                        set_state.set(VaultState::Working("Import completed.".into()));
                                    }
                                    load_tasks();
                                }
                                Err(e) => set_error_state(&set_state, e),
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
                    autocomplete="off"


                    spellcheck="false"
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
                                    let v = last_vault_id.get_untracked();
                                    if !v.is_empty() {
                                        set_state.set(VaultState::Unlocked { vault_id: v });
                                    } else {
                                        set_state.set(VaultState::Idle);
                                    }
                                    set_recovery_b64.set(b64);
                                }
                                Err(e) => set_error_state(&set_state, e),
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
                        let load_tasks = load_tasks.clone();
                        w.send(
                            "restore",
                            &p,
                            &b64,
                            Box::new(move |res| match res {
                                Ok(id) => {
                                    set_last_vault_id.set(id.clone());
                                    set_state.set(VaultState::Unlocked { vault_id: id });
                                    load_tasks();
                                }
                                Err(e) => set_error_state(&set_state, e),
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
