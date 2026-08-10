use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine as _;
use cbor2;
use eisen_core::{DeviceId, Field, Hlc, Task};
use js_sys::Date;
use leptos::prelude::*;
use serde_json::json;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{HtmlInputElement, HtmlSelectElement, HtmlTextAreaElement};

use crate::bridge::WorkerClient;

fn short_device_label(id: &DeviceId) -> String {
    let prefix =
        id.0.iter()
            .take(2)
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
    format!("Device {}...", prefix)
}

fn format_hlc(hlc: &Hlc) -> String {
    let date = Date::new(&JsValue::from_f64(hlc.wall as f64));
    let timestamp = date
        .to_utc_string()
        .as_string()
        .unwrap_or_else(|| hlc.wall.to_string());
    let device = short_device_label(&hlc.device_id);
    format!("{} on {}", timestamp, device)
}

fn field_evidence<T>(
    field: &Field<T>,
    local: DeviceId,
    created_at: Hlc,
    label: Option<&str>,
    set_msg: &str,
    clear_msg: Option<&str>,
) -> AnyView {
    if field.hlc == created_at || field.hlc.device_id == local {
        return view! {}.into_any();
    }
    let outcome = if field.value.is_some() {
        set_msg
    } else if let Some(msg) = clear_msg {
        msg
    } else {
        set_msg
    };
    let time = format_hlc(&field.hlc);
    let text = match label {
        Some(l) => format!("{}: {} at {}", l, outcome, time),
        None => format!("{} at {}", outcome, time),
    };
    view! { <p class="merge-evidence">{text}</p> }.into_any()
}

fn has_remote_evidence(task: &Task) -> bool {
    let local = task.created_at.hlc.device_id;
    let created_at = task.created_at.hlc;
    (task.title.hlc.device_id != local && task.title.hlc != created_at)
        || (task.notes.hlc.device_id != local && task.notes.hlc != created_at)
        || (task.quadrant.hlc.device_id != local && task.quadrant.hlc != created_at)
        || (task.due_date.hlc.device_id != local && task.due_date.hlc != created_at)
        || (task.completed_at.hlc.device_id != local && task.completed_at.hlc != created_at)
        || (task.deleted_at.hlc.device_id != local && task.deleted_at.hlc != created_at)
}

fn decode_task_list(b64: &str) -> Option<Vec<Task>> {
    let bytes = B64.decode(b64).ok()?;
    cbor2::from_slice::<Vec<Task>>(&bytes).ok()
}

