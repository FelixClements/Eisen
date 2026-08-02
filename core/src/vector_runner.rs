//! Cross-platform seed vector runner (P1.18).
//!
//! With EISEN_GENERATE_VECTORS=1 this test creates/overwrites
//! core/tests/vectors.json with seed fixtures. Run normally it reads the file
//! and verifies every vector.

use crate::canonical::{self, Limits};
use crate::envelope::Envelope;
use crate::epoch::EpochRoot;
use crate::identity::{
    create_vault, GenesisManifest, InMemorySecureStorage, OwnerTrust, SecureStorage, VaultId,
};
use crate::manifest::ManifestChain;
use crate::recovery::{ArgonProfile, RecoveryPackage};
use crate::snapshot::{Snapshot, SnapshotError};
use crate::store::LocalStore;
use crate::{DeviceId, Hlc, Mutation, TaskId, TaskStore};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;


#[derive(Serialize, Deserialize, Debug)]
pub struct Fixture {
    id: String,
    kind: String,
    description: String,
    input: Value,
    expected: Value,
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn from_hex(s: &str) -> Result<Vec<u8>, String> {
    if s.len() % 2 != 0 {
        return Err("odd hex length".into());
    }
    (0..s.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&s[i..i + 2], 16)
                .map_err(|e| format!("invalid hex at byte {i}: {e}"))
        })
        .collect()
}

fn hex_to_array<const N: usize>(s: &str) -> Result<[u8; N], String> {
    let bytes = from_hex(s)?;
    let len = bytes.len();
    bytes
        .try_into()
        .map_err(|_| format!("expected {N} bytes, got {len}"))
}

fn device_id_from_hex(s: &str) -> Result<DeviceId, String> {
    Ok(DeviceId(hex_to_array::<16>(s)?))
}

fn task_id_from_hex(s: &str) -> Result<TaskId, String> {
    Ok(TaskId(hex_to_array::<16>(s)?))
}

fn vault_id_from_hex(s: &str) -> Result<VaultId, String> {
    Ok(VaultId(hex_to_array::<16>(s)?))
}

fn hlc_to_value(hlc: &Hlc) -> Value {
    json!({
        "wall": hlc.wall,
        "counter": hlc.counter,
        "device_id_hex": to_hex(&hlc.device_id.0),
    })
}

fn hlc_from_value(v: &Value) -> Result<Hlc, String> {
    let wall = v
        .get("wall")
        .and_then(|x| x.as_u64())
        .ok_or("missing wall")?;
    let counter = v
        .get("counter")
        .and_then(|x| x.as_u64())
        .ok_or("missing counter")? as u32;
    let device_id = device_id_from_hex(
        v.get("device_id_hex")
            .and_then(|x| x.as_str())
            .ok_or("missing device_id_hex")?,
    )?;
    Ok(Hlc {
        wall,
        counter,
        device_id,
    })
}

fn optional_string(v: &Value, key: &str) -> Result<Option<String>, String> {
    match v.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        _ => Err(format!("{key} must be a string or null")),
    }
}

fn optional_u64(v: &Value, key: &str) -> Result<Option<u64>, String> {
    match v.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(x) => x
            .as_u64()
            .ok_or_else(|| format!("{key} must be an unsigned integer"))
            .map(Some),
    }
}

fn optional_optional_string(v: &Value, key: &str) -> Result<Option<Option<String>>, String> {
    match v.get(key) {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::String(s)) => Ok(Some(Some(s.clone()))),
        _ => Err(format!("{key} must be a string or null")),
    }
}

fn optional_optional_u64(v: &Value, key: &str) -> Result<Option<Option<u64>>, String> {
    match v.get(key) {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(x) => x
            .as_u64()
            .ok_or_else(|| format!("{key} must be an unsigned integer"))
            .map(|n| Some(Some(n))),
    }
}

fn optional_u8(v: &Value, key: &str) -> Result<Option<u8>, String> {
    match v.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(x) => x
            .as_u64()
            .and_then(|n| n.try_into().ok())
            .ok_or_else(|| format!("{key} must be an unsigned byte"))
            .map(Some),
    }
}

