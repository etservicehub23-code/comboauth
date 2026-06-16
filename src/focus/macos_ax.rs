// Phase 9-B: query focused element via accessibility-sys.
// Check kAXSecureTextFieldSubrole / kAXRoleAttribute.
// Requires AXIsProcessTrustedWithOptions; returns Unknown if not trusted.
pub fn focused_field_kind() -> super::FieldKind {
    super::FieldKind::Unknown // placeholder until Phase 9-B
}
