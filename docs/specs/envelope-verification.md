# Envelope Verification Algorithm

This document defines the fail-closed order for verifying an incoming envelope before applying it to local state.

## Input

- `envelope_bytes`: the raw envelope bytes received from storage, transport, or a peer.
- `trusted_manifests`: the ordered list of owner-signed manifests from genesis to current epoch.
- `local_nonce_state`: the set of nonces already seen from the claimed origin device.
- `local_op_log`: the existing immutable operation log.

## Algorithm

1. **Limits**
   - Reject if `envelope_bytes` length exceeds the maximum envelope size (default 1 MiB).
   - Reject if parsing allocates more than the bounded limit for nested structures.

2. **Structure**
   - Parse the envelope as Canonical CBOR per `docs/specs/canonical-encoding.md`.
   - Reject if the top-level value is not a map.
   - Reject if any required field is missing or any field is duplicated.
   - Reject if the map keys are not in canonical sorted order.

3. **Version**
   - Read the integer `ver` field.
   - Reject if the major version is greater than the supported major version.
   - Reject if the minor version is unsupported and the envelope contains unknown required fields.

4. **Canonical re-encoding**
   - Re-encode the parsed header map according to the canonical rules.
   - Reject if the re-encoded bytes are not byte-identical to the input header bytes.

5. **Routing**
   - Read `vault_id`, `origin_device_id`, and `destination_device_id` (if present).
   - Reject if `vault_id` does not match the local vault.
   - Reject if `destination_device_id` is present and does not match this device's ID.

6. **Identity**
   - Read the `device_pubkey` field.
   - Reject if the public key is not a valid Ed25519 public key.
   - Verify the claimed `origin_device_id` matches a stable derivation from `device_pubkey`.
   - Reject if the origin device is not a current member in the manifest chain.

7. **Manifest authorization**
   - Find the applicable owner-signed manifest for the envelope's `epoch`.
   - Reject if no manifest exists for the epoch.
   - Reject if the manifest does not authorize the origin device for the operation type.

8. **Signature**
   - Reconstruct the canonical signed header bytes: `canonical_header`.
   - Verify `signature` over `canonical_header` using `device_pubkey`.
   - Reject on verification failure.

9. **Nonce / key domain**
   - Read `nonce` and `key_domain`.
   - Reject if the nonce has been seen for this origin device and key domain.
   - Reject if the nonce is outside the expected sequence range (e.g., too far in the future).
   - Reject if `key_domain` is unknown for the epoch.

10. **AEAD**
    - Derive the AEAD key for `(epoch, origin_device_id, key_domain)` from the epoch root.
    - Decrypt `ciphertext` using `nonce` as the IV and the canonical header as AAD.
    - Reject on authentication failure.

11. **Plaintext schema**
    - Parse the decrypted plaintext as Canonical CBOR.
    - Reject if it violates the operation schema for the claimed mutation kind.
    - Reject if any operation ID, HLC, or digest fields are malformed or out of bounds.

12. **Merge transaction**
    - Compute the canonical digest of the plaintext.
    - Check for an existing operation ID or digest in `local_op_log`.
    - If the same `op_id` with a different digest is present, quarantine the envelope and reject.
    - If the `op_id` is new and the epoch/key domain is authorized, prepare a local transaction:
      - reserve the nonce,
      - advance or merge HLC,
      - insert into the immutable log,
      - update the materialized view.
    - Commit the transaction durably before sending an acknowledgement.

## Retry versus quarantine

- **Retry failure:** A transient failure (storage unavailable, counter uncertain, manifest not yet received) should cause a retry after a backoff. The envelope is not quarantined.
- **Quarantine failure:** A permanent failure (signature invalid, nonce reused, digest conflict, schema invalid, unauthorized origin) causes the envelope to be moved to a quarantine log with a stable error code. Quarantined envelopes are not reprocessed automatically.