fn action_to_mutation(v: &Value) -> Result<Mutation, String> {
    let kind = v
        .get("kind")
        .and_then(|x| x.as_str())
        .ok_or("missing kind")?;
    let hlc = hlc_from_value(v.get("hlc").ok_or("missing hlc")?)?;
    let id = task_id_from_hex(
        v.get("id_hex")
            .and_then(|x| x.as_str())
            .ok_or("missing id_hex")?,
    )?;

    match kind {
        "Create" => Ok(Mutation::Create {
            hlc,
            id,
            title: v
                .get("title")
                .and_then(|x| x.as_str())
                .ok_or("missing title")?
                .to_string(),
            notes: optional_string(v, "notes")?,
            quadrant: v
                .get("quadrant")
                .and_then(|x| x.as_u64())
                .ok_or("missing quadrant")? as u8,
            due_date: optional_u64(v, "due_date")?,
        }),
        "Update" => Ok(Mutation::Update {
            hlc,
            id,
            title: optional_string(v, "title")?,
            notes: optional_optional_string(v, "notes")?,
            quadrant: optional_u8(v, "quadrant")?,
            due_date: optional_optional_u64(v, "due_date")?,
        }),
        "Complete" => Ok(Mutation::Complete { hlc, id }),
        "Restore" => Ok(Mutation::Restore { hlc, id }),
        "Delete" => Ok(Mutation::Delete { hlc, id }),
        "Purge" => Ok(Mutation::Purge { id }),
        _ => Err(format!("unknown action kind: {kind}")),
    }
}

fn mutation_to_action(m: &Mutation) -> Value {
    let mut map = Map::new();
    match m {
        Mutation::Create {
            hlc,
            id,
            title,
            notes,
            quadrant,
            due_date,
        } => {
            map.insert("kind".into(), "Create".into());
            map.insert("hlc".into(), hlc_to_value(hlc));
            map.insert("id_hex".into(), to_hex(&id.0).into());
            map.insert("title".into(), title.clone().into());
            if let Some(n) = notes {
                map.insert("notes".into(), n.clone().into());
            }
            map.insert("quadrant".into(), (*quadrant).into());
            if let Some(d) = due_date {
                map.insert("due_date".into(), (*d).into());
            }
        }
        Mutation::Update {
            hlc,
            id,
            title,
            notes,
            quadrant,
            due_date,
        } => {
            map.insert("kind".into(), "Update".into());
            map.insert("hlc".into(), hlc_to_value(hlc));
            map.insert("id_hex".into(), to_hex(&id.0).into());
            if let Some(t) = title {
                map.insert("title".into(), t.clone().into());
            }
            match notes {
                None => {}
                Some(None) => {
                    map.insert("notes".into(), Value::Null);
                }
                Some(Some(n)) => {
                    map.insert("notes".into(), n.clone().into());
                }
            }
            if let Some(q) = quadrant {
                map.insert("quadrant".into(), (*q).into());
            }
            match due_date {
                None => {}
                Some(None) => {
                    map.insert("due_date".into(), Value::Null);
                }
                Some(Some(d)) => {
                    map.insert("due_date".into(), (*d).into());
                }
            }
        }
        Mutation::Complete { hlc, id } => {
            map.insert("kind".into(), "Complete".into());
            map.insert("hlc".into(), hlc_to_value(hlc));
            map.insert("id_hex".into(), to_hex(&id.0).into());
        }
        Mutation::Restore { hlc, id } => {
            map.insert("kind".into(), "Restore".into());
            map.insert("hlc".into(), hlc_to_value(hlc));
            map.insert("id_hex".into(), to_hex(&id.0).into());
        }
        Mutation::Delete { hlc, id } => {
            map.insert("kind".into(), "Delete".into());
            map.insert("hlc".into(), hlc_to_value(hlc));
            map.insert("id_hex".into(), to_hex(&id.0).into());
        }
        Mutation::Purge { id } => {
            map.insert("kind".into(), "Purge".into());
            map.insert("id_hex".into(), to_hex(&id.0).into());
        }
    }
    Value::Object(map)
}

fn actions_to_mutations(actions: &[Value]) -> Result<Vec<Mutation>, String> {
    actions.iter().map(action_to_mutation).collect()
}

