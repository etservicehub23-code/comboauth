#![allow(dead_code)]

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::profile::{ComboProfile, ComboProfileId};
use crate::service::{ServiceId, ServiceRecord, ServiceRegistry, ServiceStatus};

pub const SCHEMA_VERSION: u32 = 1;

// ── DTOs ──────────────────────────────────────────────────────────────────────

/// Serializable form of ComboProfile. Stored as the secret payload in the keychain.
/// Sequences and timing gaps live here — never in item labels or attributes.
#[derive(Serialize, Deserialize)]
pub struct ComboProfileDto {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub sequence: String,
    pub status: String,
    pub timing_window_ms: u32,
    pub gaps_ms: Vec<u64>,
}

/// Serializable form of a single service record.
/// `status` is intentionally absent — recomputed from SecretStore state at load time.
#[derive(Serialize, Deserialize)]
pub struct ServiceRecordDto {
    pub id: String,
    pub name: String,
    pub username: String,
    pub combo_profile_id: Option<String>,
    pub pinned: bool,
}

/// Serializable form of the full ServiceRegistry.
#[derive(Serialize, Deserialize)]
pub struct ServiceRegistryDto {
    pub schema_version: u32,
    pub services: Vec<ServiceRecordDto>,
}

// ── Conversions ───────────────────────────────────────────────────────────────

impl From<&ComboProfile> for ComboProfileDto {
    fn from(p: &ComboProfile) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            id: p.id.0.clone(),
            name: p.name.clone(),
            sequence: p.sequence.clone(),
            status: p.status.clone(),
            timing_window_ms: p.timing_window_ms,
            gaps_ms: p.gaps_ms.clone(),
        }
    }
}

impl TryFrom<ComboProfileDto> for ComboProfile {
    type Error = PersistenceError;

    fn try_from(dto: ComboProfileDto) -> Result<Self, Self::Error> {
        if dto.schema_version != SCHEMA_VERSION {
            return Err(PersistenceError::Serialize(format!(
                "unsupported schema_version: {}",
                dto.schema_version
            )));
        }
        if dto.id.is_empty() {
            return Err(PersistenceError::Serialize("profile id is empty".into()));
        }
        if dto.sequence.is_empty() {
            return Err(PersistenceError::Serialize("sequence is empty".into()));
        }
        Ok(ComboProfile {
            id: ComboProfileId(dto.id),
            name: dto.name,
            sequence: dto.sequence,
            status: dto.status,
            timing_window_ms: dto.timing_window_ms,
            gaps_ms: dto.gaps_ms,
        })
    }
}

impl From<&ServiceRecord> for ServiceRecordDto {
    fn from(r: &ServiceRecord) -> Self {
        Self {
            id: r.id.0.clone(),
            name: r.name.clone(),
            username: r.username.clone(),
            combo_profile_id: r.combo_profile_id.as_ref().map(|c| c.0.clone()),
            pinned: r.pinned,
        }
    }
}

impl From<ServiceRecordDto> for ServiceRecord {
    fn from(dto: ServiceRecordDto) -> Self {
        ServiceRecord {
            id: ServiceId(dto.id),
            name: dto.name,
            username: dto.username,
            combo_profile_id: dto.combo_profile_id.map(ComboProfileId),
            pinned: dto.pinned,
            status: ServiceStatus::Unassigned, // recomputed by sync_service_statuses
        }
    }
}

impl From<&ServiceRegistry> for ServiceRegistryDto {
    fn from(reg: &ServiceRegistry) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            services: reg.services().iter().map(ServiceRecordDto::from).collect(),
        }
    }
}

impl TryFrom<ServiceRegistryDto> for ServiceRegistry {
    type Error = PersistenceError;

