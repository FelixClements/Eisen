//! Canonical CBOR parser and validator (P1.03).
//!
//! Decodes a byte slice into a dynamic CBOR value, validates all canonical
//! constraints (definite lengths, shortest integers, sorted unique map keys,
//! NFC text, finite non-integer floats, no unknown tags/simples, recursion/size
//! bounds), then re-encodes canonically and verifies the bytes are byte-identical
//! to the input. The returned value can be used as the input to crypto/storage.

use cbor2::Value;
use unicode_normalization::UnicodeNormalization;

/// Default protocol limits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    /// Maximum total message size in bytes.
    pub max_size: usize,
    /// Maximum nesting depth.
    pub max_depth: usize,
    /// Maximum length of any single text or byte string.
    pub max_string_len: usize,
    /// Maximum number of elements in any single array or map.
    pub max_container_len: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_size: 1_048_576, // 1 MiB
            max_depth: 64,
            max_string_len: 65_535,
            max_container_len: 65_535,
        }
    }
}

/// Errors returned by canonical decode/verify.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CanonError {
    Decode(String),
    Oversized { limit: usize, actual: usize },
    TooDeep { limit: usize, actual: usize },
    StringTooLong { limit: usize, actual: usize },
    ContainerTooLong { limit: usize, actual: usize },
    NonCanonical,
    DuplicateKey,
    UnsortedKeys,
    NonNfcText,
    NonFiniteFloat,
    IntegerFloat,
    IntegerOverflow,
    UnknownTag(u64),
    UnknownSimple,
    TrailingData,
    NotExactlyOneItem,
}

impl std::fmt::Display for CanonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CanonError::Decode(s) => write!(f, "decode error: {s}"),
            CanonError::Oversized { limit, actual } => {
                write!(f, "message too large: {actual} bytes, limit {limit}")
            }
            CanonError::TooDeep { limit, actual } => {
                write!(f, "nesting too deep: {actual}, limit {limit}")
            }
            CanonError::StringTooLong { limit, actual } => {
                write!(f, "string/bytes too long: {actual}, limit {limit}")
            }
            CanonError::ContainerTooLong { limit, actual } => {
                write!(f, "array/map too long: {actual}, limit {limit}")
            }
            CanonError::NonCanonical => write!(f, "input is not in canonical CBOR form"),
            CanonError::DuplicateKey => write!(f, "duplicate map key"),
            CanonError::UnsortedKeys => write!(f, "map keys are not in canonical order"),
            CanonError::NonNfcText => write!(f, "text string is not NFC-normalized"),
            CanonError::NonFiniteFloat => write!(f, "float is NaN or infinite"),
            CanonError::IntegerFloat => write!(f, "float is an exact integer"),
            CanonError::IntegerOverflow => write!(f, "integer does not fit in 64 bits"),
            CanonError::UnknownTag(t) => write!(f, "unknown or forbidden CBOR tag {t}"),
            CanonError::UnknownSimple => write!(f, "unknown or forbidden CBOR simple value"),
            CanonError::TrailingData => write!(f, "trailing data after CBOR item"),
            CanonError::NotExactlyOneItem => {
                write!(f, "input does not contain exactly one CBOR item")
            }
        }
    }
}

impl std::error::Error for CanonError {}

/// Decode and validate canonical CBOR bytes, returning the parsed value.
pub fn parse(bytes: &[u8], limits: &Limits) -> Result<Value, CanonError> {
    if bytes.len() > limits.max_size {
        return Err(CanonError::Oversized {
            limit: limits.max_size,
            actual: bytes.len(),
        });
    }

    cbor2::validate(bytes).map_err(|e| CanonError::Decode(e.to_string()))?;

    let mut value: Value =
        cbor2::from_slice(bytes).map_err(|e| CanonError::Decode(e.to_string()))?;

    // `from_slice` returns the first item; `validate` ensures exactly one item
    // and no trailing data, so we do not need to recheck length here.

    validate(&value, limits, 1)?;

    // Normalize the in-memory value: sort map keys, reject duplicates, strip
    // bignum leading zeros, canonicalize NaN. This is idempotent for canonical
    // inputs and errors for non-canonical structural issues.
    value
        .canonicalize()
        .map_err(|e| CanonError::Decode(e.to_string()))?;

    // Re-encode canonically and compare to the input. This catches non-shortest
    // integer/float encodings and unsorted map keys that survived decoding.
    let canonical =
        cbor2::to_canonical_vec(&value).map_err(|e| CanonError::Decode(e.to_string()))?;
    if canonical != bytes {
        return Err(CanonError::NonCanonical);
    }

    Ok(value)
}

