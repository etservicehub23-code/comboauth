//! Linux paste: set clipboard via arboard, synthesize Ctrl+V via enigo (X11),
//! then clear the clipboard.
//!
//! `exclude_from_history()` is set on every clipboard write so that clipboard
//! managers (e.g. GPaste, Clipman) do not persist the secret.
//!
//! On Wayland, enigo's X11 Ctrl+V synthesis is unavailable. `paste_and_clear`
//! detects Wayland via `is_wayland_session()` and falls back to `copy_and_clear`
//! plus a desktop notification ("Secret copied — paste manually") so the user
//! knows to Ctrl+V themselves.

use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use arboard::Clipboard;
#[cfg(target_os = "linux")]
use arboard::SetExtLinux as _;
use enigo::{Direction, Enigo, Key, Keyboard, Settings};

/// Returns `true` when the process is running inside a Wayland session.
///
/// Checks both `WAYLAND_DISPLAY` (set by compositors when the Wayland socket is
/// available) and `XDG_SESSION_TYPE=wayland` (set by login managers such as GDM,
/// SDDM, and systemd-logind). Either signal is sufficient; together they cover
/// daemon launch environments where one may be absent (e.g. systemd user services
/// that do not inherit the compositor socket but do get XDG_SESSION_TYPE from the
/// PAM environment).
pub fn is_wayland_session() -> bool {
    let wayland_display = std::env::var("WAYLAND_DISPLAY").map(|v| !v.is_empty()).unwrap_or(false);
    let xdg_type = std::env::var("XDG_SESSION_TYPE").map(|v| v == "wayland").unwrap_or(false);
    wayland_display || xdg_type
}

/// Send a desktop notification via `notify-send`. Fire-and-forget; silently
/// ignored when `notify-send` is not installed or the notification daemon is
/// absent (headless / server environments).
fn notify_desktop(summary: &str, body: &str) {
    let summary = summary.to_owned();
    let body = body.to_owned();
    thread::spawn(move || {
        // Wait on the child (via status()) so it is reaped before the thread
        // exits — bare spawn() + drop would leave a zombie while the daemon lives.
        let _ = Command::new("notify-send")
            .args(["--urgency=normal", "--expire-time=5000", &summary, &body])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    });
}

pub fn paste_and_clear(secret: &str, clear_after_ms: u64) -> Result<(), Box<dyn std::error::Error>> {
    // Wayland: enigo's X11 Ctrl+V synthesis is not available; fall back to
    // clipboard-only delivery so the user can paste manually.
    if is_wayland_session() {
        // Use a longer clear delay than the caller requested: auto-paste callers pass
        // ~200 ms (just long enough for Ctrl+V to complete), but on Wayland there is
        // no keystroke — the user must paste manually, which realistically takes several
        // seconds. 8000 ms matches the existing copy_and_clear usage for Refuse decisions.
        let manual_clear_ms = clear_after_ms.max(8000);
        eprintln!("comboauth: Wayland session detected — auto-paste unavailable; secret copied to clipboard (clears in {manual_clear_ms} ms)");
        // Notify before blocking in copy_and_clear so the user sees the prompt
        // immediately after the hotkey fires, not after 8 s.
        notify_desktop("ComboAuth", "Secret copied — paste manually");
        return copy_and_clear(secret, manual_clear_ms);
    }

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
