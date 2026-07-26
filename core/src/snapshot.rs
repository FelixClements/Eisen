//! Local snapshot persistence and replay (P1.07).
//!
//! A snapshot captures the encrypted materialized task view plus the mutation
//! log. It can be written to and read from the epoch-scoped encrypted snapshot
//! store, and replayed deterministically to rebuild the TaskStore state.

use crate::canonical::{self, Limits};
use crate::envelope::{mutation_to_value, value_to_mutation};
use crate::epoch::{AeadSnapshot, EpochError, SnapshotStore};
use crate::{Field, Hlc, ModelError, Mutation, Task, TaskId, TaskStore};
use cbor2::Value;

/// Errors returned by snapshot operations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SnapshotError {
    Encode(String),
    Decode(String),
    Crypto(EpochError),
    Model(ModelError),
    InvalidFormat,
    UnsupportedVersion(u64),
}

impl std::fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SnapshotError::Encode(s) => write!(f, "snapshot encode error: {s}"),
            SnapshotError::Decode(s) => write!(f, "snapshot decode error: {s}"),
            SnapshotError::Crypto(e) => write!(f, "snapshot crypto error: {e}"),
            SnapshotError::Model(e) => write!(f, "snapshot model error: {e}"),
            SnapshotError::InvalidFormat => write!(f, "snapshot format is invalid"),
            SnapshotError::UnsupportedVersion(v) => {
                write!(f, "unsupported snapshot version: {v}")
            }
        }
    }
}

impl std::error::Error for SnapshotError {}

impl From<crate::canonical::CanonError> for SnapshotError {
    fn from(e: crate::canonical::CanonError) -> Self {
        SnapshotError::Decode(e.to_string())
    }
}

impl From<crate::envelope::EnvelopeError> for SnapshotError {
    fn from(e: crate::envelope::EnvelopeError) -> Self {
        SnapshotError::Decode(e.to_string())
    }
}

impl From<crate::epoch::EpochError> for SnapshotError {
    fn from(e: crate::epoch::EpochError) -> Self {
        SnapshotError::Crypto(e)
    }
}

impl From<crate::ModelError> for SnapshotError {
    fn from(e: crate::ModelError) -> Self {
        SnapshotError::Model(e)
    }
}

/// Current snapshot protocol version.
const SNAPSHOT_VERSION: u64 = 1;

/// A materialized task view plus the mutation log needed to replay it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub protocol_version: u64,
    pub tasks: Vec<Task>,
    pub log: Vec<Mutation>,
}

impl Snapshot {
    /// Create a snapshot from a materialized store and its mutation log.
    pub fn from_store(store: &TaskStore, log: &[Mutation]) -> Self {
        Self {
            protocol_version: SNAPSHOT_VERSION,
            tasks: store.values().cloned().collect(),
            log: log.to_vec(),
        }
    }

    /// Replay the mutation log into a fresh store.
    ///
    /// The resulting store should match the materialized `tasks` view for a
    /// well-formed snapshot.
    pub fn replay(&self) -> Result<TaskStore, ModelError> {
        let mut store = TaskStore::new();
        for mutation in &self.log {
            store.apply(mutation.clone())?;
        }
        Ok(store)
    }

    /// Serialize the snapshot to canonical CBOR bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>, SnapshotError> {
        let value = snapshot_to_value(self)?;
        canonical::encode(&value).map_err(SnapshotError::from)
    }

    /// Parse a snapshot from canonical CBOR bytes and validate the format.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SnapshotError> {
        let value = canonical::parse(bytes, &Limits::default())?;
        value_to_snapshot(&value)
    }
}

/// Manager that owns a `TaskStore`, its mutation log, and encrypted persistence.
pub struct SnapshotManager<'a> {
    store: TaskStore,
    log: Vec<Mutation>,
    snapshot_store: SnapshotStore<'a>,
}

impl<'a> SnapshotManager<'a> {
    /// Create a new empty manager bound to a snapshot store.
    pub fn new(snapshot_store: SnapshotStore<'a>) -> Self {
        Self {
            store: TaskStore::new(),
            log: Vec::new(),
            snapshot_store,
        }
    }

    /// Load a previously saved snapshot from the encrypted store and replay it.
    pub fn load(snapshot_store: SnapshotStore<'a>) -> Result<Self, SnapshotError> {
        let plaintext = snapshot_store.load()?;
        let snapshot = Snapshot::from_bytes(&plaintext)?;
        let store = snapshot.replay()?;
        Ok(Self {
            store,
            log: snapshot.log,
            snapshot_store,
        })
    }

