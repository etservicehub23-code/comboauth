#![cfg(target_os = "macos")]
#![cfg(feature = "macos-keychain")]
#![allow(dead_code)]

use crate::service::ServiceId;
use crate::vault::{SecretMaterial, SecretStore, SecretStoreError};

const SERVICE: &str = "comboauth";

/// macOS Keychain backend using GenericPassword items.
///
/// Each credential is stored as a generic password keyed by ServiceId.
/// Service attribute: "comboauth". Account attribute: the ServiceId string.
pub struct MacosKeychainStore;

impl SecretStore for MacosKeychainStore {
    fn get_secret(&self, id: &ServiceId) -> Result<SecretMaterial, SecretStoreError> {
        security_framework::passwords::get_generic_password(SERVICE, &id.0)
            .map(|bytes| SecretMaterial::new(bytes))
            .map_err(|e| {
                if e.code() == security_framework_sys::base::errSecItemNotFound {
                    SecretStoreError::NotFound
                } else {
                    SecretStoreError::Backend(e.to_string())
                }
            })
    }

    fn put_secret(&mut self, id: ServiceId, secret: SecretMaterial) -> Result<(), SecretStoreError> {
        security_framework::passwords::set_generic_password(SERVICE, &id.0, secret.expose_bytes())
            .map_err(|e| SecretStoreError::Backend(e.to_string()))
    }

    fn delete_secret(&mut self, id: &ServiceId) -> Result<(), SecretStoreError> {
        security_framework::passwords::delete_generic_password(SERVICE, &id.0)
            .map_err(|e| {
                if e.code() == security_framework_sys::base::errSecItemNotFound {
                    SecretStoreError::NotFound
                } else {
                    SecretStoreError::Backend(e.to_string())
                }
            })
    }

    fn contains_secret(&self, id: &ServiceId) -> bool {
        security_framework::passwords::get_generic_password(SERVICE, &id.0).is_ok()
    }
}