#[component]
pub fn TaskForm(
    worker: Rc<WorkerClient>,
    editing: ReadSignal<Option<Task>>,
    set_editing: WriteSignal<Option<Task>>,
    set_tasks: WriteSignal<Vec<Task>>,
) -> impl IntoView {
    let worker = StoredValue::new_local(Some(worker));
    let (title, set_title) = signal(String::new());
    let (notes, set_notes) = signal(String::new());
    let (quadrant, set_quadrant) = signal(0u8);
    let (error, set_error) = signal(None::<String>);
    let (show_history, set_show_history) = signal(false);

    // Populate form when a task is selected for editing.
    Effect::new({
        let set_title = set_title.clone();
        let set_notes = set_notes.clone();
        let set_quadrant = set_quadrant.clone();
        move || {
            if let Some(t) = editing.get() {
                set_title.set(t.title.value.clone().unwrap_or_default());
                set_notes.set(t.notes.value.clone().unwrap_or_default());
                set_quadrant.set(t.quadrant.value.unwrap_or(0));
            }
            set_show_history.set(false);
        }
    });

    let on_submit = {
        let set_title = set_title.clone();
        let set_notes = set_notes.clone();
        let set_quadrant = set_quadrant.clone();
        let set_error = set_error.clone();
        let set_editing = set_editing.clone();
        move |_| {
            let new_title = title.get();
            let new_notes = notes.get();
            let new_quadrant = quadrant.get();

            if new_title.is_empty() {
                set_error.set(Some("Title is required.".into()));
                return;
            }

            let (action, payload) = if let Some(task) = editing.get() {
                let id_b64 = B64.encode(&task.id.0);
                let payload = serde_json::to_string(&json!({
                    "id": id_b64,
                    "title": new_title,
                    "notes": new_notes,
                    "quadrant": new_quadrant,
                }))
                .unwrap_or_default();
                ("update_task", payload)
            } else {
                let payload = serde_json::to_string(&json!({
                    "title": new_title,
                    "notes": new_notes,
                    "quadrant": new_quadrant,
                }))
                .unwrap_or_default();
                ("create_task", payload)
            };

            worker.with_value(|w| {
                if let Some(w) = w {
                    w.send(
                        &action,
                        "",
                        &payload,
                        Box::new(move |res| match res {
                            Ok(b64) => {
                                if let Some(list) = decode_task_list(&b64) {
                                    set_tasks.set(list);
                                    set_editing.set(None);
                                    set_title.set(String::new());
                                    set_notes.set(String::new());
                                    set_quadrant.set(0);
                                    set_error.set(None);
                                }
                            }
                            Err(e) => set_error.set(Some(e)),
                        }),
                    );
                }
            });
        }
    };

    let on_cancel = {
        move |_| {
            set_editing.set(None);
            set_title.set(String::new());
            set_notes.set(String::new());
            set_quadrant.set(0);
            set_error.set(None);
        }
    };

    view! {
        <div class="task-form">
            <h3>{move || if editing.get().is_some() { "Edit task" } else { "New task" }}</h3>

            {move || {
                error.get().map(|e| view! { <p class="error">{e}</p> }.into_any())
            }}

            <input
                type="text"
                placeholder="Title"
                autocomplete="off"


                spellcheck="false"
                prop:value=title
                on:input=move |ev| {
                    if let Some(el) = ev.target() {
                        if let Ok(input) = el.dyn_into::<HtmlInputElement>() {
                            set_title.set(input.value());
                        }
                    }
                }
            />

            {move || if let Some(task) = editing.get() {
                field_evidence(&task.title, task.created_at.hlc.device_id, task.created_at.hlc, None, "Updated from another device", None)
            } else {
                view! {}.into_any()
            }}

            <textarea
                placeholder="Notes"
                autocomplete="off"


                spellcheck="false"
                prop:value=notes
                on:input=move |ev| {
                    if let Some(el) = ev.target() {
                        if let Ok(input) = el.dyn_into::<HtmlTextAreaElement>() {
                            set_notes.set(input.value());
                        }
                    }
                }
            />

            {move || if let Some(task) = editing.get() {
                field_evidence(&task.notes, task.created_at.hlc.device_id, task.created_at.hlc, None, "Updated from another device", None)
            } else {
                view! {}.into_any()
            }}

            <select
                prop:value=move || quadrant.get().to_string()
                on:change=move |ev| {
                    if let Some(el) = ev.target() {
                        if let Ok(select) = el.dyn_into::<HtmlSelectElement>() {
                            if let Ok(q) = select.value().parse::<u8>() {
                                set_quadrant.set(q);
                            }
                        }
                    }
                }
            >
                <option value="0">"Urgent & Important"</option>
                <option value="1">"Important, Not Urgent"</option>
                <option value="2">"Urgent, Not Important"</option>
                <option value="3">"Not Urgent, Not Important"</option>
            </select>

            {move || if let Some(task) = editing.get() {
                field_evidence(&task.quadrant, task.created_at.hlc.device_id, task.created_at.hlc, None, "Quadrant changed from another device", None)
            } else {
                view! {}.into_any()
            }}

            <div class="form-actions">
                <button on:click=on_submit>{move || if editing.get().is_some() { "Save" } else { "Add" }}</button>
                {move || if editing.get().is_some() {
                    view! {
                        <>
                            <button on:click=on_cancel class="secondary">"Cancel"</button>
                            <button on:click=move |_| set_show_history.set(!show_history.get()) class="secondary">
                                {move || if show_history.get() { "Hide history" } else { "Show history" }}
                            </button>
                        </>
                    }.into_any()
                } else {
                    view! {}.into_any()
                }}
            </div>

            {move || if show_history.get() {
                if let Some(task) = editing.get() {
                    if has_remote_evidence(&task) {
                        view! {
                            <div class="history">
                                <h4>"Field history"</h4>
                                {field_evidence(&task.title, task.created_at.hlc.device_id, task.created_at.hlc, Some("Title"), "Set to:", Some("Cleared"))}
                                {field_evidence(&task.notes, task.created_at.hlc.device_id, task.created_at.hlc, Some("Notes"), "Set to:", Some("Cleared"))}
                                {field_evidence(&task.quadrant, task.created_at.hlc.device_id, task.created_at.hlc, Some("Quadrant"), "Changed", None)}
                                {field_evidence(&task.completed_at, task.created_at.hlc.device_id, task.created_at.hlc, Some("Completed"), "Completed", None)}
                                {field_evidence(&task.deleted_at, task.created_at.hlc.device_id, task.created_at.hlc, Some("Deleted"), "Deleted", None)}
                            </div>
                        }.into_any()
                    } else {
                        view! { <p class="merge-evidence">"No remote changes for this task."</p> }.into_any()
                    }
                } else {
                    view! {}.into_any()
                }
            } else {
                view! {}.into_any()
            }}
        </div>
    }
}