    fn try_from(dto: ServiceRegistryDto) -> Result<Self, Self::Error> {
        if dto.schema_version != SCHEMA_VERSION {
            return Err(PersistenceError::Serialize(format!(
                "unsupported registry schema_version: {}",
                dto.schema_version
            )));
        }
        Ok(ServiceRegistry::new(
            dto.services.into_iter().map(ServiceRecord::from).collect(),
        ))
    }
}

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum PersistenceError {
    #[error("serialization error: {0}")]
    Serialize(String),
    #[error("backend error: {0}")]
    Backend(String),
    #[error("item not found")]
    NotFound,
}

// ── Trait ─────────────────────────────────────────────────────────────────────

pub trait PersistenceStore: std::fmt::Debug {
    /// Upsert a combo profile (stored as encrypted JSON in keychain).
    fn save_profile(&mut self, profile: &ComboProfile) -> Result<(), PersistenceError>;
    /// Load all combo profiles. Returns empty vec when none are stored.
    fn load_profiles(&self) -> Result<Vec<ComboProfile>, PersistenceError>;
    /// Delete a combo profile by ID.
    fn delete_profile(&mut self, id: &ComboProfileId) -> Result<(), PersistenceError>;
    /// Upsert the full service registry (stored as encrypted JSON in keychain).
    fn save_registry(&mut self, registry: &ServiceRegistry) -> Result<(), PersistenceError>;
    /// Load the service registry. Returns an empty registry when none is stored.
    fn load_registry(&self) -> Result<ServiceRegistry, PersistenceError>;
}

// ── Mock implementation ───────────────────────────────────────────────────────

#[derive(Debug, Default)]
pub struct MockPersistenceStore {
    profiles: HashMap<ComboProfileId, ComboProfile>,
    registry: Option<ServiceRegistry>,
}

impl MockPersistenceStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl PersistenceStore for MockPersistenceStore {
    fn save_profile(&mut self, profile: &ComboProfile) -> Result<(), PersistenceError> {
        self.profiles.insert(profile.id.clone(), profile.clone());
        Ok(())
    }

    fn load_profiles(&self) -> Result<Vec<ComboProfile>, PersistenceError> {
        Ok(self.profiles.values().cloned().collect())
    }

    fn delete_profile(&mut self, id: &ComboProfileId) -> Result<(), PersistenceError> {
        self.profiles.remove(id).ok_or(PersistenceError::NotFound)?;
        Ok(())
    }

    fn save_registry(&mut self, registry: &ServiceRegistry) -> Result<(), PersistenceError> {
        self.registry = Some(registry.clone());
        Ok(())
    }

    fn load_registry(&self) -> Result<ServiceRegistry, PersistenceError> {
        Ok(self.registry.clone().unwrap_or_default())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::ServiceRecord;

    fn make_profile(id: &str) -> ComboProfile {
        ComboProfile {
            id: ComboProfileId(id.to_owned()),
            name: id.to_owned(),
            sequence: "down right A".to_owned(),
            status: "recorded".to_owned(),
            timing_window_ms: 300,
            gaps_ms: vec![120, 95],
        }
    }

    #[test]
    fn mock_store_round_trips_profile() {
        let mut store = MockPersistenceStore::new();
        let profile = make_profile("quick-fire");
        store.save_profile(&profile).unwrap();
        let loaded = store.load_profiles().unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, profile.id);
        assert_eq!(loaded[0].gaps_ms, vec![120, 95]);
    }

    #[test]
    fn mock_store_delete_profile() {
        let mut store = MockPersistenceStore::new();
        let profile = make_profile("to-delete");
        store.save_profile(&profile).unwrap();
        store.delete_profile(&profile.id).unwrap();
        assert!(store.load_profiles().unwrap().is_empty());
    }

