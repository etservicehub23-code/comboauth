#![allow(dead_code)]

use std::collections::HashMap;

use crate::service::ServiceId;
use super::{SecretMaterial, SecretStore, SecretStoreError};

#[derive(Debug)]
pub struct MockSecretStore {
    entries: HashMap<ServiceId, SecretMaterial>,
}

impl MockSecretStore {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }
}

impl Default for MockSecretStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SecretStore for MockSecretStore {
    fn get_secret(&self, id: &ServiceId) -> Result<SecretMaterial, SecretStoreError> {
        self.entries.get(id).cloned().ok_or(SecretStoreError::NotFound)
    }

    fn put_secret(&mut self, id: ServiceId, secret: SecretMaterial) -> Result<(), SecretStoreError> {
        self.entries.insert(id, secret);
        Ok(())
    }

    fn delete_secret(&mut self, id: &ServiceId) -> Result<(), SecretStoreError> {
        if self.entries.remove(id).is_some() {
            Ok(())
        } else {
            Err(SecretStoreError::NotFound)
        }
    }

    fn contains_secret(&self, id: &ServiceId) -> bool {
        self.entries.contains_key(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_secret_store_round_trips_by_service_id() {
        let mut store = MockSecretStore::new();
        let id = ServiceId("github".to_owned());
        store
            .put_secret(id.clone(), SecretMaterial::new(b"test-secret".to_vec()))
            .unwrap();
        let retrieved = store.get_secret(&id).unwrap();
        assert_eq!(retrieved.expose_bytes(), b"test-secret");
    }

    #[test]
    fn mock_secret_store_unknown_returns_not_found() {
        let store = MockSecretStore::new();
        let err = store
            .get_secret(&ServiceId("nonexistent".to_owned()))
            .unwrap_err();
        assert_eq!(err, SecretStoreError::NotFound);
    }
}
