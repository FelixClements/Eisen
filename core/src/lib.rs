//! Eisen reference mutation/merge model (P1.01).
//!
//! This crate implements the deterministic, local-first task state oracle:
//! field-level LWW tuples, mutation kinds, tombstone/restore semantics, and
//! merge. It intentionally contains no encoding, crypto, or persistence logic;
//! those are P1.03, P1.05, and P1.09.

pub mod clock;

use std::cmp::Ordering;
use std::collections::BTreeMap;

/// 128-bit device identifier (UUID bytes, big-endian).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DeviceId(pub [u8; 16]);

/// 128-bit task identifier (UUID bytes, big-endian).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TaskId(pub [u8; 16]);

/// Hybrid logical clock: (wall, counter, device_id).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Hlc {
    pub wall: u64,
    pub counter: u32,
    pub device_id: DeviceId,
}

impl PartialOrd for Hlc {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Hlc {
    fn cmp(&self, other: &Self) -> Ordering {
        self.wall
            .cmp(&other.wall)
            .then_with(|| self.counter.cmp(&other.counter))
            .then_with(|| self.device_id.cmp(&other.device_id))
    }
}

/// A field value with its winning HLC.
/// `value == None` means the field has been cleared by a winning update.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Field<T> {
    pub hlc: Hlc,
    pub value: Option<T>,
}

impl<T: Clone> Field<T> {
    fn merge(&mut self, other: &Field<T>) {
        if other.hlc > self.hlc {
            *self = other.clone();
        }
    }
}

/// A task materialized from per-field LWW winners.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Task {
    pub id: TaskId,
    pub title: Field<String>,
    pub notes: Field<String>,
    pub quadrant: Field<u8>,
    pub due_date: Field<u64>,
    pub completed_at: Field<u64>,
    pub deleted_at: Field<u64>,
    pub created_at: Field<u64>,
    pub updated_at: Field<u64>,
}

impl Task {
    /// Returns true if the task is visible in the materialized view.
    /// A task is hidden while `deleted_at` is set and no later restore wins.
    pub fn is_deleted(&self) -> bool {
        self.deleted_at.value.is_some()
    }

    /// Returns true if the task is completed and not deleted/restored.
    pub fn is_completed(&self) -> bool {
        !self.is_deleted() && self.completed_at.value.is_some()
    }

    /// Merge another task's field winners into this task.
    pub fn merge(&mut self, other: &Task) {
        self.title.merge(&other.title);
        self.notes.merge(&other.notes);
        self.quadrant.merge(&other.quadrant);
        self.due_date.merge(&other.due_date);
        self.completed_at.merge(&other.completed_at);
        self.deleted_at.merge(&other.deleted_at);
        self.created_at.merge(&other.created_at);
        self.updated_at.merge(&other.updated_at);
    }
}

/// Mutation kinds supported by the reference model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Mutation {
    /// Create a new task. Fails if the task already exists locally.
    Create {
        hlc: Hlc,
        id: TaskId,
        title: String,
        notes: Option<String>,
        quadrant: u8,
        due_date: Option<u64>,
    },
    /// Update one or more fields. `Some(None)` clears a field.
    Update {
        hlc: Hlc,
        id: TaskId,
        title: Option<String>,
        notes: Option<Option<String>>,
        quadrant: Option<u8>,
        due_date: Option<Option<u64>>,
    },
    /// Mark the task completed at `hlc`.
    Complete { hlc: Hlc, id: TaskId },
    /// Clear `completed_at` and `deleted_at` at `hlc`.
    Restore { hlc: Hlc, id: TaskId },
    /// Set `deleted_at` to `hlc`.
    Delete { hlc: Hlc, id: TaskId },
    /// Local-only request to remove the task from the materialized view and log.
    Purge { id: TaskId },
}

impl Mutation {
    /// Task identifier targeted by this mutation.
    pub fn task_id(&self) -> TaskId {
        match self {
            Mutation::Create { id, .. }
            | Mutation::Update { id, .. }
            | Mutation::Complete { id, .. }
            | Mutation::Restore { id, .. }
            | Mutation::Delete { id, .. }
            | Mutation::Purge { id } => *id,
        }
    }

    /// HLC of the mutation, if any (purge has no HLC).
    pub fn hlc(&self) -> Option<Hlc> {
        match self {
            Mutation::Create { hlc, .. }
            | Mutation::Update { hlc, .. }
            | Mutation::Complete { hlc, .. }
            | Mutation::Restore { hlc, .. }
            | Mutation::Delete { hlc, .. } => Some(*hlc),
            Mutation::Purge { .. } => None,
        }
    }
}

/// Errors returned by the reference model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelError {
    TaskAlreadyExists,
    TaskNotFound,
    TitleTooLong,
    TitleEmpty,
    NotesTooLong,
    InvalidQuadrant,
    NotDeleted,
}

