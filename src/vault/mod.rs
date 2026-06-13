#![allow(dead_code)]

pub mod mock;

/// Backend trait for storing and retrieving secrets keyed by service name.
pub trait SecretStore {
    /// Look up the placeholder/secret for a service by name.
    /// Returns None if the service is not registered.
    fn get(&self, service: &str) -> Option<&str>;

    /// List all registered service names.
    fn services(&self) -> Vec<&str>;
}
