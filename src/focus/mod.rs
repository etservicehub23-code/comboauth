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
