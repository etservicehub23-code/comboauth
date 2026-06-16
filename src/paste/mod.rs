#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "linux")]
pub mod linux;

/// Write `secret` to clipboard, synthesize Cmd+V / Ctrl+V, then clear after `clear_after_ms`.
pub fn paste_and_clear(_secret: &str, _clear_after_ms: u64) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "macos")]
    return macos::paste_and_clear(_secret, _clear_after_ms);
    #[cfg(target_os = "linux")]
    return linux::paste_and_clear(_secret, _clear_after_ms);
    #[allow(unreachable_code)]
    Err("unsupported platform".into())
}
