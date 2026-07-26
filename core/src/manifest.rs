//! Owner-signed manifest-chain verification (P1.08).
//!
//! Verifies a contiguous chain of owner-signed membership manifests from genesis
//! to the current epoch, supports historical manifest lookup by key epoch, and
//! rejects forks or unauthorized changes.

use crate::identity::{DeviceStatus, GenesisManifest, IdentityError, ManifestContent};
use ed25519_dalek::Signer;

/// A contiguous, owner-signed chain of membership manifests.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ManifestChain {
    manifests: Vec<GenesisManifest>,
}

impl ManifestChain {
    /// Create a chain from a verified genesis manifest.
    pub fn new(genesis: GenesisManifest) -> Result<Self, IdentityError> {
        let mut chain = Self {
            manifests: Vec::new(),
        };
        chain.push(genesis)?;
        Ok(chain)
    }

    /// Verify every link in the chain.
    pub fn verify(&self) -> Result<(), IdentityError> {
        if self.manifests.is_empty() {
            return Err(IdentityError::Chain("chain is empty".into()));
        }

        let genesis = &self.manifests[0];
        verify_genesis(genesis)?;
        self.verify_management_device_active(genesis)?;

        for i in 1..self.manifests.len() {
            let prev = &self.manifests[i - 1];
            let curr = &self.manifests[i];
            curr.verify()?;
            self.verify_transition(prev, curr)?;
        }

        Ok(())
    }

    /// Append a manifest to the chain after validating the link.
    pub fn push(&mut self, manifest: GenesisManifest) -> Result<(), IdentityError> {
        if let Some(prev) = self.manifests.last() {
            manifest.verify()?;
            self.verify_transition(prev, &manifest)?;
        } else {
            verify_genesis(&manifest)?;
            self.verify_management_device_active(&manifest)?;
        }
        self.manifests.push(manifest);
        Ok(())
    }

    /// Return the manifest that applies to the given key epoch: the one with the
    /// greatest `key_epoch` that is not greater than `epoch`.
    pub fn manifest_at_epoch(&self, epoch: u64) -> Option<&GenesisManifest> {
        self.manifests
            .iter()
            .filter(|m| m.content.key_epoch <= epoch)
            .max_by_key(|m| (m.content.key_epoch, m.content.membership_version))
    }

    /// Return the most recent manifest in the chain.
    pub fn current(&self) -> &GenesisManifest {
        self.manifests
            .last()
            .expect("chain is never empty after construction")
    }

    /// Number of manifests in the chain.
    pub fn len(&self) -> usize {
        self.manifests.len()
    }

    pub fn is_empty(&self) -> bool {
        self.manifests.is_empty()
    }

    /// Iterate over the chain from genesis to current.
    pub fn iter(&self) -> impl Iterator<Item = &GenesisManifest> {
        self.manifests.iter()
    }

    fn verify_transition(
        &self,
        prev: &GenesisManifest,
        curr: &GenesisManifest,
    ) -> Result<(), IdentityError> {
        let prev_content = &prev.content;
        let curr_content = &curr.content;

        if curr_content.membership_version != prev_content.membership_version + 1 {
            return Err(IdentityError::Chain(
                "membership version is not contiguous".into(),
            ));
        }
        if curr_content.key_epoch < prev_content.key_epoch {
            return Err(IdentityError::Chain("key epoch decreased".into()));
        }
        if curr_content.owner_pubkey != prev_content.owner_pubkey {
            return Err(IdentityError::Chain("owner pubkey changed".into()));
        }
        if curr_content.vault_id != prev_content.vault_id {
            return Err(IdentityError::Chain("vault id changed".into()));
        }
        if curr_content.created_at <= prev_content.created_at {
            return Err(IdentityError::Chain(
                "created_at is not monotonically increasing".into(),
            ));
        }

        self.verify_management_device_active(curr)
    }

