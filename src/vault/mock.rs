use super::SecretStore;

/// In-memory mock vault — maps service names to placeholder secrets.
/// No real credentials; used during M1–M5 development only.
pub struct MockSecretStore {
    entries: Vec<(&'static str, &'static str)>,
}

impl MockSecretStore {
    pub fn new(entries: Vec<(&'static str, &'static str)>) -> Self {
        Self { entries }
    }
}

impl SecretStore for MockSecretStore {
    fn get(&self, service: &str) -> Option<&str> {
        self.entries
            .iter()
            .find(|(name, _)| *name == service)
            .map(|(_, secret)| *secret)
    }

    fn services(&self) -> Vec<&str> {
        self.entries.iter().map(|(name, _)| *name).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_store() -> MockSecretStore {
        MockSecretStore::new(vec![
            ("GitHub", "***mock-gh-token-abc123***"),
            ("Research Wiki", "***mock-wiki-pass-xyz789***"),
        ])
    }

    #[test]
    fn get_returns_secret_for_known_service() {
        let store = make_store();
        assert_eq!(store.get("GitHub"), Some("***mock-gh-token-abc123***"));
    }

    #[test]
    fn get_returns_none_for_unknown_service() {
        let store = make_store();
        assert_eq!(store.get("NoSuchService"), None);
    }

    #[test]
    fn services_lists_all_names() {
        let store = make_store();
        let names = store.services();
        assert!(names.contains(&"GitHub"));
        assert!(names.contains(&"Research Wiki"));
        assert_eq!(names.len(), 2);
    }
}
