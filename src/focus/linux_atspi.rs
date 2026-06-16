// Phase 9-D: query focused element via atspi crate.
// Detect Role::PasswordText; return Unknown aggressively when D-Bus unavailable.
pub fn focused_field_kind() -> super::FieldKind {
    super::FieldKind::Unknown // placeholder until Phase 9-D
}