/// Alias for `parse` that returns the canonical byte representation.
pub fn verify(bytes: &[u8], limits: &Limits) -> Result<Vec<u8>, CanonError> {
    parse(bytes, limits)?;
    Ok(bytes.to_vec())
}

/// Encode a value into canonical CBOR bytes.
pub fn encode(value: &Value) -> Result<Vec<u8>, CanonError> {
    cbor2::to_canonical_vec(value).map_err(|e| CanonError::Decode(e.to_string()))
}

fn validate(value: &Value, limits: &Limits, depth: usize) -> Result<(), CanonError> {
    if depth > limits.max_depth {
        return Err(CanonError::TooDeep {
            limit: limits.max_depth,
            actual: depth,
        });
    }

    match value {
        Value::Integer(i) => validate_integer(*i),
        Value::Bytes(b) => {
            if b.len() > limits.max_string_len {
                return Err(CanonError::StringTooLong {
                    limit: limits.max_string_len,
                    actual: b.len(),
                });
            }
            Ok(())
        }
        Value::Float(f) => validate_float(*f),
        Value::Text(s) => {
            if s.len() > limits.max_string_len {
                return Err(CanonError::StringTooLong {
                    limit: limits.max_string_len,
                    actual: s.len(),
                });
            }
            let nfc: String = s.nfc().collect();
            if nfc != *s {
                return Err(CanonError::NonNfcText);
            }
            Ok(())
        }
        Value::Bool(_) | Value::Null => Ok(()),
        Value::Tag(t, _) => Err(CanonError::UnknownTag(*t)),
        Value::Simple(_) => Err(CanonError::UnknownSimple),
        Value::Array(arr) => {
            if arr.len() > limits.max_container_len {
                return Err(CanonError::ContainerTooLong {
                    limit: limits.max_container_len,
                    actual: arr.len(),
                });
            }
            for v in arr {
                validate(v, limits, depth + 1)?;
            }
            Ok(())
        }
        Value::Map(map) => {
            if map.len() > limits.max_container_len {
                return Err(CanonError::ContainerTooLong {
                    limit: limits.max_container_len,
                    actual: map.len(),
                });
            }
            for i in 1..map.len() {
                for j in 0..i {
                    if map[i].0 == map[j].0 {
                        return Err(CanonError::DuplicateKey);
                    }
                }
            }
            for (k, v) in map {
                validate(k, limits, depth + 1)?;
                validate(v, limits, depth + 1)?;
            }
            Ok(())
        }
        _ => Err(CanonError::Decode(
            "unsupported or unknown CBOR value variant".into(),
        )),
    }
}

fn validate_integer(i: cbor2::value::Integer) -> Result<(), CanonError> {
    let v: i128 = i.into();
    if v >= 0 && v <= u64::MAX as i128 {
        return Ok(());
    }
    if v < 0 && v >= i64::MIN as i128 {
        return Ok(());
    }
    Err(CanonError::IntegerOverflow)
}

