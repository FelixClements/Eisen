//! Device-signed mutation envelopes (P1.06).
//!
//! A mutation envelope binds an HLC, a device ID, and a mutation body with an
//! Ed25519 signature from that device's signing key. The body is canonical CBOR,
//! and verification checks the signature, the manifest trust anchor, and the
//! device's active membership.

use crate::canonical;
use crate::identity::{DeviceEntry, DeviceStatus, OwnerTrust, SignatureBytes};
use crate::{DeviceId, Hlc, Mutation, TaskId};
use cbor2::Value;
use ed25519_dalek::{Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};

/// Errors from envelope construction or verification.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EnvelopeError {
    Encode(String),
    Decode(String),
    Crypto(String),
    InvalidBody,
    InvalidMutation(String),
    DeviceNotActive,
    DeviceNotInManifest,
    BadSignature,
    HlcMismatch,
}

impl std::fmt::Display for EnvelopeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EnvelopeError::Encode(s) => write!(f, "encode error: {s}"),
            EnvelopeError::Decode(s) => write!(f, "decode error: {s}"),
            EnvelopeError::Crypto(s) => write!(f, "crypto error: {s}"),
            EnvelopeError::InvalidBody => write!(f, "envelope body is malformed"),
            EnvelopeError::InvalidMutation(s) => write!(f, "invalid mutation: {s}"),
            EnvelopeError::DeviceNotActive => write!(f, "device is not active in manifest"),
            EnvelopeError::DeviceNotInManifest => write!(f, "device is not in manifest"),
            EnvelopeError::BadSignature => write!(f, "signature verification failed"),
            EnvelopeError::HlcMismatch => {
                write!(f, "envelope device_id does not match hlc.device_id")
            }
        }
    }
}

impl std::error::Error for EnvelopeError {}

impl From<crate::canonical::CanonError> for EnvelopeError {
    fn from(e: crate::canonical::CanonError) -> Self {
        EnvelopeError::Encode(e.to_string())
    }
}

/// A signed mutation envelope.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Envelope {
    pub device_id: DeviceId,
    pub hlc: Hlc,
    pub mutation: Value,
    pub signature: SignatureBytes,
}

impl Envelope {
    /// Create a signed envelope for a mutation.
    ///
    /// `hlc` must have `hlc.device_id` equal to the signing device.
    pub fn sign(
        mutation: &Mutation,
        hlc: Hlc,
        signing_key: &SigningKey,
    ) -> Result<Self, EnvelopeError> {
        if matches!(mutation, Mutation::Purge { .. }) {
            return Err(EnvelopeError::InvalidMutation(
                "purge mutations are local-only and cannot be enclosed".into(),
            ));
        }

        let device_id = hlc.device_id;
        let mut mutation_value = mutation_to_value(mutation)?;
        mutation_value
            .canonicalize()
            .map_err(|e| EnvelopeError::Encode(e.to_string()))?;
        let body = body_value(device_id, hlc, mutation_value.clone())?;
        let body_bytes = canonical::encode(&body)?;
        let signature = signing_key.sign(&body_bytes);

        Ok(Self {
            device_id,
            hlc,
            mutation: mutation_value,
            signature: SignatureBytes(signature.to_bytes()),
        })
    }

    /// Verify the envelope against an owner trust anchor and return the
    /// contained mutation.
    pub fn verify(&self, owner_trust: &OwnerTrust) -> Result<Mutation, EnvelopeError> {
        if self.hlc.device_id != self.device_id {
            return Err(EnvelopeError::HlcMismatch);
        }

        let device = find_active_device(self.device_id, owner_trust)?;

        let body = body_value(self.device_id, self.hlc, self.mutation.clone())?;
        let body_bytes = canonical::encode(&body)?;

        let pubkey = VerifyingKey::from_bytes(&device.signing_pubkey.0)
            .map_err(|e| EnvelopeError::Crypto(e.to_string()))?;
        let sig = self
            .signature
            .to_ed()
            .map_err(|e| EnvelopeError::Crypto(e.to_string()))?;
        pubkey
            .verify(&body_bytes, &sig)
            .map_err(|_| EnvelopeError::BadSignature)?;

        value_to_mutation(&self.mutation)
    }

    /// Encode the envelope to canonical CBOR bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>, EnvelopeError> {
        cbor2::to_canonical_vec(self).map_err(|e| EnvelopeError::Encode(e.to_string()))
    }

    /// Decode an envelope from canonical CBOR bytes, validating the canonical
    /// form first.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, EnvelopeError> {
        canonical::parse(bytes, &canonical::Limits::default())?;
        cbor2::from_slice(bytes).map_err(|e| EnvelopeError::Decode(e.to_string()))
    }
}

