#![allow(dead_code)]

use crate::profile::ComboProfileId;
use crate::service::ServiceId;
use crate::vault::SecretStoreError;

/// Result of a combo activation attempt. `Activated` carries only metadata —
/// never secret bytes — so it is safe to store in App state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivationResult {
    Waiting,
    Activated {
        service_id: ServiceId,
        service_name: String,
    },
    InvalidInput,
    NoMatch,
    TimingMismatch,
    Locked,
    NoServiceForCombo {
        combo_profile_id: ComboProfileId,
        combo_name: String,
    },
    SecretUnavailable {
        service_id: ServiceId,
        service_name: String,
        error: SecretStoreError,
    },
    DeliveryFailed {
        service_id: ServiceId,
        service_name: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activation_result_activated_contains_no_secret() {
        // Structural proof: the Activated variant has no secret field.
        let result = ActivationResult::Activated {
            service_id: ServiceId("github".to_owned()),
            service_name: "GitHub".to_owned(),
        };
        assert!(matches!(result, ActivationResult::Activated { .. }));
        let debug = format!("{result:?}");
        assert!(debug.contains("GitHub"));
        // No secret bytes in the variant — confirmed by type structure.
    }
}
