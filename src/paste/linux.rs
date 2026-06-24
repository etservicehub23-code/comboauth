//! Linux paste: set clipboard via arboard, synthesize Ctrl+V via enigo (X11),
//! then clear the clipboard.
//!
//! `exclude_from_history()` is set on every clipboard write so that clipboard
//! managers (e.g. GPaste, Clipman) do not persist the secret.
//!
//! On Wayland, enigo's Ctrl+V synthesis may fail — the caller can detect this
//! and fall back to clipboard-only (`copy_and_clear`). Phase 9-E adds explicit
//! Wayland detection and desktop notification; until then, we attempt the
//! keystroke and propagate the error.

use std::thread;
use std::time::Duration;

use arboard::Clipboard;
#[cfg(target_os = "linux")]
use arboard::SetExtLinux as _;
use enigo::{Direction, Enigo, Key, Keyboard, Settings};

pub fn paste_and_clear(secret: &str, clear_after_ms: u64) -> Result<(), Box<dyn std::error::Error>> {
    let mut clipboard = Clipboard::new()?;
    let previous = clipboard.get_text().ok();

    // exclude_from_history prevents clipboard managers from persisting the secret.
    clipboard.set().exclude_from_history().text(secret.to_owned())?;

    let paste_result = (|| -> Result<(), Box<dyn std::error::Error>> {
        let mut enigo = Enigo::new(&Settings::default())?;
        enigo.key(Key::Control, Direction::Press)?;
        // If Click fails after Press, enigo's Drop releases held keys (Control).
        enigo.key(Key::Unicode('v'), Direction::Click)?;
        enigo.key(Key::Control, Direction::Release)?;
        Ok(())
    })();

    thread::sleep(Duration::from_millis(clear_after_ms));

    // Cleanup is best-effort; we log the cleanup failure but return the paste
    // result so callers know whether the secret was actually delivered.
    let cleanup_result = match previous {
        Some(text) => clipboard.set_text(text),
        None => clipboard.clear(),
    };
    if let Err(e) = cleanup_result {
        eprintln!("comboauth: clipboard cleanup failed after paste: {e}");
    }

    paste_result
}

/// Write `secret` to clipboard then clear it after `clear_after_ms`, without
/// synthesizing a Ctrl+V keystroke. Used for `PasteDecision::Refuse` (focused
/// field confirmed non-editable) and as the Wayland fallback.
pub fn copy_and_clear(secret: &str, clear_after_ms: u64) -> Result<(), Box<dyn std::error::Error>> {
    let mut clipboard = Clipboard::new()?;
    let previous = clipboard.get_text().ok();

    clipboard.set().exclude_from_history().text(secret.to_owned())?;

    thread::sleep(Duration::from_millis(clear_after_ms));

    let cleanup_result = match previous {
        Some(text) => clipboard.set_text(text),
        None => clipboard.clear(),
    };
    if let Err(e) = cleanup_result {
        eprintln!("comboauth: clipboard cleanup failed after copy: {e}");
    }

    Ok(())
}