    /// Apply a mutation to the in-memory store and append it to the log.
    pub fn apply(&mut self, mutation: Mutation) -> Result<(), ModelError> {
        self.store.apply(mutation.clone())?;
        self.log.push(mutation);
        Ok(())
    }

    /// Persist the current store and log as an encrypted snapshot.
    pub fn save(&mut self) -> Result<AeadSnapshot, SnapshotError> {
        let snapshot = Snapshot::from_store(&self.store, &self.log);
        let bytes = snapshot.to_bytes()?;
        self.snapshot_store
            .store(&bytes)
            .map_err(SnapshotError::from)
    }

    /// Borrow the materialized task store.
    pub fn store(&self) -> &TaskStore {
        &self.store
    }

    /// Borrow the mutation log.
    pub fn log(&self) -> &[Mutation] {
        &self.log
    }
}

fn text(s: &str) -> Value {
    Value::Text(s.into())
}

fn bytes(b: &[u8]) -> Value {
    Value::Bytes(b.to_vec())
}

fn get_field(map: &[(Value, Value)], key: &str) -> Result<Value, SnapshotError> {
    map.iter()
        .find(|(k, _)| matches!(k, Value::Text(s) if s == key))
        .map(|(_, v)| v.clone())
        .ok_or(SnapshotError::InvalidFormat)
}

fn to_text(v: &Value) -> Result<String, SnapshotError> {
    match v {
        Value::Text(s) => Ok(s.clone()),
        _ => Err(SnapshotError::InvalidFormat),
    }
}

fn to_u64(v: &Value) -> Result<u64, SnapshotError> {
    match v {
        Value::Integer(i) => {
            let n: i128 = (*i).into();
            if n < 0 || n > u64::MAX as i128 {
                return Err(SnapshotError::InvalidFormat);
            }
            Ok(n as u64)
        }
        _ => Err(SnapshotError::InvalidFormat),
    }
}

fn to_u8(v: &Value) -> Result<u8, SnapshotError> {
    match v {
        Value::Integer(i) => {
            let n: i128 = (*i).into();
            if n < 0 || n > u8::MAX as i128 {
                return Err(SnapshotError::InvalidFormat);
            }
            Ok(n as u8)
        }
        _ => Err(SnapshotError::InvalidFormat),
    }
}

fn hlc_to_value(hlc: &Hlc) -> Value {
    Value::Array(vec![
        Value::Integer((hlc.wall).into()),
        Value::Integer((hlc.counter).into()),
        bytes(&hlc.device_id.0),
    ])
}

fn value_to_hlc(v: &Value) -> Result<Hlc, SnapshotError> {
    match v {
        Value::Array(arr) if arr.len() == 3 => {
            let wall = to_u64(&arr[0])?;
            let counter = to_u64(&arr[1])? as u32;
            let device_id = match &arr[2] {
                Value::Bytes(b) if b.len() == 16 => {
                    let arr: [u8; 16] = b.clone().try_into().unwrap();
                    crate::DeviceId(arr)
                }
                _ => return Err(SnapshotError::InvalidFormat),
            };
            Ok(Hlc {
                wall,
                counter,
                device_id,
            })
        }
        _ => Err(SnapshotError::InvalidFormat),
    }
}

fn value_to_task_id(v: &Value) -> Result<TaskId, SnapshotError> {
    match v {
        Value::Bytes(b) if b.len() == 16 => {
            let arr: [u8; 16] = b.clone().try_into().unwrap();
            Ok(TaskId(arr))
        }
        _ => Err(SnapshotError::InvalidFormat),
    }
}

fn field_to_value<T>(field: &Field<T>, value_to_cbor: impl Fn(&T) -> Value) -> Value {
    let value = match &field.value {
        Some(v) => Value::Array(vec![value_to_cbor(v)]),
        None => Value::Null,
    };
    Value::Map(vec![
        (text("hlc"), hlc_to_value(&field.hlc)),
        (text("value"), value),
    ])
}

fn value_to_field<T>(
    v: &Value,
    value_from_cbor: impl Fn(&Value) -> Result<T, SnapshotError>,
) -> Result<Field<T>, SnapshotError> {
    match v {
        Value::Map(map) => {
            let hlc = value_to_hlc(&get_field(map, "hlc")?)?;
            let value = match get_field(map, "value")? {
                Value::Array(arr) if arr.len() == 1 => Some(value_from_cbor(&arr[0])?),
                Value::Null => None,
                _ => return Err(SnapshotError::InvalidFormat),
            };
            Ok(Field { hlc, value })
        }
        _ => Err(SnapshotError::InvalidFormat),
    }
}

fn string_to_value(s: &String) -> Value {
    Value::Text(s.clone())
}

fn u8_to_value(q: &u8) -> Value {
    Value::Integer((*q).into())
}