fn generate_canonical_round_trip() -> Result<Fixture, String> {
    let mut store = TaskStore::new();
    let id = TaskId([1; 16]);
    let create = Mutation::Create {
        hlc: Hlc {
            wall: 1,
            counter: 0,
            device_id: DeviceId([1; 16]),
        },
        id,
        title: "Vector".into(),
        notes: None,
        quadrant: 0,
        due_date: None,
    };
    store.apply(create).map_err(|e| e.to_string())?;
    let snapshot = Snapshot::from_store(&store, &[]);
    let bytes = snapshot.to_bytes().map_err(|e| e.to_string())?;
    Ok(Fixture {
        id: "canonical-001".into(),
        kind: "canonical".into(),
        description: "Canonical CBOR round-trips through canonical parser".into(),
        input: json!({"bytes_hex": to_hex(&bytes)}),
        expected: json!({"valid": true}),
    })
}

fn generate_canonical_negative() -> Result<Fixture, String> {
    Ok(Fixture {
        id: "canonical-002".into(),
        kind: "canonical".into(),
        description: "Invalid CBOR is rejected by canonical parser".into(),
        input: json!({"bytes_hex": "1f"}),
        expected: json!({"valid": false}),
    })
}

fn generate_hlc_ordering() -> Result<Fixture, String> {
    let a = Hlc {
        wall: 1,
        counter: 0,
        device_id: DeviceId([2; 16]),
    };
    let b = Hlc {
        wall: 2,
        counter: 0,
        device_id: DeviceId([1; 16]),
    };
    Ok(Fixture {
        id: "hlc-001".into(),
        kind: "hlc".into(),
        description: "HLC ordering is wall-first then counter then device_id".into(),
        input: json!({
            "hlc_a": hlc_to_value(&a),
            "hlc_b": hlc_to_value(&b),
        }),
        expected: json!({"b_greater_than_a": true}),
    })
}

fn generate_merge_duplicate() -> Result<Fixture, String> {
    let id = TaskId([2; 16]);
    let create = Mutation::Create {
        hlc: Hlc {
            wall: 1,
            counter: 0,
            device_id: DeviceId([1; 16]),
        },
        id,
        title: "A".into(),
        notes: None,
        quadrant: 0,
        due_date: None,
    };
    let update = Mutation::Update {
        hlc: Hlc {
            wall: 2,
            counter: 0,
            device_id: DeviceId([1; 16]),
        },
        id,
        title: Some("B".into()),
        notes: None,
        quadrant: None,
        due_date: None,
    };
    let actions = vec![
        mutation_to_action(&create),
        mutation_to_action(&update),
        mutation_to_action(&update),
    ];
    Ok(Fixture {
        id: "merge-001".into(),
        kind: "merge".into(),
        description: "Duplicate updates do not change the merged task".into(),
        input: json!({"actions": actions}),
        expected: json!({"title": "B"}),
    })
}

fn generate_merge_drop() -> Result<Fixture, String> {
    let id = TaskId([3; 16]);
    let create = Mutation::Create {
        hlc: Hlc {
            wall: 1,
            counter: 0,
            device_id: DeviceId([1; 16]),
        },
        id,
        title: "A".into(),
        notes: None,
        quadrant: 0,
        due_date: None,
    };
    let update = Mutation::Update {
        hlc: Hlc {
            wall: 2,
            counter: 0,
            device_id: DeviceId([1; 16]),
        },
        id,
        title: Some("B".into()),
        notes: None,
        quadrant: None,
        due_date: None,
    };
    let actions = vec![mutation_to_action(&create), mutation_to_action(&update)];
    Ok(Fixture {
        id: "merge-002".into(),
        kind: "merge".into(),
        description: "Dropping an update changes the merged title".into(),
        input: json!({"actions": actions}),
        expected: json!({
            "with_update_title": "B",
            "without_update_title": "A",
            "changed": true,
        }),
    })
}

fn generate_merge_permutation() -> Result<Fixture, String> {
    let id = TaskId([4; 16]);
    let create = Mutation::Create {
        hlc: Hlc {
            wall: 1,
            counter: 0,
            device_id: DeviceId([1; 16]),
        },
        id,
        title: "A".into(),
        notes: None,
        quadrant: 0,
        due_date: None,
    };
    let update_b = Mutation::Update {
        hlc: Hlc {
            wall: 2,
            counter: 0,
            device_id: DeviceId([1; 16]),
        },
        id,
        title: Some("B".into()),
        notes: None,
        quadrant: None,
        due_date: None,
    };
    let update_c = Mutation::Update {
        hlc: Hlc {
            wall: 3,
            counter: 0,
            device_id: DeviceId([1; 16]),
        },
        id,
        title: Some("C".into()),
        notes: None,
        quadrant: None,
        due_date: None,
    };
    let actions = vec![
        mutation_to_action(&create),
        mutation_to_action(&update_b),
        mutation_to_action(&update_c),
    ];
    Ok(Fixture {
        id: "merge-003".into(),
        kind: "merge".into(),
        description: "Merge converges to the same state regardless of update order".into(),
        input: json!({"actions": actions}),
        expected: json!({"title": "C"}),
    })
}

