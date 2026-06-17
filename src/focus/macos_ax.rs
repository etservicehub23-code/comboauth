//! macOS focused-element classification via the Accessibility (AX) API.
//!
//! AX field detection is advisory only — coverage is incomplete for
//! Electron apps, sandboxed browsers, and custom widgets. Callers must
//! treat `FieldKind::Unknown` as "ask the user", never as "safe to paste".

use std::ptr;

use accessibility_sys::{
    AXIsProcessTrustedWithOptions, AXUIElementCopyAttributeValue, AXUIElementCreateSystemWide,
    AXUIElementRef, kAXFocusedUIElementAttribute, kAXRoleAttribute, kAXSecureTextFieldSubrole,
    kAXSubroleAttribute, kAXTextFieldRole, kAXTrustedCheckOptionPrompt,
};
use core_foundation::base::TCFType;
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::CFDictionary;
use core_foundation::string::CFString;
use core_foundation_sys::base::{CFRelease, CFTypeRef};
use core_foundation_sys::string::CFStringRef;

use super::FieldKind;

/// Triggers the macOS Accessibility permission prompt if not already granted.
/// Must be called once at daemon startup, before any AX queries.
pub fn ensure_trusted_with_prompt() -> bool {
    unsafe {
        let key = CFString::wrap_under_get_rule(kAXTrustedCheckOptionPrompt);
        let value = CFBoolean::true_value();
        let options = CFDictionary::from_CFType_pairs(&[(key, value)]);
        AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef())
    }
}

/// Best-effort classification of the currently focused UI element.
pub fn focused_field_kind() -> FieldKind {
    unsafe {
        let system_wide: AXUIElementRef = AXUIElementCreateSystemWide();
        if system_wide.is_null() {
            return FieldKind::Unknown;
        }

        let focused_raw = copy_attribute_raw(system_wide, kAXFocusedUIElementAttribute);
        CFRelease(system_wide as CFTypeRef);

        let focused_raw = match focused_raw {
            Some(value) => value,
            None => return FieldKind::Unknown,
        };
        let focused_element = focused_raw as AXUIElementRef;

        let subrole = copy_string_attribute(focused_element, kAXSubroleAttribute);
        let role = copy_string_attribute(focused_element, kAXRoleAttribute);
        CFRelease(focused_raw);

        if subrole.as_deref() == Some(kAXSecureTextFieldSubrole) {
            FieldKind::Secure
        } else if role.as_deref() == Some(kAXTextFieldRole) {
            FieldKind::Editable
        } else {
            FieldKind::NonEditable
        }
    }
}

/// Copies an attribute value, returning the raw owned (+1 ref) CFTypeRef.
/// Caller is responsible for releasing the returned pointer.
unsafe fn copy_attribute_raw(element: AXUIElementRef, attribute: &str) -> Option<CFTypeRef> {
    let attr = CFString::new(attribute);
    let mut value: CFTypeRef = ptr::null();
    let err =
        unsafe { AXUIElementCopyAttributeValue(element, attr.as_concrete_TypeRef(), &mut value) };
    if err == 0 && !value.is_null() {
        Some(value)
    } else {
        None
    }
}

/// Copies a string-valued attribute and releases the underlying CFStringRef.
unsafe fn copy_string_attribute(element: AXUIElementRef, attribute: &str) -> Option<String> {
    let value = unsafe { copy_attribute_raw(element, attribute) }?;
    Some(unsafe { CFString::wrap_under_create_rule(value as CFStringRef) }.to_string())
}