fn find_active_device(
    device_id: DeviceId,
    owner_trust: &OwnerTrust,
) -> Result<DeviceEntry, EnvelopeError> {
    owner_trust
        .genesis_manifest
        .content
        .devices
        .iter()
        .find(|d| d.device_id == device_id)
        .cloned()
        .ok_or(EnvelopeError::DeviceNotInManifest)
        .and_then(|d| match d.status {
            DeviceStatus::Active => Ok(d),
            DeviceStatus::Revoked => Err(EnvelopeError::DeviceNotActive),
        })
}

fn body_value(device_id: DeviceId, hlc: Hlc, mutation: Value) -> Result<Value, EnvelopeError> {
    Ok(Value::Map(vec![
        (text("device_id"), bytes(&device_id.0)),
        (text("hlc"), hlc_to_value(&hlc)),
        (text("mutation"), mutation),
    ]))
}

fn text(s: &str) -> Value {
    Value::Text(s.into())
}

fn bytes(b: &[u8]) -> Value {
    Value::Bytes(b.to_vec())
}

fn hlc_to_value(hlc: &Hlc) -> Value {
    Value::Array(vec![
        Value::Integer((hlc.wall).into()),
        Value::Integer((hlc.counter).into()),
        bytes(&hlc.device_id.0),
    ])
}

fn value_to_hlc(v: &Value) -> Result<Hlc, EnvelopeError> {
    match v {
        Value::Array(arr) if arr.len() == 3 => {
            let wall = to_u64(&arr[0])?;
            let counter = to_u32(&arr[1])?;
            let device_id = value_to_device_id(&arr[2])?;
            Ok(Hlc {
                wall,
                counter,
                device_id,
            })
        }
        _ => Err(EnvelopeError::InvalidBody),
    }
}

fn value_to_device_id(v: &Value) -> Result<DeviceId, EnvelopeError> {
    match v {
        Value::Bytes(b) if b.len() == 16 => {
            let arr: [u8; 16] = b.clone().try_into().unwrap();
            Ok(DeviceId(arr))
        }
        _ => Err(EnvelopeError::InvalidBody),
    }
}

fn value_to_task_id(v: &Value) -> Result<TaskId, EnvelopeError> {
    match v {
        Value::Bytes(b) if b.len() == 16 => {
            let arr: [u8; 16] = b.clone().try_into().unwrap();
            Ok(TaskId(arr))
        }
        _ => Err(EnvelopeError::InvalidBody),
    }
}

fn to_u64(v: &Value) -> Result<u64, EnvelopeError> {
    match v {
        Value::Integer(i) => {
            let n: i128 = (*i).into();
            if n < 0 || n > u64::MAX as i128 {
                return Err(EnvelopeError::InvalidBody);
            }
            Ok(n as u64)
        }
        _ => Err(EnvelopeError::InvalidBody),
    }
}

fn to_u32(v: &Value) -> Result<u32, EnvelopeError> {
    match v {
        Value::Integer(i) => {
            let n: i128 = (*i).into();
            if n < 0 || n > u32::MAX as i128 {
                return Err(EnvelopeError::InvalidBody);
            }
            Ok(n as u32)
        }
        _ => Err(EnvelopeError::InvalidBody),
    }
}

fn to_u8(v: &Value) -> Result<u8, EnvelopeError> {
    match v {
        Value::Integer(i) => {
            let n: i128 = (*i).into();
            if n < 0 || n > u8::MAX as i128 {
                return Err(EnvelopeError::InvalidBody);
            }
            Ok(n as u8)
        }
        _ => Err(EnvelopeError::InvalidBody),
    }
}

fn to_text(v: &Value) -> Result<String, EnvelopeError> {
    match v {
        Value::Text(s) => Ok(s.clone()),
        _ => Err(EnvelopeError::InvalidBody),
    }
}

fn optional_text(v: &Value) -> Result<Option<String>, EnvelopeError> {
    match v {
        Value::Text(s) => Ok(Some(s.clone())),
        Value::Null => Ok(None),
        _ => Err(EnvelopeError::InvalidBody),
    }
}

fn optional_u64(v: &Value) -> Result<Option<u64>, EnvelopeError> {
    match v {
        Value::Integer(i) => Ok(Some(to_u64(&Value::Integer(*i))?)),
        Value::Null => Ok(None),
        _ => Err(EnvelopeError::InvalidBody),
    }
}

fn optional_optional_u64(v: &Value) -> Result<Option<Option<u64>>, EnvelopeError> {
    match v {
        Value::Null => Ok(Some(None)),
        Value::Integer(i) => Ok(Some(Some(to_u64(&Value::Integer(*i))?))),
        _ => Err(EnvelopeError::InvalidBody),
    }
}

