#![cfg(target_os = "linux")]
#![allow(dead_code)]

use tokio::runtime::Runtime;

use crate::service::ServiceId;
use super::{SecretMaterial, SecretStore, SecretStoreError};

const APP_ATTR: &str = "comboauth";

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
            .block_on(
                self.keyring
                    .search_items(&[("application", APP_ATTR), ("service_id", sid)]),
            )
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
        let label = format!("comboauth: {}", id.0);
        let sid = id.0.clone();
        let bytes = secret.expose_bytes().to_vec();
        self.rt
            .block_on(self.keyring.create_item(
                &label,
                &[("application", APP_ATTR), ("service_id", sid.as_str())],
                bytes,
                true,
            ))
            .map_err(|e| SecretStoreError::Backend(e.to_string()))
    }

    fn delete_secret(&mut self, id: &ServiceId) -> Result<(), SecretStoreError> {
        let sid = id.0.as_str();
        self.rt
            .block_on(
                self.keyring
                    .delete(&[("application", APP_ATTR), ("service_id", sid)]),
            )
            .map_err(|e| SecretStoreError::Backend(e.to_string()))
    }

    fn contains_secret(&self, id: &ServiceId) -> bool {
        let sid = id.0.as_str();
        self.rt
            .block_on(
                self.keyring
                    .search_items(&[("application", APP_ATTR), ("service_id", sid)]),
            )
            .map(|items| !items.is_empty())
            .unwrap_or(false)
    }
}

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
}
