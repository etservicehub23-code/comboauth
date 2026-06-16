#![cfg(target_os = "macos")]
#![cfg(feature = "macos-keychain")]
#![allow(dead_code)]

use crate::persistence::{ComboProfileDto, PersistenceError, PersistenceStore, ServiceRegistryDto};
use crate::profile::{ComboProfile, ComboProfileId};
use crate::service::{ServiceId, ServiceRegistry};
use crate::vault::{SecretMaterial, SecretStore, SecretStoreError};

const SERVICE_CRED: &str = "comboauth";
const SERVICE_PROFILE: &str = "comboauth.profile";
const SERVICE_REGISTRY: &str = "comboauth.registry";
const MANIFEST_ACCOUNT: &str = "_manifest";
const REGISTRY_ACCOUNT: &str = "default";

// ── Credential store ──────────────────────────────────────────────────────────

/// macOS Keychain backend using GenericPassword items.
///
/// Each credential is stored as a generic password keyed by ServiceId.
/// Service attribute: "comboauth". Account attribute: the ServiceId string.
#[derive(Debug)]
pub struct MacosKeychainStore;

impl SecretStore for MacosKeychainStore {
    fn get_secret(&self, id: &ServiceId) -> Result<SecretMaterial, SecretStoreError> {
        security_framework::passwords::get_generic_password(SERVICE_CRED, &id.0)
            .map(|bytes| SecretMaterial::new(bytes))
            .map_err(|e| {
                if e.code() == -25300 {
                    SecretStoreError::NotFound
                } else {
                    SecretStoreError::Backend(e.to_string())
                }
            })
    }

    fn put_secret(&mut self, id: ServiceId, secret: SecretMaterial) -> Result<(), SecretStoreError> {
        security_framework::passwords::set_generic_password(SERVICE_CRED, &id.0, secret.expose_bytes())
            .map_err(|e| SecretStoreError::Backend(e.to_string()))
    }

    fn delete_secret(&mut self, id: &ServiceId) -> Result<(), SecretStoreError> {
        security_framework::passwords::delete_generic_password(SERVICE_CRED, &id.0)
            .map_err(|e| {
                if e.code() == -25300 {
                    SecretStoreError::NotFound
                } else {
                    SecretStoreError::Backend(e.to_string())
                }
            })
    }

    fn contains_secret(&self, id: &ServiceId) -> bool {
        security_framework::passwords::get_generic_password(SERVICE_CRED, &id.0).is_ok()
    }
}

// ── Persistence store ─────────────────────────────────────────────────────────

/// macOS Keychain-backed persistence for combo profiles and service registry.
///
/// Profiles are stored as GenericPassword items under "comboauth.profile".
/// A manifest item ("_manifest" account) holds the list of profile IDs as JSON.
/// The registry is stored under "comboauth.registry" / "default".
#[derive(Debug)]
pub struct MacosPersistenceStore;

impl MacosPersistenceStore {
    pub fn new() -> Self { Self }
}

fn kc_get(service: &str, account: &str) -> Option<Vec<u8>> {
    security_framework::passwords::get_generic_password(service, account).ok()
}

fn kc_set(service: &str, account: &str, bytes: &[u8]) -> Result<(), PersistenceError> {
    security_framework::passwords::set_generic_password(service, account, bytes)
        .map_err(|e| PersistenceError::Backend(e.to_string()))
}

fn kc_delete(service: &str, account: &str) {
    let _ = security_framework::passwords::delete_generic_password(service, account);
}

fn load_manifest() -> Vec<String> {
    kc_get(SERVICE_PROFILE, MANIFEST_ACCOUNT)
        .and_then(|b| serde_json::from_slice::<Vec<String>>(&b).ok())
        .unwrap_or_default()
}

fn save_manifest(ids: &[String]) -> Result<(), PersistenceError> {
    let bytes = serde_json::to_vec(ids).map_err(|e| PersistenceError::Serialize(e.to_string()))?;
    kc_set(SERVICE_PROFILE, MANIFEST_ACCOUNT, &bytes)
}

impl PersistenceStore for MacosPersistenceStore {
    fn save_profile(&mut self, profile: &ComboProfile) -> Result<(), PersistenceError> {
        let dto = ComboProfileDto::from(profile);
        let bytes = serde_json::to_vec(&dto).map_err(|e| PersistenceError::Serialize(e.to_string()))?;
        kc_set(SERVICE_PROFILE, &profile.id.0, &bytes)?;
        let mut ids = load_manifest();
        if !ids.contains(&profile.id.0) {
            ids.push(profile.id.0.clone());
            save_manifest(&ids)?;
        }
        Ok(())
    }

    fn load_profiles(&self) -> Result<Vec<ComboProfile>, PersistenceError> {
        let ids = load_manifest();
        ids.into_iter()
            .filter_map(|id| kc_get(SERVICE_PROFILE, &id))
            .map(|bytes| {
                let dto: ComboProfileDto = serde_json::from_slice(&bytes)
                    .map_err(|e| PersistenceError::Serialize(e.to_string()))?;
                ComboProfile::try_from(dto)
            })
            .collect()
    }

    fn delete_profile(&mut self, id: &ComboProfileId) -> Result<(), PersistenceError> {
        kc_delete(SERVICE_PROFILE, &id.0);
        let ids: Vec<String> = load_manifest().into_iter().filter(|i| i != &id.0).collect();
        save_manifest(&ids)
    }

    fn save_registry(&mut self, registry: &ServiceRegistry) -> Result<(), PersistenceError> {
        let dto = ServiceRegistryDto::from(registry);
        let bytes = serde_json::to_vec(&dto).map_err(|e| PersistenceError::Serialize(e.to_string()))?;
        kc_set(SERVICE_REGISTRY, REGISTRY_ACCOUNT, &bytes)
    }

    fn load_registry(&self) -> Result<ServiceRegistry, PersistenceError> {
        match kc_get(SERVICE_REGISTRY, REGISTRY_ACCOUNT) {
            Some(bytes) => {
                let dto: ServiceRegistryDto = serde_json::from_slice(&bytes)
                    .map_err(|e| PersistenceError::Serialize(e.to_string()))?;
                Ok(ServiceRegistry::try_from(dto)?)
            }
            None => Ok(ServiceRegistry::default()),
        }
    }
}