fn optional_optional_text(v: &Value) -> Result<Option<Option<String>>, EnvelopeError> {
    match v {
        Value::Null => Ok(Some(None)),
        Value::Text(s) => Ok(Some(Some(s.clone()))),
        _ => Err(EnvelopeError::InvalidBody),
    }
}

fn get_field(map: &[(Value, Value)], key: &str) -> Result<Value, EnvelopeError> {
    map.iter()
        .find(|(k, _)| matches!(k, Value::Text(s) if s == key))
        .map(|(_, v)| v.clone())
        .ok_or(EnvelopeError::InvalidBody)
}

fn maybe_field(map: &[(Value, Value)], key: &str) -> Option<Value> {
    map.iter()
        .find(|(k, _)| matches!(k, Value::Text(s) if s == key))
        .map(|(_, v)| v.clone())
}

fn mutation_to_value(mutation: &Mutation) -> Result<Value, EnvelopeError> {
    match mutation {
        Mutation::Create {
            hlc,
            id,
            title,
            notes,
            quadrant,
            due_date,
        } => {
            let mut map = vec![
                (text("kind"), text("create")),
                (text("hlc"), hlc_to_value(hlc)),
                (text("id"), bytes(&id.0)),
                (text("title"), text(title)),
                (text("quadrant"), Value::Integer((*quadrant).into())),
            ];
            if let Some(n) = notes {
                map.push((text("notes"), text(n)));
            }
            if let Some(d) = due_date {
                map.push((text("due_date"), Value::Integer((*d).into())));
            }
            Ok(Value::Map(map))
        }
        Mutation::Update {
            hlc,
            id,
            title,
            notes,
            quadrant,
            due_date,
        } => {
            let mut map = vec![
                (text("kind"), text("update")),
                (text("hlc"), hlc_to_value(hlc)),
                (text("id"), bytes(&id.0)),
            ];
            if let Some(t) = title {
                map.push((text("title"), text(t)));
            }
            match notes {
                None => {}
                Some(None) => {
                    map.push((text("notes"), Value::Null));
                }
                Some(Some(n)) => {
                    map.push((text("notes"), text(n)));
                }
            }
            if let Some(q) = quadrant {
                map.push((text("quadrant"), Value::Integer((*q).into())));
            }
            match due_date {
                None => {}
                Some(None) => {
                    map.push((text("due_date"), Value::Null));
                }
                Some(Some(d)) => {
                    map.push((text("due_date"), Value::Integer((*d).into())));
                }
            }
            Ok(Value::Map(map))
        }
        Mutation::Complete { hlc, id } => Ok(Value::Map(vec![
            (text("kind"), text("complete")),
            (text("hlc"), hlc_to_value(hlc)),
            (text("id"), bytes(&id.0)),
        ])),
        Mutation::Restore { hlc, id } => Ok(Value::Map(vec![
            (text("kind"), text("restore")),
            (text("hlc"), hlc_to_value(hlc)),
            (text("id"), bytes(&id.0)),
        ])),
        Mutation::Delete { hlc, id } => Ok(Value::Map(vec![
            (text("kind"), text("delete")),
            (text("hlc"), hlc_to_value(hlc)),
            (text("id"), bytes(&id.0)),
        ])),
        Mutation::Purge { .. } => Err(EnvelopeError::InvalidMutation(
            "purge cannot be enclosed".into(),
        )),
    }
}