fn generate_snapshot_round_trip() -> Result<Fixture, String> {
    let mut store = TaskStore::new();
    let id = TaskId([5; 16]);
    let create = Mutation::Create {
        hlc: Hlc {
            wall: 1,
            counter: 0,
            device_id: DeviceId([1; 16]),
        },
        id,
        title: "Snapshot".into(),
        notes: None,
        quadrant: 1,
        due_date: None,
    };
    store.apply(create.clone()).map_err(|e| e.to_string())?;
    let log = vec![create];
    let snapshot = Snapshot::from_store(&store, &log);
    let bytes = snapshot.to_bytes().map_err(|e| e.to_string())?;
    Ok(Fixture {
        id: "snapshot-001".into(),
        kind: "snapshot".into(),
        description: "Snapshot round-trips through bytes and replays correctly".into(),
        input: json!({"bytes_hex": to_hex(&bytes)}),
        expected: json!({"valid": true, "title": "Snapshot"}),
    })
}

fn generate_snapshot_unsupported_version() -> Result<Fixture, String> {
    let snapshot = Snapshot {
        protocol_version: 2,
        tasks: vec![],
        log: vec![],
    };
    let bytes = snapshot.to_bytes().map_err(|e| e.to_string())?;
    Ok(Fixture {
        id: "snapshot-002".into(),
        kind: "snapshot".into(),
        description: "Unsupported snapshot protocol version is rejected".into(),
        input: json!({"bytes_hex": to_hex(&bytes)}),
        expected: json!({"valid": false, "error": "UnsupportedVersion(2)"}),
    })
}

fn generate_envelope_valid() -> Result<Fixture, String> {
    let storage = InMemorySecureStorage::default();
    let (owner_trust, device) = create_vault(
        &storage,
        Hlc {
            wall: 1,
            counter: 0,
            device_id: DeviceId([0; 16]),
        },
    )
    .map_err(|e| e.to_string())?;
    let hlc = Hlc {
        wall: 2,
        counter: 0,
        device_id: device.device_id,
    };
    let create = Mutation::Create {
        hlc,
        id: TaskId([6; 16]),
        title: "Envelope".into(),
        notes: None,
        quadrant: 0,
        due_date: None,
    };
    let envelope = Envelope::sign(&create, hlc, &device.signing_key).map_err(|e| e.to_string())?;
    let envelope_hex = to_hex(&envelope.to_bytes().map_err(|e| e.to_string())?);
    let manifest_hex = to_hex(&owner_trust.genesis_manifest.to_bytes().map_err(|e| e.to_string())?);
    let owner_signing_bytes = owner_trust.owner_signing_key.to_bytes();
    let owner_signing_key_hex = to_hex(owner_signing_bytes.as_ref());
    let vault_id_hex = owner_trust.vault_id.to_hex();
    Ok(Fixture {
        id: "envelope-001".into(),
        kind: "envelope".into(),
        description: "A signed device envelope verifies against owner trust".into(),
        input: json!({
            "envelope_hex": envelope_hex,
            "manifest_hex": manifest_hex,
            "owner_signing_key_hex": owner_signing_key_hex,
            "vault_id_hex": vault_id_hex,
        }),
        expected: json!({"valid": true}),
    })
}

fn generate_envelope_malformed() -> Result<Fixture, String> {
    Ok(Fixture {
        id: "envelope-002".into(),
        kind: "envelope".into(),
        description: "Malformed envelope bytes are rejected".into(),
        input: json!({"bytes_hex": to_hex(&[0u8; 100])}),
        expected: json!({"valid": false}),
    })
}

