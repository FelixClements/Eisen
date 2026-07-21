# Enrollment Handshake

## Roles

- **Owner device**: the existing authorized device that initiates enrollment.
- **New device**: the device being added to the vault.

## Flow

1. **Owner initiation**
   - The owner device generates an expiring, single-use enrollment capability.
   - The capability is a signed token containing:
     - `capability_id` (random 128-bit value)
     - `vault_id`
     - `created_at` HLC
     - `expires_at` HLC
     - `owner_pubkey`
   - The owner signs the token with the owner signing key.

2. **New-device keys**
   - The new device generates an Ed25519 signing key pair and an X25519 encryption key pair.
   - The public keys are kept local until the capability is presented.

3. **Bounded QR payload**
   - The owner may display the capability as a QR code.
   - The QR payload is bounded to 1,000 bytes and contains only the signed capability and a rendezvous relay/cloud URL.
   - The QR payload does not contain the epoch key or any plaintext vault data.

4. **Expiring single-use capability**
   - The capability expires after 5 minutes by default.
   - It can be cancelled by the owner at any time before completion.
   - It is consumed on first successful use and cannot be replayed.

5. **Authenticated key exchange**
   - The new device sends the capability and its new public keys to the owner device via the cloud or relay.
   - The owner verifies the capability signature and checks that it has not expired or been cancelled.
   - The owner and new device perform an X25519 key exchange to establish an ephemeral shared secret.

6. **Confirmation code**
   - Both devices display a short confirmation code derived from the shared secret (e.g., first 6 digits of a SHA-256 hash).
   - The user confirms on both devices that the codes match.
   - This prevents man-in-the-middle attacks during the exchange.

7. **Encrypted epoch-key transfer**
   - The owner encrypts the current epoch root and membership manifest under the shared secret.
   - The encrypted payload is sent to the new device.
   - The new device decrypts the epoch root and derives its device-specific signing and encryption keys.

8. **Membership publication**
   - The owner signs a new membership manifest that includes the new device public keys.
   - The manifest is published to all existing devices via the cloud or relay.
   - All devices update their local manifest chain before accepting operations from the new device.

9. **Cancellation**
   - The owner can cancel the enrollment at any time before membership publication.
   - A cancelled capability is recorded to prevent replay.

10. **Disappearance handling**
    - If the new device disappears before completion, the capability expires and is discarded.
    - The owner does not add the device to the membership manifest.
    - If the new device reappears after expiry, the owner must restart the handshake.

## Security properties

- The new device cannot impersonate the owner.
- The owner cannot decrypt the new device's private keys.
- The cloud/relay cannot decrypt the epoch key or learn device private keys.
- The capability is single-use and time-bound, limiting exposure if intercepted.