fn value_to_mutation(v: &Value) -> Result<Mutation, EnvelopeError> {
    match v {
        Value::Map(map) => {
            let kind = match get_field(map, "kind")? {
                Value::Text(s) => s,
                _ => return Err(EnvelopeError::InvalidBody),
            };
            let hlc = value_to_hlc(&get_field(map, "hlc")?)?;
            let id = value_to_task_id(&get_field(map, "id")?)?;

            match kind.as_str() {
                "create" => {
                    let title = to_text(&get_field(map, "title")?)?;
                    let notes = maybe_field(map, "notes")
                        .map(|v| optional_text(&v))
                        .transpose()?
                        .flatten();
                    let due_date = maybe_field(map, "due_date")
                        .map(|v| optional_u64(&v))
                        .transpose()?
                        .flatten();
                    let quadrant = to_u8(&get_field(map, "quadrant")?)?;
                    Ok(Mutation::Create {
                        hlc,
                        id,
                        title,
                        notes,
                        quadrant,
                        due_date,
                    })
                }
                "update" => {
                    let title = maybe_field(map, "title")
                        .map(|v| optional_text(&v))
                        .transpose()?
                        .flatten();
                    let notes = maybe_field(map, "notes")
                        .map(|v| optional_optional_text(&v))
                        .transpose()?
                        .flatten();
                    let quadrant = maybe_field(map, "quadrant")
                        .map(|v| to_u8(&v))
                        .transpose()?;
                    let due_date = maybe_field(map, "due_date")
                        .map(|v| optional_optional_u64(&v))
                        .transpose()?
                        .flatten();
                    Ok(Mutation::Update {
                        hlc,
                        id,
                        title,
                        notes,
                        quadrant,
                        due_date,
                    })
                }
                "complete" => Ok(Mutation::Complete { hlc, id }),
                "restore" => Ok(Mutation::Restore { hlc, id }),
                "delete" => Ok(Mutation::Delete { hlc, id }),
                _ => Err(EnvelopeError::InvalidBody),
            }
        }
        _ => Err(EnvelopeError::InvalidBody),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::{create_vault, InMemorySecureStorage};
    use crate::{DeviceId, Hlc, TaskId};

    fn make_hlc(device_id: DeviceId, wall: u64, counter: u32) -> Hlc {
        Hlc {
            wall,
            counter,
            device_id,
        }
    }

    #[test]
    fn sign_and_verify_create_mutation() {
        let storage = InMemorySecureStorage::default();
        let (owner_trust, device) =
            create_vault(&storage, make_hlc(DeviceId([1; 16]), 1, 0)).unwrap();
        let hlc = make_hlc(device.device_id, 1, 0);

        let mutation = Mutation::Create {
            hlc,
            id: TaskId([10; 16]),
            title: "Buy milk".into(),
            notes: Some("2%".into()),
            quadrant: 1,
            due_date: Some(42),
        };

        let envelope = Envelope::sign(&mutation, hlc, &device.signing_key).unwrap();
        let verified = envelope.verify(&owner_trust).unwrap();
        assert_eq!(verified, mutation);
    }

    #[test]
    fn sign_and_verify_update_mutation() {
        let storage = InMemorySecureStorage::default();
        let (owner_trust, device) =
            create_vault(&storage, make_hlc(DeviceId([2; 16]), 2, 0)).unwrap();
        let hlc = make_hlc(device.device_id, 2, 0);

        let mutation = Mutation::Update {
            hlc,
            id: TaskId([20; 16]),
            title: None,
            notes: Some(None),
            quadrant: None,
            due_date: Some(Some(99)),
        };

        let envelope = Envelope::sign(&mutation, hlc, &device.signing_key).unwrap();
        let verified = envelope.verify(&owner_trust).unwrap();
        assert_eq!(verified, mutation);
    }

    #[test]
    fn tampered_envelope_fails_verification() {
        let storage = InMemorySecureStorage::default();
        let (owner_trust, device) =
            create_vault(&storage, make_hlc(DeviceId([3; 16]), 3, 0)).unwrap();
        let hlc = make_hlc(device.device_id, 3, 0);

        let mutation = Mutation::Complete {
            hlc,
            id: TaskId([30; 16]),
        };
        let mut envelope = Envelope::sign(&mutation, hlc, &device.signing_key).unwrap();
        envelope.hlc.counter = 99;
        assert!(envelope.verify(&owner_trust).is_err());
    }

    #[test]
    fn unknown_device_fails_verification() {
        let storage = InMemorySecureStorage::default();
        let (owner_trust, device) =
            create_vault(&storage, make_hlc(DeviceId([4; 16]), 4, 0)).unwrap();
        let hlc = make_hlc(device.device_id, 4, 0);

        let other_hlc = make_hlc(DeviceId([0xff; 16]), 4, 0);
        let mutation = Mutation::Delete {
            hlc: other_hlc,
            id: TaskId([40; 16]),
        };
        let envelope = Envelope::sign(&mutation, other_hlc, &device.signing_key).unwrap();
        assert!(envelope.verify(&owner_trust).is_err());
    }

    #[test]
    fn envelope_round_trips_bytes() {
        let storage = InMemorySecureStorage::default();
        let (owner_trust, device) =
            create_vault(&storage, make_hlc(DeviceId([5; 16]), 5, 0)).unwrap();
        let hlc = make_hlc(device.device_id, 5, 0);

        let mutation = Mutation::Restore {
            hlc,
            id: TaskId([50; 16]),
        };
        let envelope = Envelope::sign(&mutation, hlc, &device.signing_key).unwrap();
        let bytes = envelope.to_bytes().unwrap();
        let parsed = Envelope::from_bytes(&bytes).unwrap();
        assert_eq!(parsed, envelope);
        assert_eq!(parsed.verify(&owner_trust).unwrap(), mutation);
    }

    #[test]
    fn purge_is_rejected() {
        let storage = InMemorySecureStorage::default();
        let (_, device) = create_vault(&storage, make_hlc(DeviceId([6; 16]), 6, 0)).unwrap();
        let hlc = make_hlc(device.device_id, 6, 0);

        let mutation = Mutation::Purge {
            id: TaskId([60; 16]),
        };
        assert!(Envelope::sign(&mutation, hlc, &device.signing_key).is_err());
    }
}