impl std::fmt::Display for ModelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModelError::TaskAlreadyExists => write!(f, "task already exists"),
            ModelError::TaskNotFound => write!(f, "task not found"),
            ModelError::TitleTooLong => write!(f, "title exceeds 256 UTF-8 bytes"),
            ModelError::TitleEmpty => write!(f, "title is empty"),
            ModelError::NotesTooLong => write!(f, "notes exceed 4096 UTF-8 bytes"),
            ModelError::InvalidQuadrant => write!(f, "quadrant must be 0..=3"),
            ModelError::NotDeleted => write!(f, "purge requires the task to be deleted"),
        }
    }
}

impl std::error::Error for ModelError {}

/// Enforce task field bounds.
fn validate_title(title: &str) -> Result<(), ModelError> {
    if title.is_empty() {
        return Err(ModelError::TitleEmpty);
    }
    if title.len() > 256 {
        return Err(ModelError::TitleTooLong);
    }
    Ok(())
}

fn validate_notes(notes: &Option<String>) -> Result<(), ModelError> {
    if let Some(n) = notes {
        if n.len() > 4096 {
            return Err(ModelError::NotesTooLong);
        }
    }
    Ok(())
}

fn validate_quadrant(q: u8) -> Result<(), ModelError> {
    if q > 3 {
        return Err(ModelError::InvalidQuadrant);
    }
    Ok(())
}

/// Local task store / materialized view.
#[derive(Clone, Debug, Default)]
pub struct TaskStore {
    tasks: BTreeMap<TaskId, Task>,
}

impl TaskStore {
    /// Create an empty store.
    pub fn new() -> Self {
        Self {
            tasks: BTreeMap::new(),
        }
    }

