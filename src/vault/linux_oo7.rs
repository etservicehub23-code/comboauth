#![cfg(target_os = "linux")]
#![allow(dead_code)]

use tokio::runtime::Runtime;

use crate::persistence::{
    ComboProfileDto, PersistenceError, PersistenceStore, ServiceRegistryDto,
};
use crate::profile::{ComboProfile, ComboProfileId};
use crate::service::{ServiceId, ServiceRegistry};
use super::{SecretMaterial, SecretStore, SecretStoreError};

const APP_ATTR: &str = "comboauth";
/// Attribute distinguishing credentials from profile/registry items.
const KIND_ATTR: &str = "kind";
const KIND_CRED: &str = "credential";
const KIND_PROFILE: &str = "combo_profile";
const KIND_REGISTRY: &str = "service_registry";
const REGISTRY_ITEM_ID: &str = "default";

// ── Credential store ──────────────────────────────────────────────────────────

/// Linux Secret Service backend via oo7.
///
/// Owns a single-threaded Tokio runtime so SecretStore's sync trait can bridge
/// to oo7's async API. Safe to call from the Ratatui event loop — operations
/// block only during the Secret Service round-trip, which is local IPC.
pub struct OsSecretStore {
    rt: Runtime,
    keyring: oo7::Keyring,
}

impl OsSecretStore {
    pub fn new() -> Result<Self, SecretStoreError> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| SecretStoreError::Backend(format!("tokio runtime: {e}")))?;
        let keyring = rt
            .block_on(oo7::Keyring::new())
            .map_err(|e| SecretStoreError::Backend(format!("keyring init: {e}")))?;
        Ok(Self { rt, keyring })
    }
}

impl SecretStore for OsSecretStore {
    fn get_secret(&self, id: &ServiceId) -> Result<SecretMaterial, SecretStoreError> {
        let sid = id.0.as_str();
        let items = self
            .rt
            .block_on(self.keyring.search_items(&[
                ("application", APP_ATTR),
                (KIND_ATTR, KIND_CRED),
                ("service_id", sid),
            ]))
            .map_err(|e| SecretStoreError::Backend(e.to_string()))?;
        let item = items.into_iter().next().ok_or(SecretStoreError::NotFound)?;
        let secret = self
            .rt
            .block_on(item.secret())
            .map_err(|e| SecretStoreError::Backend(e.to_string()))?;
        Ok(SecretMaterial::new(secret.as_bytes().to_vec()))
    }

    fn put_secret(
        &mut self,
        id: ServiceId,
        secret: SecretMaterial,
    ) -> Result<(), SecretStoreError> {
        let label = format!("comboauth:cred:{}", id.0);
        let sid = id.0.clone();
        let bytes = zeroize::Zeroizing::new(secret.expose_bytes().to_vec());
        self.rt
            .block_on(self.keyring.create_item(
                &label,
                &[
                    ("application", APP_ATTR),
                    (KIND_ATTR, KIND_CRED),
                    ("service_id", sid.as_str()),
                ],
                bytes,
                true,
            ))
            .map_err(|e| SecretStoreError::Backend(e.to_string()))
    }

    fn delete_secret(&mut self, id: &ServiceId) -> Result<(), SecretStoreError> {
        let sid = id.0.as_str();
        self.rt
            .block_on(self.keyring.delete(&[
                ("application", APP_ATTR),
                (KIND_ATTR, KIND_CRED),
                ("service_id", sid),
            ]))
            .map_err(|e| SecretStoreError::Backend(e.to_string()))
    }

    fn contains_secret(&self, id: &ServiceId) -> bool {
        let sid = id.0.as_str();
        self.rt
            .block_on(self.keyring.search_items(&[
                ("application", APP_ATTR),
                (KIND_ATTR, KIND_CRED),
                ("service_id", sid),
            ]))
            .map(|items| !items.is_empty())
            .unwrap_or(false)
    }
}

// ── Typed item ops (profiles + registry) ─────────────────────────────────────

