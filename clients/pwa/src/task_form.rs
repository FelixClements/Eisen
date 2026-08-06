use eisen_core::{DeviceId, Field, Hlc, Mutation, Task, TaskId, TaskStore};
use js_sys::Date;
use leptos::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{HtmlInputElement, HtmlSelectElement, HtmlTextAreaElement};

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

fn has_remote_evidence(task: &Task, local: DeviceId) -> bool {
    let created_at = task.created_at.hlc;
    (task.title.hlc.device_id != local && task.title.hlc != created_at)
        || (task.notes.hlc.device_id != local && task.notes.hlc != created_at)
        || (task.quadrant.hlc.device_id != local && task.quadrant.hlc != created_at)
        || (task.due_date.hlc.device_id != local && task.due_date.hlc != created_at)
        || (task.completed_at.hlc.device_id != local && task.completed_at.hlc != created_at)
        || (task.deleted_at.hlc.device_id != local && task.deleted_at.hlc != created_at)
}

#[component]
pub fn TaskForm(
    store: ReadSignal<TaskStore>,
    set_store: WriteSignal<TaskStore>,
    editing: ReadSignal<Option<Task>>,
    set_editing: WriteSignal<Option<Task>>,
    next_id: ReadSignal<u64>,
    set_next_id: WriteSignal<u64>,
) -> impl IntoView {
    let (title, set_title) = signal(String::new());
    let (notes, set_notes) = signal(String::new());
    let (quadrant, set_quadrant) = signal(0u8);
    let (error, set_error) = signal(None::<String>);
    let (show_history, set_show_history) = signal(false);
    let device = DeviceId([1u8; 16]);

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

    let new_hlc = {
        let next_id = next_id.clone();
        let set_next_id = set_next_id.clone();
        move || {
            let now = js_sys::Date::now() as u64;
            let id = next_id.get_untracked();
            set_next_id.set(id + 1);
            Hlc {
                wall: now,
                counter: id as u32,
                device_id: device,
            }
        }
    };

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

            let mut s = store.get();

            let result = if let Some(task) = editing.get() {
                Mutation::Update {
                    hlc: new_hlc(),
                    id: task.id,
                    title: Some(new_title),
                    notes: Some(if new_notes.is_empty() {
                        None
                    } else {
                        Some(new_notes)
                    }),
                    quadrant: Some(new_quadrant),
                    due_date: None,
                }
            } else {
                let now = js_sys::Date::now() as u64;
                let id = next_id.get_untracked();
                set_next_id.set(id + 1);
                let hlc = Hlc {
                    wall: now,
                    counter: id as u32,
                    device_id: device,
                };
                let mut id_bytes = [0u8; 16];
                id_bytes[0..8].copy_from_slice(&id.to_be_bytes());
                id_bytes[8..12].copy_from_slice(&1u32.to_be_bytes());
                Mutation::Create {
                    hlc,
                    id: TaskId(id_bytes),
                    title: new_title,
                    notes: if new_notes.is_empty() {
                        None
                    } else {
                        Some(new_notes)
                    },
                    quadrant: new_quadrant,
                    due_date: None,
                }
            };

            match s.apply(result) {
                Ok(()) => {
                    set_store.set(s);
                    set_editing.set(None);
                    set_title.set(String::new());
                    set_notes.set(String::new());
                    set_quadrant.set(0);
                    set_error.set(None);
                }
                Err(e) => set_error.set(Some(e.to_string())),
            }
        }
    };

    let on_cancel = {
        let set_title = set_title.clone();
        let set_notes = set_notes.clone();
        let set_quadrant = set_quadrant.clone();
        let set_error = set_error.clone();
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
                field_evidence(&task.title, device, task.created_at.hlc, None, "Updated from another device", None)
            } else {
                view! {}.into_any()
            }}

            <textarea
                placeholder="Notes"
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
                field_evidence(&task.notes, device, task.created_at.hlc, None, "Updated from another device", None)
            } else {
                view! {}.into_any()
            }}

            <select
                prop:value=move || quadrant.get().to_string()
                on:change=move |ev| {
                    if let Some(el) = ev.target() {
                        if let Ok(select) = el.dyn_into::<HtmlSelectElement>() {
                            set_quadrant.set(select.value().parse().unwrap_or(0));
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
                field_evidence(&task.quadrant, device, task.created_at.hlc, None, "Updated from another device", None)
            } else {
                view! {}.into_any()
            }}

            <div class="form-actions">
                <button on:click=on_submit>
                    {move || if editing.get().is_some() { "Update task" } else { "Create task" }}
                </button>
                {move || if editing.get().is_some() {
                    view! {
                        <button class="secondary" on:click=on_cancel>"Cancel"</button>
                    }
                    .into_any()
                } else {
                    view! {}.into_any()
                }}
            </div>

            {move || if let Some(task) = editing.get() {
                if has_remote_evidence(&task, device) {
                    view! {
                        <button
                            class="secondary history-toggle"
                            on:click=move |_| set_show_history.set(!show_history.get())
                        >
                            {move || if show_history.get() { "Hide history" } else { "Show history" }}
                        </button>
                    }
                    .into_any()
                } else {
                    view! {}.into_any()
                }
            } else {
                view! {}.into_any()
            }}

            {move || if show_history.get() {
                editing
                    .get()
                    .map(|task| {
                        view! {
                            <div class="history">
                                <h4>"Merge history"</h4>
                                {field_evidence(&task.title, device, task.created_at.hlc, Some("Title"), "Updated from another device", None)}
                                {field_evidence(&task.notes, device, task.created_at.hlc, Some("Notes"), "Updated from another device", None)}
                                {field_evidence(&task.quadrant, device, task.created_at.hlc, Some("Quadrant"), "Updated from another device", None)}
                                {field_evidence(&task.due_date, device, task.created_at.hlc, Some("Due date"), "Updated from another device", None)}
                                {field_evidence(&task.completed_at, device, task.created_at.hlc, Some("Completed"), "Marked complete from another device", Some("Restored from another device"))}
                                {field_evidence(&task.deleted_at, device, task.created_at.hlc, Some("Deleted"), "Deleted from another device", Some("Restored from another device"))}
                            </div>
                        }
                        .into_any()
                    })
                    .unwrap_or_else(|| view! {}.into_any())
            } else {
                view! {}.into_any()
            }}
        </div>
    }
}
