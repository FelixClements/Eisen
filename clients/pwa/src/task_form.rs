use eisen_core::{DeviceId, Hlc, Mutation, Task, TaskId, TaskStore};
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{HtmlInputElement, HtmlSelectElement, HtmlTextAreaElement};

#[component]
pub fn TaskForm(
    store: ReadSignal<TaskStore>,
    set_store: WriteSignal<TaskStore>,
    editing: ReadSignal<Option<Task>>,
    set_editing: WriteSignal<Option<Task>>,
) -> impl IntoView {
    let (title, set_title) = signal(String::new());
    let (notes, set_notes) = signal(String::new());
    let (quadrant, set_quadrant) = signal(0u8);
    let (error, set_error) = signal(None::<String>);
    let (next_id, set_next_id) = signal(1u64);
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
        }
    });

    let on_submit = {
        let set_title = set_title.clone();
        let set_notes = set_notes.clone();
        let set_quadrant = set_quadrant.clone();
        let set_error = set_error.clone();
        let set_editing = set_editing.clone();
        let set_next_id = set_next_id.clone();
        move |_| {
            let new_title = title.get();
            let new_notes = notes.get();
            let new_quadrant = quadrant.get();

            let mut s = store.get();
            let now = js_sys::Date::now() as u64;
            let id_counter = next_id.get();
            set_next_id.set(id_counter + 1);
            let hlc = Hlc {
                wall: now,
                counter: id_counter as u32,
                device_id: device,
            };

            let result = if let Some(task) = editing.get() {
                Mutation::Update {
                    hlc,
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
                let mut id_bytes = [0u8; 16];
                id_bytes[0..8].copy_from_slice(&id_counter.to_be_bytes());
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
        </div>
    }
}