fn generate_manifest_round_trip() -> Result<Fixture, String> {
    let storage = InMemorySecureStorage::default();
    let hlc = Hlc {
        wall: 1,
        counter: 0,
        device_id: DeviceId([0; 16]),
    };
    let (owner_trust, _) = create_vault(&storage, hlc).map_err(|e| e.to_string())?;
    let bytes = owner_trust
        .genesis_manifest
        .to_bytes()
        .map_err(|e| e.to_string())?;
    Ok(Fixture {
        id: "manifest-001".into(),
        kind: "manifest".into(),
        description: "Genesis manifest round-trips and verifies".into(),
        input: json!({"bytes_hex": to_hex(&bytes)}),
        expected: json!({"valid": true}),
    })
}

fn generate_nonce_monotonic() -> Result<Fixture, String> {
    let device_id = DeviceId([8; 16]);
    let mut actions = Vec::new();
    for wall in [1u64, 2] {
        let id = TaskId([wall as u8; 16]);
        let m = Mutation::Create {
            hlc: Hlc {
                wall,
                counter: 0,
                device_id,
            },
            id,
            title: "Nonce".into(),
            notes: None,
            quadrant: 0,
            due_date: None,
        };
        actions.push(mutation_to_action(&m));
    }
    Ok(Fixture {
        id: "nonce-001".into(),
        kind: "nonce".into(),
        description: "LocalStore nonces are monotonically assigned".into(),
        input: json!({
            "epoch_root_hex": to_hex(&[7u8; 32]),
            "device_id_hex": to_hex(&device_id.0),
            "actions": actions,
        }),
        expected: json!({"last_nonce": 2}),
    })
}

fn generate_recovery_malformed() -> Result<Fixture, String> {
    Ok(Fixture {
        id: "recovery-001".into(),
        kind: "recovery".into(),
        description: "Malformed recovery package bytes are rejected".into(),
        input: json!({"bytes_hex": to_hex(&[0u8; 20])}),
        expected: json!({"valid": false}),
    })
}

fn generate_recovery_valid() -> Result<Fixture, String> {
    let storage = InMemorySecureStorage::default();
    let hlc = Hlc {
        wall: 1,
        counter: 0,
        device_id: DeviceId([0; 16]),
    };
    let (owner_trust, device) = create_vault(&storage, hlc).map_err(|e| e.to_string())?;
    let chain = ManifestChain::new(owner_trust.genesis_manifest.clone()).map_err(|e| e.to_string())?;
    let devices = vec![device.manifest_entry(crate::identity::DeviceStatus::Active)];
    let epoch_roots: BTreeMap<u64, crate::epoch::EpochRoot> = BTreeMap::new();
    let passphrase = "fixture-passphrase";
    let package = RecoveryPackage::create(
        passphrase.as_bytes(),
        &owner_trust,
        &epoch_roots,
        &chain,
        &devices,
        None,
        ArgonProfile::test(),
    )
    .map_err(|e| e.to_string())?;
    Ok(Fixture {
        id: "recovery-002".into(),
        kind: "recovery".into(),
        description: "A valid recovery package parses structurally".into(),
        input: json!({
            "package_hex": to_hex(&package.to_bytes()),
            "passphrase": passphrase,
        }),
        expected: json!({"valid": true}),
    })
}

pub fn generate_fixtures() -> Result<Vec<Fixture>, String> {
    let mut fixtures = Vec::new();
    fixtures.push(generate_canonical_round_trip()?);
    fixtures.push(generate_canonical_negative()?);
    fixtures.push(generate_hlc_ordering()?);
    fixtures.push(generate_merge_duplicate()?);
    fixtures.push(generate_merge_drop()?);
    fixtures.push(generate_merge_permutation()?);
    fixtures.push(generate_snapshot_round_trip()?);
    fixtures.push(generate_snapshot_unsupported_version()?);
    fixtures.push(generate_envelope_valid()?);
    fixtures.push(generate_envelope_malformed()?);
    fixtures.push(generate_manifest_round_trip()?);
    fixtures.push(generate_nonce_monotonic()?);
    fixtures.push(generate_recovery_malformed()?);
    fixtures.push(generate_recovery_valid()?);
    Ok(fixtures)
}

fn merge_actions(actions: &[Value]) -> Result<Option<crate::Task>, String> {
    let mutations = actions_to_mutations(actions)?;
    crate::merge_mutations(&mutations).map_err(|e| e.to_string())
}

