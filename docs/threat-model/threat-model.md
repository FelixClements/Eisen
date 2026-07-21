# Threat Model Review

## Scope

This document reviews the threats against Eisen and the mitigations in the protocol design.

## Trust boundaries

- **Device**: user-owned phone or computer. The device stores keys in platform secure storage.
- **Cloud service**: operated by the project. It stores opaque envelopes and snapshots, authenticates accounts, and routes data. It does not hold decryption keys.
- **Relay service**: volatile peer-routing service. It does not persist operation payloads.
- **Other devices**: peer devices owned by the same user.
- **Attacker**: any party that can monitor, modify, or disrupt network traffic, or compromise a device or service.

## Threats and mitigations

| Threat | Mitigation |
|---|---|
| Cloud service reads task content | End-to-end AES-GCM encryption; service sees only opaque envelopes and routing metadata. |
| Cloud service forges an operation | Device signatures on every operation; service tokens do not authorize vault operations. |
| Relay service persists payloads | Relay is volatile by design; payload persistence audit required. |
| Man-in-the-middle during enrollment | Confirmation code + authenticated key exchange; encrypted epoch-key transfer. |
| Stolen device | Revocation and epoch rotation; old device keys rejected after cutover. |
| Lost passphrase | No server reset; recovery only via recovery package or another enrolled device. |
| Offline old-epoch operations | Old-epoch keys retained for a bounded window; after cutover, new operations require the new epoch, but pre-cutover history remains valid. |
| Rollback / clone attack | Fail-closed nonce/counter policy; clone detected by duplicate device ID or nonce reuse; rollback triggers repair. |
| Malformed input | Bounded canonical encoding; fail-closed verification order; size limits before parsing. |
| Secrets in logs / backups | Local-at-rest encryption; no plaintext in crash reports; recovery package encrypted. |
| Metadata analysis | Cloud sees only vault ID, operation IDs, digests, and sizes; no plaintext titles or task counts. |

## Service access vs cryptographic authorization

- Service authentication (account token) only proves that the request comes from a registered account.
- Vault-operation authorization is via device Ed25519 signatures on the operation or request.
- A compromised service token cannot create a valid operation because the service does not possess device private keys.

## Offline old-epoch handling

- Devices that are offline during an epoch rotation receive the new epoch key when they reconnect.
- Until they receive it, they cannot author new operations.
- They can continue to read old-epoch data they already have.
- Old-epoch keys are retained for 90 days; after that, old-epoch snapshots and backups can no longer be decrypted.

## Residual risks

- Cryptographic implementation bugs in the core.
- Compromise of platform secure storage on a device.
- Social engineering of the user's passphrase.
- Denial-of-service against cloud or relay.

## Review plan

- Independent crypto/protocol review before cloud-sync release (G5).
- Annual threat-model update or after any protocol change.
- Operational penetration testing of cloud and relay services.
