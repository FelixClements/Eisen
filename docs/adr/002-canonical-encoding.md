# ADR-002: Canonical Encoding

## Status

Accepted / P0.04

## Context

The protocol requires a deterministic encoding so that digests, signatures, and merge comparisons are stable across Android, Windows, and any future platform. The encoding must be bounded, reject malformed input, and support version negotiation.

## Decision

Use **Canonical CBOR (RFC 8949 Section 4.2)** with the profile defined in `docs/specs/canonical-encoding.md`.

Key points of the profile:

- Shortest integer and floating-point encodings.
- UTF-8 text strings normalized to NFC; rejection of invalid UTF-8 and non-NFC input.
- Byte strings are opaque and bounded.
- Map keys are sorted by canonical CBOR byte encoding and duplicate keys are rejected.
- Top-level `ver` integer field encodes `major << 16 | minor`.
- Digest input is the canonical re-encoding of the message.
- Default bounds: 64 nesting depth, 65,535 element/string length, 1 MiB total message size.

## Consequences

- CBOR parsers and generators are available for Rust, Kotlin, and C# with some adaptation for the profile rules.
- Implementations must enforce canonicality on decode, not only on encode, to prevent canonicalization attacks.
- The bounded limits and fail-closed decoder behavior support the threat-model requirement to validate size and structure before expensive crypto or storage operations.

## Evidence

- Specification: `docs/specs/canonical-encoding.md`
- Reference implementation and test vectors will be added in P1.03 and P1.18.