fn u64_to_value(n: &u64) -> Value {
    Value::Integer((*n).into())
}

fn task_to_value(task: &Task) -> Result<Value, SnapshotError> {
    Ok(Value::Map(vec![
        (text("id"), bytes(&task.id.0)),
        (text("t"), field_to_value(&task.title, string_to_value)),
        (text("n"), field_to_value(&task.notes, string_to_value)),
        (text("q"), field_to_value(&task.quadrant, u8_to_value)),
        (text("d"), field_to_value(&task.due_date, u64_to_value)),
        (text("c"), field_to_value(&task.completed_at, u64_to_value)),
        (text("x"), field_to_value(&task.deleted_at, u64_to_value)),
        (text("r"), field_to_value(&task.created_at, u64_to_value)),
        (text("u"), field_to_value(&task.updated_at, u64_to_value)),
    ]))
}

fn value_to_task(v: &Value) -> Result<Task, SnapshotError> {
    match v {
        Value::Map(map) => Ok(Task {
            id: value_to_task_id(&get_field(map, "id")?)?,
            title: value_to_field(&get_field(map, "t")?, |v| to_text(v))?,
            notes: value_to_field(&get_field(map, "n")?, |v| to_text(v))?,
            quadrant: value_to_field(&get_field(map, "q")?, |v| to_u8(v))?,
            due_date: value_to_field(&get_field(map, "d")?, |v| to_u64(v))?,
            completed_at: value_to_field(&get_field(map, "c")?, |v| to_u64(v))?,
            deleted_at: value_to_field(&get_field(map, "x")?, |v| to_u64(v))?,
            created_at: value_to_field(&get_field(map, "r")?, |v| to_u64(v))?,
            updated_at: value_to_field(&get_field(map, "u")?, |v| to_u64(v))?,
        }),
        _ => Err(SnapshotError::InvalidFormat),
    }
}

fn snapshot_to_value(snapshot: &Snapshot) -> Result<Value, SnapshotError> {
    let tasks = snapshot
        .tasks
        .iter()
        .map(task_to_value)
        .collect::<Result<Vec<_>, _>>()?;
    let log = snapshot
        .log
        .iter()
        .map(mutation_to_value)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Value::Map(vec![
        (
            text("version"),
            Value::Integer(snapshot.protocol_version.into()),
        ),
        (text("tasks"), Value::Array(tasks)),
        (text("log"), Value::Array(log)),
    ]))
}