fn task_title(task: &Option<crate::Task>) -> Result<String, String> {
    task.as_ref()
        .map(|t| t.title.value.clone())
        .flatten()
        .ok_or_else(|| "merged task has no title".into())
}

fn verify_canonical_round_trip(f: &Fixture) -> Result<(), String> {
    let bytes = from_hex(
        f.input["bytes_hex"]
            .as_str()
            .ok_or("bytes_hex must be a string")?,
    )?;
    canonical::parse(&bytes, &Limits::default()).map_err(|e| e.to_string())?;
    assert_bool(&f.expected["valid"], true, "valid")?;
    Ok(())
}

fn verify_canonical_negative(f: &Fixture) -> Result<(), String> {
    let bytes = from_hex(
        f.input["bytes_hex"]
            .as_str()
            .ok_or("bytes_hex must be a string")?,
    )?;
    if canonical::parse(&bytes, &Limits::default()).is_ok() {
        return Err("invalid CBOR parsed without error".into());
    }
    assert_bool(&f.expected["valid"], false, "valid")?;
    Ok(())
}

fn verify_hlc_ordering(f: &Fixture) -> Result<(), String> {
    let a = hlc_from_value(&f.input["hlc_a"])?;
    let b = hlc_from_value(&f.input["hlc_b"])?;
    assert_bool(&f.expected["b_greater_than_a"], b > a, "b_greater_than_a")?;
    Ok(())
}

fn verify_merge_duplicate(f: &Fixture) -> Result<(), String> {
    let actions = f.input["actions"].as_array().ok_or("actions must be an array")?;
    let title = task_title(&merge_actions(actions)?)?;
    assert_string(&f.expected["title"], &title, "title")?;
    Ok(())
}

fn verify_merge_drop(f: &Fixture) -> Result<(), String> {
    let actions = f.input["actions"].as_array().ok_or("actions must be an array")?;
    let full = actions_to_mutations(actions)?;
    let mut dropped = full.clone();
    dropped.pop();
    let full_title = task_title(&crate::merge_mutations(&full).map_err(|e| e.to_string())?)?;
    let dropped_title =
        task_title(&crate::merge_mutations(&dropped).map_err(|e| e.to_string())?)?;
    assert_string(
        &f.expected["with_update_title"],
        &full_title,
        "with_update_title",
    )?;
    assert_string(
        &f.expected["without_update_title"],
        &dropped_title,
        "without_update_title",
    )?;
    assert_bool(&f.expected["changed"], full_title != dropped_title, "changed")?;
    Ok(())
}

fn verify_merge_permutation(f: &Fixture) -> Result<(), String> {
    let actions = f.input["actions"].as_array().ok_or("actions must be an array")?;
    let id = task_id_from_hex(
        actions[0]["id_hex"]
            .as_str()
            .ok_or("missing id_hex")?,
    )?;
    let original = actions_to_mutations(actions)?;
    let mut swapped = original.clone();
    swapped.swap(1, 2);

    fn build_task(mutations: &[Mutation]) -> Option<crate::Task> {
        let mut store = TaskStore::new();
        for m in mutations {
            store.apply(m.clone()).unwrap();
        }
        let id = mutations.first()?.task_id();
        store.get(id).cloned()
    }

    let task_a = build_task(&original).ok_or("failed to build task_a")?;
    let task_b = build_task(&swapped).ok_or("failed to build task_b")?;

    let mut store = TaskStore::new();
    store.merge_task(&task_a);
    store.merge_task(&task_b);
    let merged = store.get(id).ok_or("merged task not found")?;
    let title = merged
        .title
        .value
        .clone()
        .ok_or("merged task has no title")?;
    assert_string(&f.expected["title"], &title, "title")?;
    Ok(())
}

fn verify_snapshot_round_trip(f: &Fixture) -> Result<(), String> {
    let bytes = from_hex(
        f.input["bytes_hex"]
            .as_str()
            .ok_or("bytes_hex must be a string")?,
    )?;
    let parsed = Snapshot::from_bytes(&bytes).map_err(|e| e.to_string())?;
    let replayed = parsed.replay().map_err(|e| e.to_string())?;
    let id = TaskId([5; 16]);
    let title = replayed
        .get(id)
        .ok_or("task not found")?
        .title
        .value
        .clone()
        .ok_or("title missing")?;
    assert_bool(&f.expected["valid"], true, "valid")?;
    assert_string(&f.expected["title"], &title, "title")?;
    Ok(())
}

