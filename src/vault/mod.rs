#![allow(dead_code)]

pub mod mock;

use crate::service::ServiceId;
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct SecretMaterial(Vec<u8>);

impl SecretMaterial {
    pub fn new(bytes: impl Into<Vec<u8>>) -> Self {
        Self(bytes.into())
    }

    pub fn expose_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl std::fmt::Debug for SecretMaterial {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SecretMaterial([redacted])")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SecretStoreError {
    #[error("secret not found")]
    NotFound,
    #[error("backend error: {0}")]
    Backend(String),
}

pub trait SecretStore {
    fn get_secret(&self, id: &ServiceId) -> Result<SecretMaterial, SecretStoreError>;
    fn put_secret(&mut self, id: ServiceId, secret: SecretMaterial) -> Result<(), SecretStoreError>;
    fn delete_secret(&mut self, id: &ServiceId) -> Result<(), SecretStoreError>;
    fn contains_secret(&self, id: &ServiceId) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_material_debug_is_redacted() {
        let material = SecretMaterial::new(b"super-secret-password".to_vec());
        let debug = format!("{material:?}");
        assert_eq!(debug, "SecretMaterial([redacted])");
        assert!(!debug.contains("super-secret"));
    }
}