impl OsSecretStore {
    /// Store an arbitrary payload under (kind, item_id). Overwrites if exists.
    fn put_item(&mut self, kind: &str, item_id: &str, bytes: Vec<u8>) -> Result<(), SecretStoreError> {
        let label = format!("comboauth:{kind}:{item_id}");
        self.rt
            .block_on(self.keyring.create_item(
                &label,
                &[
                    ("application", APP_ATTR),
                    (KIND_ATTR, kind),
                    ("item_id", item_id),
                ],
                bytes,
                true,
            ))
            .map_err(|e| SecretStoreError::Backend(e.to_string()))
    }

    /// Retrieve a payload by (kind, item_id).
    fn get_item(&self, kind: &str, item_id: &str) -> Result<Vec<u8>, SecretStoreError> {
        let items = self
            .rt
            .block_on(self.keyring.search_items(&[
                ("application", APP_ATTR),
                (KIND_ATTR, kind),
                ("item_id", item_id),
            ]))
            .map_err(|e| SecretStoreError::Backend(e.to_string()))?;
        let item = items.into_iter().next().ok_or(SecretStoreError::NotFound)?;
        let secret = self
            .rt
            .block_on(item.secret())
            .map_err(|e| SecretStoreError::Backend(e.to_string()))?;
        Ok(secret.as_bytes().to_vec())
    }

    /// Delete an item by (kind, item_id).
    fn delete_item(&mut self, kind: &str, item_id: &str) -> Result<(), SecretStoreError> {
        self.rt
            .block_on(self.keyring.delete(&[
                ("application", APP_ATTR),
                (KIND_ATTR, kind),
                ("item_id", item_id),
            ]))
            .map_err(|e| SecretStoreError::Backend(e.to_string()))
    }

    /// Return all payloads for a given kind (e.g., all combo profiles).
    fn list_item_payloads(&self, kind: &str) -> Result<Vec<Vec<u8>>, SecretStoreError> {
        let items = self
            .rt
            .block_on(
                self.keyring
                    .search_items(&[("application", APP_ATTR), (KIND_ATTR, kind)]),
            )
            .map_err(|e| SecretStoreError::Backend(e.to_string()))?;
        let mut payloads = Vec::with_capacity(items.len());
        for item in items {
            let secret = self
                .rt
                .block_on(item.secret())
                .map_err(|e| SecretStoreError::Backend(e.to_string()))?;
            payloads.push(secret.as_bytes().to_vec());
        }
        Ok(payloads)
    }
}

// ── OsPersistenceStore ────────────────────────────────────────────────────────

/// Linux persistence backend: profiles and registry stored in GNOME Keyring via oo7.
/// Separate instance from OsSecretStore to keep credential and profile namespaces independent.
pub struct OsPersistenceStore {
    inner: OsSecretStore,
}

impl std::fmt::Debug for OsPersistenceStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OsPersistenceStore").finish_non_exhaustive()
    }
}

impl OsPersistenceStore {
    pub fn new() -> Result<Self, PersistenceError> {
        OsSecretStore::new()
            .map(|inner| Self { inner })
            .map_err(|e| PersistenceError::Backend(e.to_string()))
    }
}

impl PersistenceStore for OsPersistenceStore {
    fn save_profile(&mut self, profile: &ComboProfile) -> Result<(), PersistenceError> {
        let dto = ComboProfileDto::from(profile);
        let bytes = serde_json::to_vec(&dto)
            .map_err(|e| PersistenceError::Serialize(e.to_string()))?;
        self.inner
            .put_item(KIND_PROFILE, &profile.id.0, bytes)
            .map_err(|e| PersistenceError::Backend(e.to_string()))
    }

    fn load_profiles(&self) -> Result<Vec<ComboProfile>, PersistenceError> {
        let payloads = self
            .inner
            .list_item_payloads(KIND_PROFILE)
            .map_err(|e| PersistenceError::Backend(e.to_string()))?;
        payloads
            .into_iter()
            .map(|bytes| {
                let dto: ComboProfileDto = serde_json::from_slice(&bytes)
                    .map_err(|e| PersistenceError::Serialize(e.to_string()))?;
                ComboProfile::try_from(dto)
            })
            .collect()
    }

