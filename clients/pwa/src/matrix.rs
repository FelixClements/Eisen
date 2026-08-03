use eisen_core::{DeviceId, Hlc, Mutation, Task, TaskId, TaskStore};
use leptos::prelude::*;

const QUADRANT_LABELS: [&str; 4] = [
    "Urgent & Important",
    "Important, Not Urgent",
    "Urgent, Not Important",
    "Not Urgent, Not Important",
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
    set_editing: WriteSignal<Option<Task>>,
) -> impl IntoView {
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
                                            let title = task
                                                .title
                                                .value
                                                .as_deref()
                                                .unwrap_or("Untitled")
                                                .to_string();
                                            view! {
                                                <li
                                                    class="task-item"
                                                    class:completed=task.is_completed()
                                                >
                                                    <span class="task-title">{title}</span>
                                                    <button
                                                        class="edit"
                                                        on:click=move |_| {
                                                            set_editing.set(Some(task_for_edit.clone()));
                                                        }
                                                    >
                                                        "Edit"
                                                    </button>
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
