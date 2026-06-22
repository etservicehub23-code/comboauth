//! macOS paste: set clipboard, synthesize Cmd+V, then clear the clipboard.
//!
//! Auto-paste requires Accessibility permission for the synthetic key
//! events. See `crate::focus::macos_ax::ensure_trusted_with_prompt`.
//!
//! The synthetic Cmd+V must run on the main thread: enigo's macOS backend
//! resolves `Key::Unicode` through HIToolbox's Text Services Manager
//! (`TSMGetInputSourceProperty`, to account for the active keyboard
//! layout), and that call asserts it is on the main GCD queue — calling it
//! from a background thread crashes the process with SIGTRAP
//! (`dispatch_assert_queue_fail`). The picker already marshals its AppKit
//! work onto main via `dispatch2::run_on_main`; this does the same for the
//! keystroke itself, since it runs after the picker returns control to the
//! background hotkey-listener thread.

use std::thread;
use std::time::Duration;

use arboard::Clipboard;
use enigo::{Direction, Enigo, Key, Keyboard, Settings};

pub fn paste_and_clear(secret: &str, clear_after_ms: u64) -> Result<(), Box<dyn std::error::Error>> {
    let mut clipboard = Clipboard::new()?;
    let previous = clipboard.get_text().ok();

    clipboard.set_text(secret.to_owned())?;

    let paste_result: Result<(), String> = dispatch2::run_on_main(|_mtm| {
        (|| -> Result<(), Box<dyn std::error::Error>> {
            let mut enigo = Enigo::new(&Settings::default())?;
            enigo.key(Key::Meta, Direction::Press)?;
            enigo.key(Key::Unicode('v'), Direction::Click)?;
            enigo.key(Key::Meta, Direction::Release)?;
            Ok(())
        })()
        .map_err(|e| e.to_string())
    });

    thread::sleep(Duration::from_millis(clear_after_ms));

    match previous {
        Some(text) => clipboard.set_text(text)?,
        None => clipboard.clear()?,
    }

    paste_result?;
    Ok(())
}
