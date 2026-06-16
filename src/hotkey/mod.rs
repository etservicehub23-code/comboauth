#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "linux")]
pub mod linux;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyEvent {
    CtrlK,
}