fn verify_snapshot_unsupported_version(f: &Fixture) -> Result<(), String> {
    let bytes = from_hex(
        f.input["bytes_hex"]
            .as_str()
            .ok_or("bytes_hex must be a string")?,
    )?;
    match Snapshot::from_bytes(&bytes) {
        Err(SnapshotError::UnsupportedVersion(2)) => {}
        other => return Err(format!("expected UnsupportedVersion(2), got {other:?}")),
    }
    assert_bool(&f.expected["valid"], false, "valid")?;
    Ok(())
}

fn verify_envelope_valid(f: &Fixture) -> Result<(), String> {
    let envelope_bytes = from_hex(
        f.input["envelope_hex"]
            .as_str()
            .ok_or("missing envelope_hex")?,
    )?;
    let manifest_bytes = from_hex(
        f.input["manifest_hex"]
            .as_str()
            .ok_or("missing manifest_hex")?,
    )?;
    let owner_signing_bytes = from_hex(
        f.input["owner_signing_key_hex"]
            .as_str()
            .ok_or("missing owner_signing_key_hex")?,
    )?;
    let vault_id = vault_id_from_hex(
        f.input["vault_id_hex"]
            .as_str()
            .ok_or("missing vault_id_hex")?,
    )?;

    let storage = InMemorySecureStorage::default();
    storage
        .store(
            &format!("vault:{}:owner_signing", vault_id.to_hex()),
            &owner_signing_bytes,
        )
        .map_err(|e| e.to_string())?;
    storage
        .store(&format!("vault:{}:genesis", vault_id.to_hex()), &manifest_bytes)
        .map_err(|e| e.to_string())?;

    let owner_trust = OwnerTrust::load(vault_id, &storage).map_err(|e| e.to_string())?;
    let envelope = Envelope::from_bytes(&envelope_bytes).map_err(|e| e.to_string())?;
    envelope.verify(&owner_trust).map_err(|e| e.to_string())?;
    assert_bool(&f.expected["valid"], true, "valid")?;
    Ok(())
}

fn verify_envelope_malformed(f: &Fixture) -> Result<(), String> {
    let bytes = from_hex(
        f.input["bytes_hex"]
            .as_str()
            .ok_or("bytes_hex must be a string")?,
    )?;
    if Envelope::from_bytes(&bytes).is_ok() {
        return Err("malformed envelope parsed without error".into());
    }
    assert_bool(&f.expected["valid"], false, "valid")?;
    Ok(())
}

fn verify_manifest_round_trip(f: &Fixture) -> Result<(), String> {
    let bytes = from_hex(
        f.input["bytes_hex"]
            .as_str()
            .ok_or("bytes_hex must be a string")?,
    )?;
    let parsed = GenesisManifest::from_bytes(&bytes).map_err(|e| e.to_string())?;
    parsed.verify().map_err(|e| e.to_string())?;
    assert_bool(&f.expected["valid"], true, "valid")?;
    Ok(())
}

fn verify_nonce_monotonic(f: &Fixture) -> Result<(), String> {
    let root_bytes = hex_to_array::<32>(
        f.input["epoch_root_hex"]
            .as_str()
            .ok_or("epoch_root_hex must be a string")?,
    )?;
    let device_id = device_id_from_hex(
        f.input["device_id_hex"]
            .as_str()
            .ok_or("device_id_hex must be a string")?,
    )?;
    let actions = f.input["actions"].as_array().ok_or("actions must be an array")?;

    let root = EpochRoot::from_bytes(root_bytes);
    let epoch_key = root.derive(0).map_err(|e| e.to_string())?;
    let storage = InMemorySecureStorage::default();
    let mut store = LocalStore::open(&storage, epoch_key, device_id).map_err(|e| e.to_string())?;

    for action in actions {
        let m = action_to_mutation(action)?;
        let wall = m.hlc().ok_or("mutation has no hlc")?.wall;
        store.commit(wall, m).map_err(|e| e.to_string())?;
    }

    let expected = f.expected["last_nonce"]
        .as_u64()
        .ok_or("last_nonce must be an unsigned integer")?;
    if store.metadata().last_nonce != expected {
        return Err(format!(
            "last_nonce = {}, expected {expected}",
            store.metadata().last_nonce
        ));
    }
    Ok(())
}

