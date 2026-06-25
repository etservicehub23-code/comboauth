#[cfg(target_os = "macos")]
pub mod macos_ax;
#[cfg(target_os = "linux")]
pub mod linux_atspi;

/// Classification of the currently focused UI element.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    /// Confirmed password / secure text field.
    Secure,
    /// Regular editable text field.
    Editable,
    /// Not an editable field.
    NonEditable,
    /// Could not determine (degrade to explicit user confirmation).
    Unknown,
}

pub fn focused_field_kind() -> FieldKind {
    #[cfg(target_os = "macos")]
    return macos_ax::focused_field_kind();
    #[cfg(target_os = "linux")]
    return linux_atspi::focused_field_kind();
    #[allow(unreachable_code)]
    FieldKind::Unknown
}

/// Async variant -- use inside async Tokio tasks (e.g. the IPC handler).
/// The sync `focused_field_kind()` must only be called from non-async threads.
pub async fn focused_field_kind_async() -> FieldKind {
    #[cfg(target_os = "linux")]
    return linux_atspi::focused_field_kind_async().await;
    #[allow(unreachable_code)]
    FieldKind::Unknown
}

/// What to do about auto-pasting into a field of the given `FieldKind`.
///
/// This reduces *accidental* paste into the wrong field; it is not a
/// security boundary -- a malicious or compromised focused app can spoof
/// the accessibility role it reports, so `Secure` is not an authoritative
/// guarantee. See `docs/security/threat-model.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasteDecision {
    /// Confirmed secure field (e.g. a password box) -- paste immediately.
    AutoPaste,
    /// Editable but not confirmed secure, or undetectable -- require one
    /// more explicit confirmation before pasting.
    ConfirmFirst,
    /// Confirmed not editable -- don't synthesize a paste keystroke; copy
    /// to clipboard instead.
    Refuse,
}

pub fn paste_decision(kind: FieldKind) -> PasteDecision {
    match kind {
        FieldKind::Secure => PasteDecision::AutoPaste,
        FieldKind::Editable | FieldKind::Unknown => PasteDecision::ConfirmFirst,
        FieldKind::NonEditable => PasteDecision::Refuse,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secure_auto_pastes() {
        assert_eq!(paste_decision(FieldKind::Secure), PasteDecision::AutoPaste);
    }

    #[test]
    fn editable_and_unknown_require_confirmation() {
        assert_eq!(paste_decision(FieldKind::Editable), PasteDecision::ConfirmFirst);
        assert_eq!(paste_decision(FieldKind::Unknown), PasteDecision::ConfirmFirst);
    }

    #[test]
    fn non_editable_refuses() {
        assert_eq!(paste_decision(FieldKind::NonEditable), PasteDecision::Refuse);
    }
}