fn value_to_snapshot(v: &Value) -> Result<Snapshot, SnapshotError> {
    match v {
        Value::Map(map) => {
            let version = to_u64(&get_field(map, "version")?)?;
            if version != SNAPSHOT_VERSION {
                return Err(SnapshotError::UnsupportedVersion(version));
            }

            let tasks = match get_field(map, "tasks")? {
                Value::Array(arr) => arr
                    .iter()
                    .map(value_to_task)
                    .collect::<Result<Vec<_>, _>>()?,
                _ => return Err(SnapshotError::InvalidFormat),
            };

            let log = match get_field(map, "log")? {
                Value::Array(arr) => arr
                    .iter()
                    .map(value_to_mutation)
                    .collect::<Result<Vec<_>, _>>()?,
                _ => return Err(SnapshotError::InvalidFormat),
            };

            Ok(Snapshot {
                protocol_version: version,
                tasks,
                log,
            })
        }
        _ => Err(SnapshotError::InvalidFormat),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::epoch::EpochRoot;
    use crate::identity::{InMemorySecureStorage, SecureStorage};
    use crate::{DeviceId, Hlc, TaskId};

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

    fn make_store() -> (TaskStore, Vec<Mutation>) {
        let mut store = TaskStore::new();
        let id = TaskId([1; 16]);
        let id2 = TaskId([2; 16]);

        let log = vec![
            Mutation::Create {
                hlc: hlc(1, 0, 1),
                id,
                title: "Buy milk".into(),
                notes: Some("2%".into()),
                quadrant: 1,
                due_date: Some(42),
            },
            Mutation::Update {
                hlc: hlc(2, 0, 1),
                id,
                title: Some("Buy oat milk".into()),
                notes: None,
                quadrant: None,
                due_date: None,
            },
            Mutation::Create {
                hlc: hlc(3, 0, 2),
                id: id2,
                title: "Task 2".into(),
                notes: None,
                quadrant: 3,
                due_date: None,
            },
            Mutation::Complete {
                hlc: hlc(4, 0, 2),
                id: id2,
            },
        ];

        for m in &log {
            store.apply(m.clone()).unwrap();
        }

        (store, log)
    }

    #[test]
    fn snapshot_round_trip_bytes() {
        let (store, log) = make_store();
        let snapshot = Snapshot::from_store(&store, &log);
        let bytes = snapshot.to_bytes().unwrap();
        let parsed = Snapshot::from_bytes(&bytes).unwrap();

        assert_eq!(parsed.protocol_version, SNAPSHOT_VERSION);
        assert_eq!(parsed.tasks, snapshot.tasks);
        assert_eq!(parsed.log, snapshot.log);

        let replayed = parsed.replay().unwrap();
        assert_eq!(replayed.len(), store.len());
        for task in store.values() {
            assert_eq!(replayed.get(task.id), Some(task));
        }
    }

    #[test]
    fn encrypted_snapshot_store_round_trip() {
        let (store, log) = make_store();
        let snapshot = Snapshot::from_store(&store, &log);

        let storage = InMemorySecureStorage::default();
        let root = EpochRoot::generate().unwrap();
        let epoch_key = root.derive(0).unwrap();
        let snapshot_store = SnapshotStore::new(&storage, epoch_key);

        let bytes = snapshot.to_bytes().unwrap();
        snapshot_store.store(&bytes).unwrap();

        let loaded = snapshot_store.load().unwrap();
        let parsed = Snapshot::from_bytes(&loaded).unwrap();
        assert_eq!(parsed.tasks, snapshot.tasks);
        assert_eq!(parsed.log, snapshot.log);
    }

    #[test]
    fn snapshot_manager_save_and_load() {
        let storage = InMemorySecureStorage::default();
        let root = EpochRoot::generate().unwrap();
        let epoch_key = root.derive(0).unwrap();

        let id = TaskId([3; 16]);
        let mut manager = SnapshotManager::new(SnapshotStore::new(&storage, epoch_key));
        manager
            .apply(Mutation::Create {
                hlc: hlc(1, 0, 1),
                id,
                title: "Walk dog".into(),
                notes: None,
                quadrant: 2,
                due_date: None,
            })
            .unwrap();
        manager
            .apply(Mutation::Complete {
                hlc: hlc(2, 0, 1),
                id,
            })
            .unwrap();

        manager.save().unwrap();

        let epoch_key2 = root.derive(0).unwrap();
        let loaded_manager =
            SnapshotManager::load(SnapshotStore::new(&storage, epoch_key2)).unwrap();

        assert_eq!(loaded_manager.store().len(), 1);
        let task = loaded_manager.store().get(id).unwrap();
        assert!(task.is_completed());
        assert_eq!(loaded_manager.log().len(), 2);
    }

    #[test]
    fn snapshot_replay_includes_purge() {
        let mut store = TaskStore::new();
        let id = TaskId([4; 16]);

        let log = vec![
            Mutation::Create {
                hlc: hlc(1, 0, 1),
                id,
                title: "Temporary".into(),
                notes: None,
                quadrant: 0,
                due_date: None,
            },
            Mutation::Delete {
                hlc: hlc(2, 0, 1),
                id,
            },
            Mutation::Purge { id },
        ];

        for m in &log {
            store.apply(m.clone()).unwrap();
        }

        let snapshot = Snapshot::from_store(&store, &log);
        let bytes = snapshot.to_bytes().unwrap();
        let parsed = Snapshot::from_bytes(&bytes).unwrap();
        let replayed = parsed.replay().unwrap();

        assert!(replayed.get(id).is_none());
        assert_eq!(replayed.len(), 0);
    }

    #[test]
    fn unsupported_version_rejected() {
        let (store, log) = make_store();
        let mut snapshot = Snapshot::from_store(&store, &log);
        snapshot.protocol_version = 99;
        let bytes = snapshot.to_bytes().unwrap();

        let err = Snapshot::from_bytes(&bytes).unwrap_err();
        assert!(matches!(err, SnapshotError::UnsupportedVersion(99)));
    }

    #[test]
    fn tampered_ciphertext_fails_decryption() {
        let (store, log) = make_store();
        let snapshot = Snapshot::from_store(&store, &log);

        let storage = InMemorySecureStorage::default();
        let root = EpochRoot::generate().unwrap();
        let epoch_key = root.derive(0).unwrap();
        let snapshot_store = SnapshotStore::new(&storage, epoch_key);

        let bytes = snapshot.to_bytes().unwrap();
        snapshot_store.store(&bytes).unwrap();

        // Tamper with the stored ciphertext.
        let stored = storage.load("snapshot:ciphertext").unwrap().unwrap();
        let mut tampered = stored;
        tampered[20] ^= 0xff;
        storage.store("snapshot:ciphertext", &tampered).unwrap();

        let epoch_key2 = root.derive(0).unwrap();
        let result = SnapshotStore::new(&storage, epoch_key2).load();
        assert!(result.is_err());
    }
}
