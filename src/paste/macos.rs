//! macOS paste: set clipboard, synthesize Cmd+V, then clear the clipboard.
//!
//! Auto-paste requires Accessibility permission for the synthetic key
//! events. See `crate::focus::macos_ax::ensure_trusted_with_prompt`.

use std::thread;
use std::time::Duration;

use arboard::Clipboard;
use enigo::{Direction, Enigo, Key, Keyboard, Settings};

pub fn paste_and_clear(secret: &str, clear_after_ms: u64) -> Result<(), Box<dyn std::error::Error>> {
    let mut clipboard = Clipboard::new()?;
    let previous = clipboard.get_text().ok();

    clipboard.set_text(secret.to_owned())?;

    let mut enigo = Enigo::new(&Settings::default())?;
    enigo.key(Key::Meta, Direction::Press)?;
    enigo.key(Key::Unicode('v'), Direction::Click)?;
    enigo.key(Key::Meta, Direction::Release)?;

    thread::sleep(Duration::from_millis(clear_after_ms));

    match previous {
        Some(text) => clipboard.set_text(text)?,
        None => clipboard.clear()?,
    }

    Ok(())
}