    fn verify_management_device_active(
        &self,
        manifest: &GenesisManifest,
    ) -> Result<(), IdentityError> {
        let id = manifest.content.owner_management_device_id;
        if manifest
            .content
            .devices
            .iter()
            .any(|d| d.device_id == id && d.status == DeviceStatus::Active)
        {
            Ok(())
        } else {
            Err(IdentityError::Chain(
                "owner management device is not active".into(),
            ))
        }
    }
}

fn verify_genesis(manifest: &GenesisManifest) -> Result<(), IdentityError> {
    manifest.verify()?;
    if manifest.content.membership_version != 0 {
        return Err(IdentityError::Chain(
            "genesis membership version must be 0".into(),
        ));
    }
    if manifest.content.key_epoch != 0 {
        return Err(IdentityError::Chain("genesis key epoch must be 0".into()));
    }
    Ok(())
}

/// Build a new manifest signed by the current owner.
pub fn sign_manifest(
    content: ManifestContent,
    owner_signing_key: &ed25519_dalek::SigningKey,
) -> Result<GenesisManifest, IdentityError> {
    let content_bytes = crate::canonical::encode(&manifest_content_to_value(&content)?)
        .map_err(|e| IdentityError::Encode(e.to_string()))?;
    let signature = owner_signing_key.sign(&content_bytes);
    Ok(GenesisManifest {
        content,
        signature: crate::identity::SignatureBytes(signature.to_bytes()),
    })
}

