# Versioned Envelope and Nonce Contract

## Envelope structure

The envelope is a Canonical CBOR map with the following fields, in canonical key order:

| Field | Key | Type | Description |
|---|---|---|---|
| Version | `ver` | uint | `major << 16 \| minor`. Current: `65536` (1.0). |
| Vault ID | `vault_id` | byte[32] | Stable vault identifier. |
| Epoch | `epoch` | uint | Epoch number of the operation. |
| Origin device | `origin_device_id` | byte[16] | Device that created the operation. |
| Destination device | `dst_device_id` | byte[16] (optional) | If present, only the named device may decrypt. |
| Public key | `device_pubkey` | byte[32] | Ed25519 public key of the origin device. |
| Key domain | `key_domain` | text | `ops`, `snap`, `recovery`, or a profile-defined domain. |
| Nonce | `nonce` | byte[12] | 96-bit AES-GCM nonce. |
| Ciphertext | `ct` | byte[] | Encrypted plaintext. |
| Signature | `sig` | byte[64] | Ed25519 signature over the canonical header. |

## Canonical header / AAD / signature inputs

- The **canonical header** is the canonical CBOR encoding of all envelope fields except `sig`.
- The **AAD** for AES-GCM is the canonical header bytes.
- The **signature input** is `SHA-256(canonical_header)`.
- All three use the same canonical bytes; any change to the header invalidates both the tag and the signature.

## Nonce layout

The 96-bit nonce is structured as:

```
| epoch_counter (48 bits) | origin_counter (48 bits) |
```

- `epoch_counter` is a monotonic counter scoped to `(vault_id, epoch, key_domain, origin_device_id)`.
- `origin_counter` is a device-local monotonic counter.
- Both counters start at `1`.

## Counter width and exhaustion

- Each counter is 48 bits, allowing approximately 281 trillion operations per scope.
- Counters are persisted and reserved transactionally.
- On counter exhaustion, the device must rotate to a new epoch or key domain; it must not wrap or reuse a nonce.

## Initial value

- Initial nonce value: `0x000000000001000000000001` (both counters at 1).
- A nonce of all zeros is reserved and must never be used.

## Key-domain binding

- The AEAD key is derived from `HKDF-SHA-256(epoch_root, "envelope-key", vault_id \| epoch \| key_domain \| origin_device_id)`.
- The signature key is `HKDF-SHA-256(epoch_root, "device-sign", vault_id \| epoch \| origin_device_id)`.
- `key_domain` is bound into the AAD and key derivation, preventing cross-domain key reuse.

## Required rejection cases

- Nonce length is not 12 bytes.
- Nonce has been seen before for the same `(epoch, key_domain, origin_device_id)`.
- `epoch` is greater than the current epoch or has no valid manifest.
- `origin_device_id` is not authorized in the epoch manifest.
- `key_domain` is unknown for the epoch or operation type.
- `ver` major version mismatch.
- Canonical header cannot be re-encoded to byte-identical form.
- Signature verification fails.
- AEAD tag verification fails.

## Compatible error codes

- `envelope_oversized`
- `invalid_envelope_structure`
- `duplicate_envelope_field`
- `unsupported_version`
- `vault_mismatch`
- `unknown_epoch`
- `unauthorized_device`
- `unknown_key_domain`
- `nonce_reused`
- `nonce_exhausted`
- `signature_invalid`
- `aead_decryption_failed`
- `plaintext_schema_invalid`
