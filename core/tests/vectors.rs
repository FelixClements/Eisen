//! Cross-platform seed vector runner (P1.18).
//!
//! Defines deterministic and functional seed vectors for canonical, HLC, merge,
//! snapshot, compatibility, envelope, manifest, nonce, and recovery categories.
//! Run with: cargo test --test vectors

use eisen_core::canonical::{self, Limits};
use eisen_core::epoch::EpochRoot;
use eisen_core::envelope::Envelope;
use eisen_core::identity::{create_vault, InMemorySecureStorage};
use eisen_core::recovery::RecoveryPackage;
use eisen_core::snapshot::{Snapshot, SnapshotError};
use eisen_core::store::LocalStore;
use eisen_core::{DeviceId, Hlc, Mutation, TaskId, TaskStore};

#[derive(Debug)]
enum Vector {
    CanonicalRoundTrip,
    CanonicalNegative,
    HlcOrdering,
    MergeDuplicate,
    MergeDrop,
    MergePermutation,
    SnapshotRoundTrip,
    SnapshotUnsupportedVersion,
    EnvelopeValid,
    EnvelopeMalformed,
    ManifestRoundTrip,
    NonceMonotonic,
    RecoveryMalformed,
}

impl Vector {
    fn id(&self) -> &'static str {
        match self {
            Vector::CanonicalRoundTrip => "canonical-001",
            Vector::CanonicalNegative => "canonical-002",
            Vector::HlcOrdering => "hlc-001",
            Vector::MergeDuplicate => "merge-001",
            Vector::MergeDrop => "merge-002",
            Vector::MergePermutation => "merge-003",
            Vector::SnapshotRoundTrip => "snapshot-001",
            Vector::SnapshotUnsupportedVersion => "snapshot-002",
            Vector::EnvelopeValid => "envelope-001",
            Vector::EnvelopeMalformed => "envelope-002",
            Vector::ManifestRoundTrip => "manifest-001",
            Vector::NonceMonotonic => "nonce-001",
            Vector::RecoveryMalformed => "recovery-001",
        }
    }

    fn run(&self) -> Result<(), String> {
        match self {
            Vector::CanonicalRoundTrip => {
                let mut store = TaskStore::new();
                let id = TaskId([1; 16]);
                store
                    .apply(Mutation::Create {
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
                    })
                    .map_err(|e| e.to_string())?;
                let snapshot = Snapshot::from_store(&store, &[]);
                let bytes = snapshot.to_bytes().map_err(|e| e.to_string())?;
                let _ = Snapshot::from_bytes(&bytes).map_err(|e| e.to_string())?;
                Ok(())
            }
            Vector::CanonicalNegative => {
                let invalid = [0x1f]; // incomplete CBOR
                if canonical::parse(&invalid, &Limits::default()).is_ok() {
                    return Err("invalid CBOR parsed without error".into());
                }
                Ok(())
            }
            Vector::HlcOrdering => {
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
                if b > a {
                    Ok(())
                } else {
                    Err("HLC ordering failed".into())
                }
            }
            Vector::MergeDuplicate => {
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
                let once = eisen_core::merge_mutations(&[create.clone(), update.clone()])
                    .map_err(|e| e.to_string())?;
                let twice =
                    eisen_core::merge_mutations(&[create.clone(), update.clone(), update.clone()])
                        .map_err(|e| e.to_string())?;
                if once != twice {
                    return Err("duplicate update changed merge result".into());
                }
                if once.as_ref().unwrap().title.value != Some("B".into()) {
                    return Err("wrong title after duplicate update".into());
                }
                Ok(())
            }
            Vector::MergeDrop => {
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
                let with_update =
                    eisen_core::merge_mutations(&[create.clone(), update.clone()]).unwrap();
                let without_update = eisen_core::merge_mutations(&[create.clone()]).unwrap();
                if with_update == without_update {
                    return Err("dropping update did not change state".into());
                }
                Ok(())
            }
            Vector::MergePermutation => {
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

                fn build_task(mutations: &[Mutation]) -> Option<eisen_core::Task> {
                    let mut store = TaskStore::new();
                    for m in mutations {
                        store.apply(m.clone()).unwrap();
                    }
                    let id = mutations.first().unwrap().task_id();
                    store.get(id).cloned()
                }

                let perm_a = [create.clone(), update_b.clone(), update_c.clone()];
                let perm_b = [create.clone(), update_c.clone(), update_b.clone()];
                let task_a = build_task(&perm_a).unwrap();
                let task_b = build_task(&perm_b).unwrap();

                let mut store = TaskStore::new();
                store.merge_task(&task_a);
                store.merge_task(&task_b);
                let merged = store.get(id).unwrap();
                if merged.title.value != Some("C".into()) {
                    return Err("permutation merge did not converge to C".into());
                }
                Ok(())
            }
            Vector::SnapshotRoundTrip => {
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
                let log: Vec<Mutation> = vec![create];
                let snapshot = Snapshot::from_store(&store, &log);
                let bytes = snapshot.to_bytes().map_err(|e| e.to_string())?;
                let parsed = Snapshot::from_bytes(&bytes).map_err(|e| e.to_string())?;
                let replayed = parsed.replay().map_err(|e| e.to_string())?;
                if replayed.get(id).unwrap().title.value != Some("Snapshot".into()) {
                    return Err("snapshot replay did not restore task".into());
                }
                Ok(())
            }
            Vector::SnapshotUnsupportedVersion => {
                let snapshot = Snapshot {
                    protocol_version: 2,
                    tasks: vec![],
                    log: vec![],
                };
                let bytes = snapshot.to_bytes().map_err(|e| e.to_string())?;
                match Snapshot::from_bytes(&bytes) {
                    Err(SnapshotError::UnsupportedVersion(2)) => Ok(()),
                    other => Err(format!("expected UnsupportedVersion(2), got {other:?}")),
                }
            }
            Vector::EnvelopeValid => {
                let storage = InMemorySecureStorage::default();
                let (owner_trust, device) = create_vault(
                    &storage,
                    Hlc {
                        wall: 1,
                        counter: 0,
                        device_id: DeviceId([0; 16]),
                    },
                )
                .map_err(|e| format!("create_vault: {e}"))?;
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
                let envelope =
                    Envelope::sign(&create, hlc, &device.signing_key).map_err(|e| e.to_string())?;
                let _ = envelope.verify(&owner_trust).map_err(|e| e.to_string())?;
                Ok(())
            }
            Vector::EnvelopeMalformed => {
                let bytes = [0u8; 100];
                if Envelope::from_bytes(&bytes).is_ok() {
                    return Err("malformed envelope parsed without error".into());
                }
                Ok(())
            }
            Vector::ManifestRoundTrip => {
                let storage = InMemorySecureStorage::default();
                let hlc = Hlc {
                    wall: 1,
                    counter: 0,
                    device_id: DeviceId([0; 16]),
                };
                let (owner_trust, _) = create_vault(&storage, hlc)
                    .map_err(|e| format!("create_vault: {e}"))?;
                let bytes = owner_trust
                    .genesis_manifest
                    .to_bytes()
                    .map_err(|e| e.to_string())?;
                let parsed = eisen_core::identity::GenesisManifest::from_bytes(&bytes)
                    .map_err(|e| e.to_string())?;
                parsed.verify().map_err(|e| e.to_string())?;
                Ok(())
            }
            Vector::NonceMonotonic => {
                let storage = InMemorySecureStorage::default();
                let root = EpochRoot::from_bytes([7u8; 32]);
                let key = root.derive(0).map_err(|e| e.to_string())?;
                let device_id = DeviceId([8; 16]);
                let mut store = LocalStore::open(&storage, key, device_id)
                    .map_err(|e| format!("open: {e}"))?;
                for wall in [1u64, 2] {
                    let id = TaskId([wall as u8; 16]);
                    store
                        .commit(
                            wall,
                            Mutation::Create {
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
                            },
                        )
                        .map_err(|e| format!("commit: {e}"))?;
                }
                if store.metadata().last_nonce != 2 {
                    return Err(format!("last_nonce = {}, expected 2", store.metadata().last_nonce));
                }
                Ok(())
            }
            Vector::RecoveryMalformed => {
                let bytes = [0u8; 20];
                if RecoveryPackage::from_bytes(&bytes).is_ok() {
                    return Err("malformed recovery package parsed without error".into());
                }
                Ok(())
            }
        }
    }
}

fn all_vectors() -> Vec<Vector> {
    vec![
        Vector::CanonicalRoundTrip,
        Vector::CanonicalNegative,
        Vector::HlcOrdering,
        Vector::MergeDuplicate,
        Vector::MergeDrop,
        Vector::MergePermutation,
        Vector::SnapshotRoundTrip,
        Vector::SnapshotUnsupportedVersion,
        Vector::EnvelopeValid,
        Vector::EnvelopeMalformed,
        Vector::ManifestRoundTrip,
        Vector::NonceMonotonic,
        Vector::RecoveryMalformed,
    ]
}

#[test]
fn run_vectors() {
    for vector in all_vectors() {
        vector.run().unwrap_or_else(|e| panic!("{}: {}", vector.id(), e));
    }
}