/// Convert manifest content to a canonical CBOR value.
///
/// This mirrors the canonical serialization used by `GenesisManifest` signing so
/// that tests can sign arbitrary manifest contents.
fn manifest_content_to_value(content: &ManifestContent) -> Result<cbor2::Value, IdentityError> {
    cbor2::to_canonical_vec(content)
        .map_err(|e| IdentityError::Encode(e.to_string()))
        .and_then(|bytes| {
            crate::canonical::parse(&bytes, &crate::canonical::Limits::default())
                .map_err(|e| IdentityError::Encode(e.to_string()))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::{create_vault, DeviceIdentity, InMemorySecureStorage, SignatureBytes};
    use crate::{DeviceId, Hlc};
    use ed25519_dalek::Signer;

    fn dev(n: u8) -> DeviceId {
        let mut bytes = [0u8; 16];
        bytes[15] = n;
        DeviceId(bytes)
    }

    fn make_hlc(wall: u64, counter: u32, device: u8) -> Hlc {
        Hlc {
            wall,
            counter,
            device_id: dev(device),
        }
    }

    fn sign_with(content: ManifestContent, key: &ed25519_dalek::SigningKey) -> GenesisManifest {
        let content_bytes = crate::canonical::encode(
            &crate::canonical::parse(
                &cbor2::to_canonical_vec(&content).unwrap(),
                &crate::canonical::Limits::default(),
            )
            .unwrap(),
        )
        .unwrap();
        let signature = key.sign(&content_bytes);
        GenesisManifest {
            content,
            signature: SignatureBytes(signature.to_bytes()),
        }
    }

    #[test]
    fn genesis_chain_verifies() {
        let storage = InMemorySecureStorage::default();
        let hlc = make_hlc(1, 0, 1);
        let (owner_trust, _device) = create_vault(&storage, hlc).unwrap();

        let chain = ManifestChain::new(owner_trust.genesis_manifest.clone()).unwrap();
        assert_eq!(chain.len(), 1);
        assert_eq!(chain.current().content.membership_version, 0);
    }

    #[test]
    fn valid_two_manifest_chain() {
        let storage = InMemorySecureStorage::default();
        let hlc = make_hlc(1, 0, 1);
        let (owner_trust, device) = create_vault(&storage, hlc).unwrap();

        let new_device = DeviceIdentity::generate().unwrap();
        let mut chain = ManifestChain::new(owner_trust.genesis_manifest.clone()).unwrap();

        let content = ManifestContent {
            protocol_version: 0,
            membership_version: 1,
            key_epoch: 1,
            vault_id: owner_trust.vault_id,
            owner_pubkey: owner_trust.owner_pubkey(),
            owner_management_device_id: device.device_id,
            created_at: make_hlc(2, 0, 1),
            devices: vec![
                device.manifest_entry(DeviceStatus::Active),
                new_device.manifest_entry(DeviceStatus::Active),
            ],
        };
        let manifest = sign_with(content, &owner_trust.owner_signing_key);

        chain.push(manifest).unwrap();
        assert_eq!(chain.len(), 2);
        assert_eq!(chain.current().content.membership_version, 1);

        assert_eq!(
            chain
                .manifest_at_epoch(1)
                .unwrap()
                .content
                .membership_version,
            1
        );
        assert_eq!(
            chain
                .manifest_at_epoch(0)
                .unwrap()
                .content
                .membership_version,
            0
        );
    }

    #[test]
    fn membership_version_gap_rejected() {
        let storage = InMemorySecureStorage::default();
        let hlc = make_hlc(1, 0, 1);
        let (owner_trust, device) = create_vault(&storage, hlc).unwrap();

        let mut chain = ManifestChain::new(owner_trust.genesis_manifest.clone()).unwrap();
        let content = ManifestContent {
            protocol_version: 0,
            membership_version: 2,
            key_epoch: 1,
            vault_id: owner_trust.vault_id,
            owner_pubkey: owner_trust.owner_pubkey(),
            owner_management_device_id: device.device_id,
            created_at: make_hlc(2, 0, 1),
            devices: vec![device.manifest_entry(DeviceStatus::Active)],
        };
        let manifest = sign_with(content, &owner_trust.owner_signing_key);
        assert!(chain.push(manifest).is_err());
    }

    #[test]
    fn key_epoch_decrease_rejected() {
        let storage = InMemorySecureStorage::default();
        let hlc = make_hlc(1, 0, 1);
        let (owner_trust, device) = create_vault(&storage, hlc).unwrap();

        let mut chain = ManifestChain::new(owner_trust.genesis_manifest.clone()).unwrap();

        // First push a manifest at key_epoch 1.
        let content1 = ManifestContent {
            protocol_version: 0,
            membership_version: 1,
            key_epoch: 1,
            vault_id: owner_trust.vault_id,
            owner_pubkey: owner_trust.owner_pubkey(),
            owner_management_device_id: device.device_id,
            created_at: make_hlc(2, 0, 1),
            devices: vec![device.manifest_entry(DeviceStatus::Active)],
        };
        let manifest1 = sign_with(content1, &owner_trust.owner_signing_key);
        chain.push(manifest1).unwrap();

        // Then try to push a manifest with a decreased key_epoch.
        let content2 = ManifestContent {
            protocol_version: 0,
            membership_version: 2,
            key_epoch: 0,
            vault_id: owner_trust.vault_id,
            owner_pubkey: owner_trust.owner_pubkey(),
            owner_management_device_id: device.device_id,
            created_at: make_hlc(3, 0, 1),
            devices: vec![device.manifest_entry(DeviceStatus::Active)],
        };
        let manifest2 = sign_with(content2, &owner_trust.owner_signing_key);
        assert!(chain.push(manifest2).is_err());
    }

    #[test]
    fn owner_pubkey_change_rejected() {
        let storage = InMemorySecureStorage::default();
        let hlc = make_hlc(1, 0, 1);
        let (owner_trust, device) = create_vault(&storage, hlc).unwrap();

        let mut chain = ManifestChain::new(owner_trust.genesis_manifest.clone()).unwrap();
        let other_key = ed25519_dalek::SigningKey::from_bytes(&[0xff; 32]);
        let content = ManifestContent {
            protocol_version: 0,
            membership_version: 1,
            key_epoch: 1,
            vault_id: owner_trust.vault_id,
            owner_pubkey: crate::identity::SignPubKey(other_key.verifying_key().to_bytes()),
            owner_management_device_id: device.device_id,
            created_at: make_hlc(2, 0, 1),
            devices: vec![device.manifest_entry(DeviceStatus::Active)],
        };
        // Sign with original owner key: signature will be valid over content,
        // but the chain will reject the owner pubkey change.
        let manifest = sign_with(content, &owner_trust.owner_signing_key);
        assert!(chain.push(manifest).is_err());
    }

    #[test]
    fn vault_id_change_rejected() {
        let storage = InMemorySecureStorage::default();
        let hlc = make_hlc(1, 0, 1);
        let (owner_trust, device) = create_vault(&storage, hlc).unwrap();

        let mut chain = ManifestChain::new(owner_trust.genesis_manifest.clone()).unwrap();
        let other_vault = crate::identity::VaultId::generate().unwrap();
        let content = ManifestContent {
            protocol_version: 0,
            membership_version: 1,
            key_epoch: 1,
            vault_id: other_vault,
            owner_pubkey: owner_trust.owner_pubkey(),
            owner_management_device_id: device.device_id,
            created_at: make_hlc(2, 0, 1),
            devices: vec![device.manifest_entry(DeviceStatus::Active)],
        };
        let manifest = sign_with(content, &owner_trust.owner_signing_key);
        assert!(chain.push(manifest).is_err());
    }

    #[test]
    fn non_monotonic_created_at_rejected() {
        let storage = InMemorySecureStorage::default();
        let hlc = make_hlc(1, 0, 1);
        let (owner_trust, device) = create_vault(&storage, hlc).unwrap();

        let mut chain = ManifestChain::new(owner_trust.genesis_manifest.clone()).unwrap();
        let content = ManifestContent {
            protocol_version: 0,
            membership_version: 1,
            key_epoch: 1,
            vault_id: owner_trust.vault_id,
            owner_pubkey: owner_trust.owner_pubkey(),
            owner_management_device_id: device.device_id,
            created_at: make_hlc(1, 0, 1),
            devices: vec![device.manifest_entry(DeviceStatus::Active)],
        };
        let manifest = sign_with(content, &owner_trust.owner_signing_key);
        assert!(chain.push(manifest).is_err());
    }

    #[test]
    fn inactive_management_device_rejected() {
        let storage = InMemorySecureStorage::default();
        let hlc = make_hlc(1, 0, 1);
        let (owner_trust, device) = create_vault(&storage, hlc).unwrap();

        let mut chain = ManifestChain::new(owner_trust.genesis_manifest.clone()).unwrap();
        let content = ManifestContent {
            protocol_version: 0,
            membership_version: 1,
            key_epoch: 1,
            vault_id: owner_trust.vault_id,
            owner_pubkey: owner_trust.owner_pubkey(),
            owner_management_device_id: device.device_id,
            created_at: make_hlc(2, 0, 1),
            devices: vec![device.manifest_entry(DeviceStatus::Revoked)],
        };
        let manifest = sign_with(content, &owner_trust.owner_signing_key);
        assert!(chain.push(manifest).is_err());
    }

    #[test]
    fn tampered_manifest_signature_rejected() {
        let storage = InMemorySecureStorage::default();
        let hlc = make_hlc(1, 0, 1);
        let (owner_trust, device) = create_vault(&storage, hlc).unwrap();

        let mut chain = ManifestChain::new(owner_trust.genesis_manifest.clone()).unwrap();
        let content = ManifestContent {
            protocol_version: 0,
            membership_version: 1,
            key_epoch: 1,
            vault_id: owner_trust.vault_id,
            owner_pubkey: owner_trust.owner_pubkey(),
            owner_management_device_id: device.device_id,
            created_at: make_hlc(2, 0, 1),
            devices: vec![device.manifest_entry(DeviceStatus::Active)],
        };
        let mut manifest = sign_with(content, &owner_trust.owner_signing_key);
        manifest.signature.0[0] ^= 0xff;
        assert!(chain.push(manifest).is_err());
    }
}
