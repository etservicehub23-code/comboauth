#![allow(dead_code)]

use crate::profile::ComboProfileId;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ServiceId(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceStatus {
    Ready,
    MissingSecret,
    Unassigned,
    Disabled,
}

impl ServiceStatus {
    pub fn label(&self) -> &'static str {
        match self {
            ServiceStatus::Ready => "ready",
            ServiceStatus::MissingSecret => "missing secret",
            ServiceStatus::Unassigned => "unassigned",
            ServiceStatus::Disabled => "disabled",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceRecord {
    pub id: ServiceId,
    pub name: String,
    pub username: String,
    pub combo_profile_id: Option<ComboProfileId>,
    pub pinned: bool,
    pub status: ServiceStatus,
}

#[derive(Debug, Clone, Default)]
pub struct ServiceRegistry {
    services: Vec<ServiceRecord>,
}

impl ServiceRegistry {
    pub fn new(services: Vec<ServiceRecord>) -> Self {
        Self { services }
    }

    pub fn services(&self) -> &[ServiceRecord] {
        &self.services
    }

    pub fn pinned_services(&self) -> impl Iterator<Item = &ServiceRecord> {
        self.services.iter().filter(|s| s.pinned)
    }

    pub fn get(&self, id: &ServiceId) -> Option<&ServiceRecord> {
        self.services.iter().find(|s| &s.id == id)
    }

    pub fn get_mut(&mut self, id: &ServiceId) -> Option<&mut ServiceRecord> {
        self.services.iter_mut().find(|s| &s.id == id)
    }

    pub fn add(&mut self, service: ServiceRecord) -> Result<(), ServiceRegistryError> {
        if self.services.iter().any(|s| s.id == service.id) {
            return Err(ServiceRegistryError::DuplicateService);
        }
        self.services.push(service);
        Ok(())
    }

    pub fn assign_combo(
        &mut self,
        service_id: &ServiceId,
        combo_profile_id: ComboProfileId,
    ) -> Result<(), ServiceRegistryError> {
        // Clone the existing owner id to avoid holding a shared borrow into self.
        let existing_owner = self.services.iter().find_map(|s| {
            if s.combo_profile_id.as_ref() == Some(&combo_profile_id) {
                Some(s.id.clone())
            } else {
                None
            }
        });
        if let Some(owner_id) = existing_owner {
            if &owner_id != service_id {
                return Err(ServiceRegistryError::ComboAlreadyAssigned);
            }
        }
        let service = self.get_mut(service_id).ok_or(ServiceRegistryError::NotFound)?;
        service.combo_profile_id = Some(combo_profile_id);
        Ok(())
    }

    pub fn service_for_combo_profile(
        &self,
        combo_profile_id: &ComboProfileId,
    ) -> Option<&ServiceRecord> {
        self.services
            .iter()
            .find(|s| s.combo_profile_id.as_ref() == Some(combo_profile_id))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ServiceRegistryError {
    #[error("service already exists")]
    DuplicateService,
    #[error("combo already assigned")]
    ComboAlreadyAssigned,
    #[error("service not found")]
    NotFound,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::ComboProfileId;

    fn make_record(id: &str, pinned: bool) -> ServiceRecord {
        ServiceRecord {
            id: ServiceId(id.to_owned()),
            name: id.to_uppercase(),
            username: String::new(),
            combo_profile_id: None,
            pinned,
            status: ServiceStatus::Unassigned,
        }
    }

    #[test]
    fn service_registry_lists_pinned_services_in_order() {
        let registry = ServiceRegistry::new(vec![
            make_record("a", true),
            make_record("b", false),
            make_record("c", true),
        ]);
        let pinned: Vec<_> = registry.pinned_services().collect();
        assert_eq!(pinned.len(), 2);
        assert_eq!(pinned[0].id, ServiceId("a".to_owned()));
        assert_eq!(pinned[1].id, ServiceId("c".to_owned()));
    }

    #[test]
    fn service_registry_rejects_duplicate_combo_assignment() {
        let mut registry = ServiceRegistry::new(vec![
            make_record("svc-1", false),
            make_record("svc-2", false),
        ]);
        let combo_id = ComboProfileId("quarter-turn".to_owned());
        registry
            .assign_combo(&ServiceId("svc-1".to_owned()), combo_id.clone())
            .unwrap();
        let err = registry
            .assign_combo(&ServiceId("svc-2".to_owned()), combo_id)
            .unwrap_err();
        assert_eq!(err, ServiceRegistryError::ComboAlreadyAssigned);
    }
}