fn verify_recovery_malformed(f: &Fixture) -> Result<(), String> {
    let bytes = from_hex(
        f.input["bytes_hex"]
            .as_str()
            .ok_or("bytes_hex must be a string")?,
    )?;
    if RecoveryPackage::from_bytes(&bytes).is_ok() {
        return Err("malformed recovery package parsed without error".into());
    }
    assert_bool(&f.expected["valid"], false, "valid")?;
    Ok(())
}

fn verify_recovery_valid(f: &Fixture) -> Result<(), String> {
    let bytes = from_hex(
        f.input["package_hex"]
            .as_str()
            .ok_or("package_hex must be a string")?,
    )?;
    RecoveryPackage::from_bytes(&bytes).map_err(|e| e.to_string())?;
    assert_bool(&f.expected["valid"], true, "valid")?;
    Ok(())
}

fn assert_bool(v: &Value, expected: bool, label: &str) -> Result<(), String> {
    let actual = v.as_bool().ok_or_else(|| format!("{label} not a bool"))?;
    if actual != expected {
        return Err(format!("{label}: expected {expected}, got {actual}"));
    }
    Ok(())
}

fn assert_string(v: &Value, expected: &str, label: &str) -> Result<(), String> {
    let actual = v.as_str().ok_or_else(|| format!("{label} not a string"))?;
    if actual != expected {
        return Err(format!("{label}: expected {expected:?}, got {actual:?}"));
    }
    Ok(())
}

pub fn verify_fixture(f: &Fixture) -> Result<(), String> {
    match f.id.as_str() {
        "canonical-001" => verify_canonical_round_trip(f),
        "canonical-002" => verify_canonical_negative(f),
        "hlc-001" => verify_hlc_ordering(f),
        "merge-001" => verify_merge_duplicate(f),
        "merge-002" => verify_merge_drop(f),
        "merge-003" => verify_merge_permutation(f),
        "snapshot-001" => verify_snapshot_round_trip(f),
        "snapshot-002" => verify_snapshot_unsupported_version(f),
        "envelope-001" => verify_envelope_valid(f),
        "envelope-002" => verify_envelope_malformed(f),
        "manifest-001" => verify_manifest_round_trip(f),
        "nonce-001" => verify_nonce_monotonic(f),
        "recovery-001" => verify_recovery_malformed(f),
        "recovery-002" => verify_recovery_valid(f),
        _ => Err(format!("unknown fixture id: {}", f.id)),
    }
}

pub fn run_vectors_path(input_path: &str, output_path: &str) -> Result<(), String> {
    let data = std::fs::read_to_string(input_path).map_err(|e| e.to_string())?;
    let fixtures: Vec<Fixture> = serde_json::from_str(&data).map_err(|e| e.to_string())?;
    let mut results = Vec::new();
    let mut failures = 0;
    for f in &fixtures {
        let error = match verify_fixture(f) {
            Ok(()) => None,
            Err(e) => {
                failures += 1;
                Some(e)
            }
        };
        let status = if error.is_none() { "pass" } else { "fail" };
        results.push(json!({
            "id": f.id,
            "status": status,
            "error": error,
            "output_hex": null
        }));
    }
    let report = json!({
        "total": fixtures.len(),
        "failed": failures,
        "vectors": results,
    });
    let file = std::fs::File::create(output_path).map_err(|e| e.to_string())?;
    serde_json::to_writer_pretty(file, &report).map_err(|e| e.to_string())?;
    if failures == 0 {
        Ok(())
    } else {
        Err(format!("{failures} vector(s) failed"))
    }
}

/// Run all fixtures from an in-memory JSON string. This avoids `std::fs` so
/// it can be used from the browser/WASM test harness.
pub fn run_vectors_str(input: &str) -> Result<(), String> {
    let fixtures: Vec<Fixture> = serde_json::from_str(input).map_err(|e| e.to_string())?;
    let mut failures = Vec::new();
    for f in &fixtures {
        if let Err(e) = verify_fixture(f) {
            failures.push(format!("{}: {}", f.id, e));
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(format!("{} vector(s) failed:\n{}", failures.len(), failures.join("\n")))
    }
}

