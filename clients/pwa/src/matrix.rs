use eisen_core::{DeviceId, Hlc, Mutation, Task, TaskId, TaskStore};
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::HtmlSelectElement;

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

pub fn seed_store() -> TaskStore {
    let mut store = TaskStore::new();
    let device = DeviceId([1u8; 16]);
    let hlc = |wall, counter| Hlc {
        wall,
        counter,
        device_id: device,
    };

    let samples = [
        ("Plan release", 0u8),
        ("Write docs", 1u8),
        ("Reply to email", 2u8),
        ("Watch webinar", 3u8),
        ("Fix build", 0u8),
        ("Research competitors", 1u8),
    ];

    for (i, (title, quadrant)) in samples.iter().enumerate() {
        let mut id = [0u8; 16];
        id[0] = i as u8 + 1;
        let _ = store.apply(Mutation::Create {
            hlc: hlc(1, i as u32),
            id: TaskId(id),
            title: (*title).into(),
            notes: None,
            quadrant: *quadrant,
            due_date: None,
        });
    }

    store
}

#[component]
pub fn Matrix(
    store: ReadSignal<TaskStore>,
    set_store: WriteSignal<TaskStore>,
    set_editing: WriteSignal<Option<Task>>,
    next_id: ReadSignal<u64>,
    set_next_id: WriteSignal<u64>,
) -> impl IntoView {
    let device = DeviceId([1u8; 16]);

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

    let apply = {
        let store = store.clone();
        let set_store = set_store.clone();
        move |mutation: Mutation| {
            let mut s = store.get();
            if s.apply(mutation).is_ok() {
                set_store.set(s);
            }
        }
    };

    view! {
        <section class="matrix">
            <h2>"Eisenhower Matrix"</h2>
            {move || {
                let quadrants = store.with(|s| {
                    let mut qs = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
                    for task in s.values().filter(|t| !t.is_deleted()) {
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
                                                    <span class="task-title">{title}</span>

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
                                                                            let hlc = new_hlc();
                                                                            apply(Mutation::Update {
                                                                                hlc,
                                                                                id: task_id,
                                                                                title: None,
                                                                                notes: None,
                                                                                quadrant: Some(q),
                                                                                due_date: None,
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
                                                                    let hlc = new_hlc();
                                                                    apply(Mutation::Complete { hlc, id: task_id });
                                                                }
                                                            >
                                                                "Complete"
                                                            </button>
                                                        }.into_any())}

                                                        <button
                                                            class="delete"
                                                            on:click=move |_| {
                                                                let hlc = new_hlc();
                                                                apply(Mutation::Delete { hlc, id: task_id });
                                                            }
                                                        >
                                                            "Delete"
                                                        </button>

                                                        {(is_completed).then(|| view! {
                                                            <button
                                                                class="restore"
                                                                on:click=move |_| {
                                                                    let hlc = new_hlc();
                                                                    apply(Mutation::Restore { hlc, id: task_id });
                                                                }
                                                            >
                                                                "Restore"
                                                            </button>
                                                        }.into_any())}

                                                        <button
                                                            class="edit"
                                                            on:click=move |_| {
                                                                set_editing.set(Some(task_for_edit.clone()));
                                                            }
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