    #[test]
    fn mock_store_round_trips_registry() {
        let mut store = MockPersistenceStore::new();
        let reg = ServiceRegistry::new(vec![ServiceRecord {
            id: ServiceId("github".to_owned()),
            name: "GitHub".to_owned(),
            username: "dev".to_owned(),
            combo_profile_id: Some(ComboProfileId("quick-fire".to_owned())),
            pinned: true,
            status: ServiceStatus::Unassigned,
        }]);
        store.save_registry(&reg).unwrap();
        let loaded = store.load_registry().unwrap();
        assert_eq!(loaded.services().len(), 1);
        assert_eq!(loaded.services()[0].name, "GitHub");
        assert_eq!(
            loaded.services()[0].combo_profile_id,
            Some(ComboProfileId("quick-fire".to_owned()))
        );
        // status is always Unassigned after load (recomputed later)
        assert_eq!(loaded.services()[0].status, ServiceStatus::Unassigned);
    }

    #[test]
    fn combo_profile_dto_round_trips_via_json() {
        let profile = make_profile("round-trip");
        let dto = ComboProfileDto::from(&profile);
        let json = serde_json::to_string(&dto).unwrap();
        let decoded: ComboProfileDto = serde_json::from_str(&json).unwrap();
        let restored = ComboProfile::try_from(decoded).unwrap();
        assert_eq!(restored.id, profile.id);
        assert_eq!(restored.gaps_ms, profile.gaps_ms);
    }

    #[test]
    fn service_registry_dto_round_trips_via_json() {
        let reg = ServiceRegistry::new(vec![ServiceRecord {
            id: ServiceId("svc-1".to_owned()),
            name: "SVC".to_owned(),
            username: "u".to_owned(),
            combo_profile_id: None,
            pinned: false,
            status: ServiceStatus::Unassigned,
        }]);
        let dto = ServiceRegistryDto::from(&reg);
        let json = serde_json::to_string(&dto).unwrap();
        let decoded: ServiceRegistryDto = serde_json::from_str(&json).unwrap();
        let restored = ServiceRegistry::try_from(decoded).unwrap();
        assert_eq!(restored.services().len(), 1);
        assert_eq!(restored.services()[0].id, ServiceId("svc-1".to_owned()));
    }

    #[test]
    fn service_registry_dto_rejects_wrong_schema_version() {
        let json = r#"{"schema_version":99,"services":[]}"#;
        let dto: ServiceRegistryDto = serde_json::from_str(json).unwrap();
        let err = ServiceRegistry::try_from(dto).unwrap_err();
        assert!(matches!(err, PersistenceError::Serialize(_)));
    }

    #[test]
    fn combo_profile_dto_rejects_wrong_schema_version() {
        let json = r#"{"schema_version":99,"id":"p1","name":"P","sequence":"down right A","status":"recorded","timing_window_ms":300,"gaps_ms":[100,80]}"#;
        let dto: ComboProfileDto = serde_json::from_str(json).unwrap();
        let err = ComboProfile::try_from(dto).unwrap_err();
        assert!(matches!(err, PersistenceError::Serialize(_)));
    }

    #[test]
    fn combo_profile_dto_rejects_empty_sequence() {
        let json = r#"{"schema_version":1,"id":"p1","name":"P","sequence":"","status":"recorded","timing_window_ms":300,"gaps_ms":[]}"#;
        let dto: ComboProfileDto = serde_json::from_str(json).unwrap();
        let err = ComboProfile::try_from(dto).unwrap_err();
        assert!(matches!(err, PersistenceError::Serialize(_)));
    }

    #[test]
    fn combo_profile_dto_rejects_empty_id() {
        let json = r#"{"schema_version":1,"id":"","name":"P","sequence":"down right A","status":"recorded","timing_window_ms":300,"gaps_ms":[]}"#;
        let dto: ComboProfileDto = serde_json::from_str(json).unwrap();
        let err = ComboProfile::try_from(dto).unwrap_err();
        assert!(matches!(err, PersistenceError::Serialize(_)));
    }

    #[test]
    fn combo_profile_dto_truncated_json_returns_error() {
        let truncated = r#"{"schema_version":1,"id":"p1","name":"P","sequence":"down"#;
        assert!(serde_json::from_str::<ComboProfileDto>(truncated).is_err());
    }
}
