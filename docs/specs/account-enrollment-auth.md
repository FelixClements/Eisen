# Account and Enrollment Authentication Design

## Account authentication

- Accounts are identified by a user-chosen account identifier or email.
- Authentication uses a short-lived bearer token obtained via a passwordless or OAuth flow (exact method is outside the protocol and must be configured by ops).
- Account tokens authenticate to the cloud service only. They do not authorize vault operations.
- Account tokens expire after 15 minutes and are refreshed with a refresh token.

## Device enrollment authentication

- Enrollment is always initiated by an existing owner device.
- The owner creates an expiring, single-use capability signed with the owner key.
- The new device proves possession of newly generated device signing and encryption keys.
- An X25519 key exchange establishes an ephemeral shared secret.
- A confirmation code is displayed on both devices and confirmed by the user.
- The epoch key is encrypted under the shared secret and transferred to the new device.
- The owner publishes a membership manifest adding the new device.

## Separation of service access and cryptographic authorization

- The cloud service authenticates the account to allow transport.
- The cloud service authorizes operations only by verifying device Ed25519 signatures.
- A compromised cloud account does not grant access to vault contents or the ability to forge operations.
- A compromised device key does not grant access to other devices' keys or the account itself.

## Revocation

- The owner signs a revocation manifest to remove a device.
- Revoked devices are rejected by all other devices after the manifest is received.
- Revocation does not delete data; it prevents new operations from the revoked device.