fn validate_float(f: f64) -> Result<(), CanonError> {
    if !f.is_finite() {
        return Err(CanonError::NonFiniteFloat);
    }
    if f.fract() == 0.0 {
        return Err(CanonError::IntegerFloat);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex_to_bytes(s: &str) -> Vec<u8> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    }

    #[test]
    fn round_trip_simple_map() {
        let value = cbor2::cbor!({
            "b" => 2,
            "a" => 1,
        })
        .unwrap();

        let bytes = encode(&value).unwrap();
        let parsed = parse(&bytes, &Limits::default()).unwrap();

        // Canonical encoding sorts keys.
        assert_eq!(
            parsed,
            cbor2::cbor!({
                "a" => 1,
                "b" => 2,
            })
            .unwrap()
        );
    }

    #[test]
    fn reject_unsorted_map_keys() {
        // {"b": 2, "a": 1} in CBOR: a2 61 62 02 61 61 01
        let bytes = hex_to_bytes("a2616202616101");
        let err = parse(&bytes, &Limits::default()).unwrap_err();
        assert!(matches!(
            err,
            CanonError::NonCanonical | CanonError::UnsortedKeys
        ));
    }

    #[test]
    fn reject_duplicate_keys() {
        // {"a": 1, "a": 2}: a2 61 61 01 61 61 02
        let bytes = hex_to_bytes("a2616101616102");
        let err = parse(&bytes, &Limits::default()).unwrap_err();
        assert!(matches!(
            err,
            CanonError::DuplicateKey | CanonError::NonCanonical
        ));
    }

    #[test]
    fn reject_non_shortest_integer() {
        // 1 encoded as uint16 (0x1901) instead of 0x01
        let bytes = hex_to_bytes("1901");
        let err = parse(&bytes, &Limits::default()).unwrap_err();
        assert!(matches!(
            err,
            CanonError::NonCanonical | CanonError::Decode(_)
        ));
    }

    #[test]
    fn reject_non_nfc_text() {
        // "é" as e + combining acute (NFD) encoded as text string length 3.
        let nfd = "e\u{0301}";
        let mut bytes = vec![0x63];
        bytes.extend_from_slice(nfd.as_bytes());
        let err = parse(&bytes, &Limits::default()).unwrap_err();
        assert!(matches!(err, CanonError::NonNfcText));
    }

    #[test]
    fn accept_nfc_text() {
        let nfc = "\u{00e9}";
        let mut bytes = vec![0x62]; // text string length 2
        bytes.extend_from_slice(nfc.as_bytes());
        let parsed = parse(&bytes, &Limits::default()).unwrap();
        assert_eq!(parsed, Value::Text(nfc.into()));
    }

    #[test]
    fn reject_integer_float() {
        // 1.0 as float64: fb 3ff0 0000 0000 0000
        let bytes = hex_to_bytes("fb3ff0000000000000");
        let err = parse(&bytes, &Limits::default()).unwrap_err();
        assert!(matches!(err, CanonError::IntegerFloat));
    }

    #[test]
    fn reject_nan() {
        // NaN as float64: fb 7ff8 0000 0000 0000
        let bytes = hex_to_bytes("fb7ff8000000000000");
        let err = parse(&bytes, &Limits::default()).unwrap_err();
        assert!(matches!(err, CanonError::NonFiniteFloat));
    }

    #[test]
    fn reject_unknown_tag() {
        // tag 99 with integer 1: d8 63 01
        let bytes = hex_to_bytes("d86301");
        let err = parse(&bytes, &Limits::default()).unwrap_err();
        assert!(matches!(err, CanonError::UnknownTag(99)));
    }

    #[test]
    fn reject_oversized_message() {
        let mut limits = Limits::default();
        limits.max_size = 4;
        let bytes = hex_to_bytes("a201020304");
        let err = parse(&bytes, &limits).unwrap_err();
        assert!(matches!(err, CanonError::Oversized { .. }));
    }

    #[test]
    fn reject_too_deep() {
        let mut limits = Limits::default();
        limits.max_depth = 2;
        // [[1]]: 81 81 01
        let bytes = hex_to_bytes("818101");
        let err = parse(&bytes, &limits).unwrap_err();
        assert!(matches!(err, CanonError::TooDeep { .. }));
    }

    #[test]
    fn accept_1_5_float() {
        // Use cbor2's canonical encoding for 1.5 (it chooses the shortest form).
        let bytes = cbor2::to_canonical_vec(&Value::Float(1.5)).unwrap();
        let parsed = parse(&bytes, &Limits::default()).unwrap();
        assert_eq!(parsed, Value::Float(1.5));
    }

    #[test]
    fn accept_u64_max() {
        let bytes = hex_to_bytes("1bffffffffffffffff");
        let parsed = parse(&bytes, &Limits::default()).unwrap();
        assert_eq!(parsed, Value::Integer(u64::MAX.into()));
    }

    #[test]
    fn reject_bignum_tag() {
        // tag 2 (positive bignum) with bytes 0x01 00 ... 00 (9 bytes = 2^72)
        let mut bytes = vec![0xc2, 0x49]; // tag 2, byte string length 9
        bytes.extend_from_slice(&[0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
        let err = parse(&bytes, &Limits::default()).unwrap_err();
        assert!(matches!(
            err,
            CanonError::UnknownTag(2) | CanonError::IntegerOverflow
        ));
    }
}
