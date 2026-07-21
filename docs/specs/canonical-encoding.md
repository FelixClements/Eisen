# Canonical Encoding Specification

## Scope

This document defines the deterministic, bounded encoding used for protocol messages, envelope plaintext, snapshots, and any other data that must produce a stable digest across platforms.

## Format

Encoding is **Canonical CBOR** as defined in RFC 8949, Section 4.2, with the additional rules below.

## Types

### Integers

- Signed and unsigned integers are encoded as CBOR major types 0 and 1.
- The shortest representation must be used. Values that fit in one byte (`0..23` and `-1..-24`) use the one-byte form. Values that fit in uint8/uint16/uint32 use the two- or three-byte forms. uint64 is allowed only when the value does not fit in a smaller form.
- Negative integers are represented as `1 + (-value - 1)` per CBOR.
- Big-number tags (2/3) are forbidden; all integers must fit in a 64-bit signed or unsigned value.

### Floating-point numbers

- Only IEEE 754 double-precision (64-bit) and single-precision (32-bit) are permitted.
- Values that are exact integers must be encoded as integers, not floats.
- For non-integer values, the shortest form that preserves the exact value must be used.
- NaN, Infinity, and -Infinity are forbidden.

### Strings and byte strings

- Text strings (major type 3) are UTF-8. They must be normalized to NFC before encoding.
- Implementations must reject invalid UTF-8 and non-NFC text strings on decode.
- Byte strings (major type 2) are opaque byte sequences. No interpretation is applied.
- Both text and byte string lengths are bounded. The maximum size for any single string or byte string is 65,535 bytes unless a protocol-level profile grants a higher bound.

### Arrays

- Arrays (major type 4) are ordered. Encoders must preserve the order given by the schema.
- Decoders must reject arrays whose length exceeds the schema-defined maximum.

### Maps / dictionaries

- Maps (major type 5) contain key-value pairs.
- **Duplicate keys are rejected on decode.**
- **Keys must be sorted** by the raw bytes of their canonical CBOR encoding, using lexicographic byte order.
- Keys must be of the same type within a map as defined by the schema (text-string keys are preferred).
- Map ordering for display is implementation-defined; only the canonical byte order is normative.

### Booleans and null

- `true`, `false`, and `null` use CBOR major types 7/20, 7/21, and 7/22.

### Tags

- Only the tags explicitly allowed by a protocol profile are permitted. Unknown tags are rejected.
- The default profile forbids tags unless listed in a protocol version's tag registry.

## Schema rules

### Field ordering

- All composite types (maps and arrays) must list fields in schema order when an array representation is used.
- When a map representation is used, the canonical byte-key order applies at encoding and decoding time.

### Required and optional fields

- Required fields must be present exactly once.
- Optional fields, when absent, are omitted entirely; `null` may not be used as a placeholder for a missing optional field unless the schema explicitly allows it.
- Unknown fields are rejected on decode.

### Version field

- Every top-level protocol object contains a required integer `ver` field (map key `"ver"`) with a major and minor component encoded as `major << 16 | minor`.
- A decoder must reject a message whose major version is higher than the decoder's supported major version. A higher minor version within the same major version is accepted and ignored fields must be handled according to the unknown-field rule.

## Limits

| Limit | Value |
|---|---|
| Maximum nesting depth | 64 |
| Maximum string / byte-string length | 65,535 bytes (default) |
| Maximum array / map element count | 65,535 (default) |
| Maximum total message size | 1 MiB (default) |

Profiles may raise these limits but must do so explicitly and negotiate the higher bounds before accepting larger messages.

## Digest input

- To compute a digest of a message, the message is first re-encoded according to this specification. The digest is taken over the resulting canonical bytes only.
- Implementations must verify that a message round-trips through encode/decode/encode and produces byte-identical output before computing or verifying a digest.
- The digest input is the canonical byte string; no extra prefix or length field is added unless the protocol explicitly prepends a domain separator.

## Determinism requirements

- Encoders must not produce equivalent but non-identical serializations. Any valid value has exactly one canonical byte sequence.
- Decoders must be fail-closed: any violation of the rules above (invalid UTF-8, non-NFC text, duplicate map keys, unsorted keys, oversized fields, unknown tags, non-shortest integer/float encoding, NaN/Infinity) results in an error.

## References

- RFC 8949: Concise Binary Object Representation (CBOR), Section 4.2 "Canonical CBOR"
- Unicode Standard Annex #15, Unicode Normalization Forms