    /// Returns the number of visible tasks in the store.
    pub fn len(&self) -> usize {
        self.tasks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    /// Get a task by id.
    pub fn get(&self, id: TaskId) -> Option<&Task> {
        self.tasks.get(&id)
    }

    /// Apply a single mutation to the local store.
    ///
    /// Create, Update, Complete, Restore, and Delete modify or insert a task.
    /// Purge removes the task from the local store entirely.
    pub fn apply(&mut self, mutation: Mutation) -> Result<(), ModelError> {
        match mutation {
            Mutation::Create {
                hlc,
                id,
                title,
                notes,
                quadrant,
                due_date,
            } => {
                if self.tasks.contains_key(&id) {
                    return Err(ModelError::TaskAlreadyExists);
                }
                validate_title(&title)?;
                validate_notes(&notes)?;
                validate_quadrant(quadrant)?;

                let task = Task {
                    id,
                    title: Field {
                        hlc,
                        value: Some(title),
                    },
                    notes: Field { hlc, value: notes },
                    quadrant: Field {
                        hlc,
                        value: Some(quadrant),
                    },
                    due_date: Field {
                        hlc,
                        value: due_date,
                    },
                    completed_at: Field { hlc, value: None },
                    deleted_at: Field { hlc, value: None },
                    created_at: Field {
                        hlc,
                        value: Some(hlc.wall),
                    },
                    updated_at: Field {
                        hlc,
                        value: Some(hlc.wall),
                    },
                };
                self.tasks.insert(id, task);
                Ok(())
            }
            Mutation::Update {
                hlc,
                id,
                title,
                notes,
                quadrant,
                due_date,
            } => {
                let task = self.tasks.get_mut(&id).ok_or(ModelError::TaskNotFound)?;

                if let Some(t) = title.as_ref() {
                    validate_title(t)?;
                    task.title = Field {
                        hlc,
                        value: Some(t.clone()),
                    };
                }
                if let Some(n) = notes {
                    validate_notes(&n)?;
                    task.notes = Field { hlc, value: n };
                }
                if let Some(q) = quadrant {
                    validate_quadrant(q)?;
                    task.quadrant = Field {
                        hlc,
                        value: Some(q),
                    };
                }
                if let Some(d) = due_date {
                    task.due_date = Field { hlc, value: d };
                }
                task.updated_at = Field {
                    hlc,
                    value: Some(hlc.wall),
                };
                Ok(())
            }
            Mutation::Complete { hlc, id } => {
                let task = self.tasks.get_mut(&id).ok_or(ModelError::TaskNotFound)?;
                task.completed_at = Field {
                    hlc,
                    value: Some(hlc.wall),
                };
                task.updated_at = Field {
                    hlc,
                    value: Some(hlc.wall),
                };
                Ok(())
            }
            Mutation::Restore { hlc, id } => {
                let task = self.tasks.get_mut(&id).ok_or(ModelError::TaskNotFound)?;
                task.completed_at = Field { hlc, value: None };
                task.deleted_at = Field { hlc, value: None };
                task.updated_at = Field {
                    hlc,
                    value: Some(hlc.wall),
                };
                Ok(())
            }
            Mutation::Delete { hlc, id } => {
                let task = self.tasks.get_mut(&id).ok_or(ModelError::TaskNotFound)?;
                task.deleted_at = Field {
                    hlc,
                    value: Some(hlc.wall),
                };
                task.updated_at = Field {
                    hlc,
                    value: Some(hlc.wall),
                };
                Ok(())
            }
            Mutation::Purge { id } => {
                let task = self.tasks.get(&id).ok_or(ModelError::TaskNotFound)?;
                if task.deleted_at.value.is_none() {
                    return Err(ModelError::NotDeleted);
                }
                self.tasks.remove(&id);
                Ok(())
            }
        }
    }

    /// Merge a received task snapshot/state into the local store.
    pub fn merge_task(&mut self, other: &Task) {
        if let Some(task) = self.tasks.get_mut(&other.id) {
            task.merge(other);
        } else {
            self.tasks.insert(other.id, other.clone());
        }
    }

    /// Iterate over all visible tasks.
    pub fn values(&self) -> impl Iterator<Item = &Task> {
        self.tasks.values()
    }
}

/// Deterministic merge of a sequence of mutations targeting the same task.
///
/// Purge mutations cause the function to return `Ok(None)` and remove the task.
/// This is intended for local log replay; remote merges should not include purge.
pub fn merge_mutations(mutations: &[Mutation]) -> Result<Option<Task>, ModelError> {
    let mut store = TaskStore::new();
    for m in mutations {
        if let Mutation::Purge { id } = m {
            if store.tasks.contains_key(id) {
                store.tasks.remove(id);
                return Ok(None);
            }
            continue;
        }
        store.apply(m.clone())?;
    }
    Ok(store.tasks.values().next().cloned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dev(n: u8) -> DeviceId {
        let mut bytes = [0u8; 16];
        bytes[15] = n;
        DeviceId(bytes)
    }

    fn hlc(wall: u64, counter: u32, device: u8) -> Hlc {
        Hlc {
            wall,
            counter,
            device_id: dev(device),
        }
    }

    #[test]
    fn create_and_read_task() {
        let mut store = TaskStore::new();
        let id = TaskId([1; 16]);
        store
            .apply(Mutation::Create {
                hlc: hlc(1, 0, 1),
                id,
                title: "Buy milk".into(),
                notes: None,
                quadrant: 0,
                due_date: None,
            })
            .unwrap();

        let task = store.get(id).unwrap();
        assert_eq!(task.title.value.as_ref().unwrap(), "Buy milk");
        assert_eq!(task.quadrant.value, Some(0));
        assert!(!task.is_deleted());
    }

    #[test]
    fn complete_and_restore() {
        let mut store = TaskStore::new();
        let id = TaskId([2; 16]);
        store
            .apply(Mutation::Create {
                hlc: hlc(1, 0, 1),
                id,
                title: "Task".into(),
                notes: None,
                quadrant: 1,
                due_date: None,
            })
            .unwrap();
        store
            .apply(Mutation::Complete {
                hlc: hlc(2, 0, 1),
                id,
            })
            .unwrap();
        assert!(store.get(id).unwrap().is_completed());

        store
            .apply(Mutation::Restore {
                hlc: hlc(3, 0, 1),
                id,
            })
            .unwrap();
        let task = store.get(id).unwrap();
        assert!(!task.is_completed());
        assert!(task.completed_at.value.is_none());
    }

    #[test]
    fn delete_and_purge() {
        let mut store = TaskStore::new();
        let id = TaskId([3; 16]);
        store
            .apply(Mutation::Create {
                hlc: hlc(1, 0, 1),
                id,
                title: "Task".into(),
                notes: None,
                quadrant: 2,
                due_date: None,
            })
            .unwrap();
        store
            .apply(Mutation::Delete {
                hlc: hlc(2, 0, 1),
                id,
            })
            .unwrap();
        assert!(store.get(id).unwrap().is_deleted());

        store.apply(Mutation::Purge { id }).unwrap();
        assert!(store.get(id).is_none());
    }

    #[test]
    fn lww_merge_prefers_higher_hlc() {
        let id = TaskId([4; 16]);
        let mut store = TaskStore::new();
        store
            .apply(Mutation::Create {
                hlc: hlc(1, 0, 1),
                id,
                title: "Original".into(),
                notes: None,
                quadrant: 0,
                due_date: None,
            })
            .unwrap();

        let other = Task {
            id,
            title: Field {
                hlc: hlc(2, 0, 2),
                value: Some("Updated".into()),
            },
            notes: Field {
                hlc: hlc(1, 0, 1),
                value: None,
            },
            quadrant: Field {
                hlc: hlc(1, 0, 1),
                value: Some(0),
            },
            due_date: Field {
                hlc: hlc(1, 0, 1),
                value: None,
            },
            completed_at: Field {
                hlc: hlc(1, 0, 1),
                value: None,
            },
            deleted_at: Field {
                hlc: hlc(1, 0, 1),
                value: None,
            },
            created_at: Field {
                hlc: hlc(1, 0, 1),
                value: Some(1),
            },
            updated_at: Field {
                hlc: hlc(2, 0, 2),
                value: Some(2),
            },
        };
        store.merge_task(&other);

        assert_eq!(
            store.get(id).unwrap().title.value.as_ref().unwrap(),
            "Updated"
        );
    }

    #[test]
    fn merge_mutations_replay() {
        let id = TaskId([5; 16]);
        let mutations = vec![
            Mutation::Create {
                hlc: hlc(1, 0, 1),
                id,
                title: "A".into(),
                notes: None,
                quadrant: 0,
                due_date: None,
            },
            Mutation::Update {
                hlc: hlc(2, 0, 1),
                id,
                title: Some("B".into()),
                notes: None,
                quadrant: None,
                due_date: None,
            },
            Mutation::Complete {
                hlc: hlc(3, 0, 1),
                id,
            },
        ];
        let task = merge_mutations(&mutations).unwrap().unwrap();
        assert_eq!(task.title.value.as_ref().unwrap(), "B");
        assert!(task.is_completed());
    }

    #[test]
    fn hlc_tie_breaker_uses_device_id() {
        let id = TaskId([6; 16]);
        let mut store = TaskStore::new();
        store
            .apply(Mutation::Create {
                hlc: Hlc {
                    wall: 1,
                    counter: 0,
                    device_id: DeviceId([0; 16]),
                },
                id,
                title: "First".into(),
                notes: None,
                quadrant: 0,
                due_date: None,
            })
            .unwrap();

        let other = Task {
            id,
            title: Field {
                hlc: Hlc {
                    wall: 1,
                    counter: 0,
                    device_id: DeviceId([1; 16]),
                },
                value: Some("Second".into()),
            },
            notes: Field {
                hlc: Hlc {
                    wall: 1,
                    counter: 0,
                    device_id: DeviceId([0; 16]),
                },
                value: None,
            },
            quadrant: Field {
                hlc: Hlc {
                    wall: 1,
                    counter: 0,
                    device_id: DeviceId([0; 16]),
                },
                value: Some(0),
            },
            due_date: Field {
                hlc: Hlc {
                    wall: 1,
                    counter: 0,
                    device_id: DeviceId([0; 16]),
                },
                value: None,
            },
            completed_at: Field {
                hlc: Hlc {
                    wall: 1,
                    counter: 0,
                    device_id: DeviceId([0; 16]),
                },
                value: None,
            },
            deleted_at: Field {
                hlc: Hlc {
                    wall: 1,
                    counter: 0,
                    device_id: DeviceId([0; 16]),
                },
                value: None,
            },
            created_at: Field {
                hlc: Hlc {
                    wall: 1,
                    counter: 0,
                    device_id: DeviceId([0; 16]),
                },
                value: Some(1),
            },
            updated_at: Field {
                hlc: Hlc {
                    wall: 1,
                    counter: 0,
                    device_id: DeviceId([1; 16]),
                },
                value: Some(1),
            },
        };
        store.merge_task(&other);

        assert_eq!(
            store.get(id).unwrap().title.value.as_ref().unwrap(),
            "Second"
        );
    }

    #[test]
    fn bounds_reject_invalid_input() {
        let mut store = TaskStore::new();
        let id = TaskId([7; 16]);
        assert!(matches!(
            store.apply(Mutation::Create {
                hlc: hlc(1, 0, 1),
                id,
                title: String::new(),
                notes: None,
                quadrant: 0,
                due_date: None,
            }),
            Err(ModelError::TitleEmpty)
        ));
        assert!(matches!(
            store.apply(Mutation::Create {
                hlc: hlc(1, 0, 1),
                id,
                title: "x".repeat(257),
                notes: None,
                quadrant: 0,
                due_date: None,
            }),
            Err(ModelError::TitleTooLong)
        ));
        assert!(matches!(
            store.apply(Mutation::Create {
                hlc: hlc(1, 0, 1),
                id,
                title: "T".into(),
                notes: Some("x".repeat(4097)),
                quadrant: 0,
                due_date: None,
            }),
            Err(ModelError::NotesTooLong)
        ));
        assert!(matches!(
            store.apply(Mutation::Create {
                hlc: hlc(1, 0, 1),
                id,
                title: "T".into(),
                notes: None,
                quadrant: 4,
                due_date: None,
            }),
            Err(ModelError::InvalidQuadrant)
        ));
    }
}