    fn delete_profile(&mut self, id: &ComboProfileId) -> Result<(), PersistenceError> {
        self.inner
            .delete_item(KIND_PROFILE, &id.0)
            .map_err(|e| PersistenceError::Backend(e.to_string()))
    }

    fn save_registry(&mut self, registry: &ServiceRegistry) -> Result<(), PersistenceError> {
        let dto = ServiceRegistryDto::from(registry);
        let bytes = serde_json::to_vec(&dto)
            .map_err(|e| PersistenceError::Serialize(e.to_string()))?;
        self.inner
            .put_item(KIND_REGISTRY, REGISTRY_ITEM_ID, bytes)
            .map_err(|e| PersistenceError::Backend(e.to_string()))
    }

    fn load_registry(&self) -> Result<ServiceRegistry, PersistenceError> {
        match self.inner.get_item(KIND_REGISTRY, REGISTRY_ITEM_ID) {
            Ok(bytes) => {
                let dto: ServiceRegistryDto = serde_json::from_slice(&bytes)
                    .map_err(|e| PersistenceError::Serialize(e.to_string()))?;
                Ok(ServiceRegistry::from(dto))
            }
            Err(SecretStoreError::NotFound) => Ok(ServiceRegistry::default()),
            Err(e) => Err(PersistenceError::Backend(e.to_string())),
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Integration test — requires GNOME Keyring or KeePassXC Secret Service bridge.
    /// Run: cargo test -- --ignored os_secret_store_round_trips
    #[test]
    #[ignore = "requires running Secret Service (GNOME Keyring or KeePassXC)"]
    fn os_secret_store_round_trips() {
        let mut store = OsSecretStore::new().expect("Secret Service unavailable");
        let id = ServiceId("comboauth-test-svc".to_owned());

        let _ = store.delete_secret(&id);

        store
            .put_secret(id.clone(), SecretMaterial::new(b"round-trip-test-value".to_vec()))
            .expect("put_secret failed");

        assert!(store.contains_secret(&id));

        let retrieved = store.get_secret(&id).expect("get_secret failed");
        assert_eq!(retrieved.expose_bytes(), b"round-trip-test-value");

        store.delete_secret(&id).expect("delete_secret failed");
        assert!(!store.contains_secret(&id));
    }

    /// Integration test for typed item persistence (profiles + registry).
    #[test]
    #[ignore = "requires running Secret Service (GNOME Keyring or KeePassXC)"]
    fn os_persistence_store_round_trips() {
        use crate::service::{ServiceRecord, ServiceStatus};

        let mut store = OsPersistenceStore::new().expect("Secret Service unavailable");

        // Clean up any previous test residue.
        let _ = store.delete_profile(&ComboProfileId("test-profile".to_owned()));

        // Save and reload a profile.
        let profile = ComboProfile {
            id: ComboProfileId("test-profile".to_owned()),
            name: "Test".to_owned(),
            sequence: "up down A".to_owned(),
            status: "recorded".to_owned(),
            timing_window_ms: 300,
            gaps_ms: vec![100, 80],
        };
        store.save_profile(&profile).unwrap();

        let loaded = store.load_profiles().unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].gaps_ms, vec![100, 80]);

        store.delete_profile(&profile.id).unwrap();
        assert!(store.load_profiles().unwrap().is_empty());

        // Save and reload the registry.
        let reg = ServiceRegistry::new(vec![ServiceRecord {
            id: ServiceId("svc-os-test".to_owned()),
            name: "OS Test Svc".to_owned(),
            username: "user".to_owned(),
            combo_profile_id: None,
            pinned: false,
            status: ServiceStatus::Unassigned,
        }]);
        store.save_registry(&reg).unwrap();
        let loaded_reg = store.load_registry().unwrap();
        assert_eq!(loaded_reg.services().len(), 1);
        assert_eq!(loaded_reg.services()[0].name, "OS Test Svc");
    }
}
