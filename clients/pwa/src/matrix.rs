use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine as _;
use cbor2;
use eisen_core::Task;
use leptos::prelude::*;
use serde_json::json;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use web_sys::HtmlSelectElement;

use crate::bridge::WorkerClient;

const QUADRANT_LABELS: [&str; 4] = [
    "Urgent & Important",
    "Important, Not Urgent",
    "Urgent, Not Important",
    "Not Urgent, Not Important",
];

const QUADRANT_OPTIONS: [(&str, &str); 4] = [
    ("0", "Urgent & Important"),
    ("1", "Important, Not Urgent"),
    ("2", "Urgent, Not Important"),
    ("3", "Not Urgent, Not Important"),
];

fn decode_task_list(b64: &str) -> Option<Vec<Task>> {
    let bytes = B64.decode(b64).ok()?;
    cbor2::from_slice::<Vec<Task>>(&bytes).ok()
}

fn send_task_action(
    worker: &WorkerClient,
    action: &'static str,
    payload: String,
    set_tasks: WriteSignal<Vec<Task>>,
) {
    worker.send(
        action,
        "",
        &payload,
        Box::new(move |res| match res {
            Ok(b64) => {
                if let Some(list) = decode_task_list(&b64) {
                    set_tasks.set(list);
                }
            }
            Err(e) => log::error!("{action} failed: {e}"),
        }),
    );
}

#[component]
pub fn Matrix(
    worker: Rc<WorkerClient>,
    tasks: ReadSignal<Vec<Task>>,
    set_tasks: WriteSignal<Vec<Task>>,
    set_editing: WriteSignal<Option<Task>>,
) -> impl IntoView {
    let worker = StoredValue::new_local(Some(worker));

    view! {
        <section class="matrix">
            <h2>"Eisenhower Matrix"</h2>
            {move || {
                let quadrants = tasks.with(|t| {
                    let mut qs = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
                    for task in t.iter().filter(|t| !t.is_deleted()) {
                        if let Some(q) = task.quadrant.value {
                            if q < 4 {
                                qs[q as usize].push(task.clone());
                            }
                        }
                    }
                    qs
                });

                view! {
                    <div class="matrix-grid">
                        {quadrants.into_iter().enumerate().map(|(q, tasks)| {
                            view! {
                                <div class="quadrant">
                                    <h3 class="quadrant-title">{QUADRANT_LABELS[q]}</h3>
                                    <ul class="task-list">
                                        {tasks.into_iter().map(|task| {
                                            let task_for_edit = task.clone();
                                            let task_id = task.id;
                                            let title = task
                                                .title
                                                .value
                                                .as_deref()
                                                .unwrap_or("Untitled")
                                                .to_string();
                                            let is_completed = task.is_completed();

                                            view! {
                                                <li
                                                    class="task-item"
                                                    class:completed=is_completed
                                                >
                                                    <span
                                                        class="task-title"
                                                        on:click=move |_| set_editing.set(Some(task_for_edit.clone()))
                                                    >
                                                        {title}
                                                    </span>

                                                    <div class="task-actions">
                                                        <select
                                                            prop:value=task
                                                                .quadrant
                                                                .value
                                                                .unwrap_or(0)
                                                                .to_string()
                                                            on:change=move |ev| {
                                                                if let Some(el) = ev.target() {
                                                                    if let Ok(select) = el.dyn_into::<HtmlSelectElement>() {
                                                                        if let Ok(q) = select.value().parse::<u8>() {
                                                                            worker.with_value(|w| {
                                                                                if let Some(w) = w {
                                                                                    let id_b64 = B64.encode(&task_id.0);
                                                                                    let payload = serde_json::to_string(&json!({
                                                                                        "id": id_b64,
                                                                                        "quadrant": q,
                                                                                    })).unwrap_or_default();
                                                                                    send_task_action(&**w, "move_task", payload, set_tasks);
                                                                                }
                                                                            });
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        >
                                                            {QUADRANT_OPTIONS.iter().map(|(value, label)| {
                                                                view! { <option value=*value>{*label}</option> }
                                                            }).collect::<Vec<_>>()}
                                                        </select>

                                                        {(!is_completed).then(|| view! {
                                                            <button
                                                                class="complete"
                                                                on:click=move |_| {
                                                                    worker.with_value(|w| {
                                                                        if let Some(w) = w {
                                                                            let id_b64 = B64.encode(&task_id.0);
                                                                            send_task_action(&**w, "complete_task", id_b64, set_tasks);
                                                                        }
                                                                    });
                                                                }
                                                            >
                                                                "Complete"
                                                            </button>
                                                        }.into_any())}

                                                        <button
                                                            class="delete"
                                                            on:click=move |_| {
                                                                worker.with_value(|w| {
                                                                    if let Some(w) = w {
                                                                        let id_b64 = B64.encode(&task_id.0);
                                                                        send_task_action(&**w, "delete_task", id_b64, set_tasks);
                                                                    }
                                                                });
                                                            }
                                                        >
                                                            "Delete"
                                                        </button>

                                                        {(is_completed).then(|| view! {
                                                            <button
                                                                class="restore"
                                                                on:click=move |_| {
                                                                    worker.with_value(|w| {
                                                                        if let Some(w) = w {
                                                                            let id_b64 = B64.encode(&task_id.0);
                                                                            send_task_action(&**w, "restore_task", id_b64, set_tasks);
                                                                        }
                                                                    });
                                                                }
                                                            >
                                                                "Restore"
                                                            </button>
                                                        }.into_any())}

                                                        <button
                                                            class="edit"
                                                            on:click=move |_| set_editing.set(Some(task.clone()))
                                                        >
                                                            "Edit"
                                                        </button>
                                                    </div>
                                                </li>
                                            }
                                        }).collect::<Vec<_>>()}
                                    </ul>
                                </div>
                            }
                        }).collect::<Vec<_>>()}
                    </div>
                }
            }}
        </section>
    }
}
