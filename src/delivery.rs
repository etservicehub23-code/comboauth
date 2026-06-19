#![allow(dead_code)]

use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Duration;

use crate::vault::SecretMaterial;

pub const CLIPBOARD_TIMEOUT_SECS: u64 = 10;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DeliveryError {
    #[error("no clipboard tool available (wl-copy or xclip required)")]
    NoTool,
    #[error("clipboard write failed: {0}")]
    Command(String),
}

pub trait SecretSink {
    fn deliver(&self, secret: &SecretMaterial) -> Result<(), DeliveryError>;
}

/// Delivers secrets to the system clipboard.
///
/// Uses pbcopy on macOS; tries wl-copy (Wayland) then xclip (X11) on Linux.
/// The caller is responsible for scheduling clipboard clearing via
/// `schedule_clipboard_clear`.
pub struct ClipboardSink;

impl SecretSink for ClipboardSink {
    fn deliver(&self, secret: &SecretMaterial) -> Result<(), DeliveryError> {
        let bytes = secret.expose_bytes();
        #[cfg(target_os = "macos")]
        {
            return pipe_to("pbcopy", &[], bytes);
        }
        #[cfg(not(target_os = "macos"))]
        {
            if pipe_to("wl-copy", &[], bytes).is_ok() {
                return Ok(());
            }
            pipe_to("xclip", &["-selection", "clipboard"], bytes)
        }
    }
}

/// Writes the secret bytes to stdout followed by a newline.
///
/// Intended for git askpass / credential helper integration.
pub struct StdoutSink;

impl SecretSink for StdoutSink {
    fn deliver(&self, secret: &SecretMaterial) -> Result<(), DeliveryError> {
        use std::io::Write;
        let mut out = std::io::stdout();
        out.write_all(secret.expose_bytes())
            .and_then(|_| out.write_all(b"\n"))
            .map_err(|e| DeliveryError::Command(e.to_string()))
    }
}

/// No-op sink for tests and platforms without clipboard support.
pub struct NullSink;

impl SecretSink for NullSink {
    fn deliver(&self, _secret: &SecretMaterial) -> Result<(), DeliveryError> {
        Ok(())
    }
}

/// Spawn a background thread that clears the clipboard after `delay_secs`.
pub fn schedule_clipboard_clear(delay_secs: u64) {
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(delay_secs));
        #[cfg(target_os = "macos")]
        {
            let _ = pipe_to("pbcopy", &[], b"");
            return;
        }
        #[cfg(not(target_os = "macos"))]
        {
            // wl-copy --clear is the canonical Wayland way
            if Command::new("wl-copy").arg("--clear").status().map(|s| s.success()).unwrap_or(false) {
                return;
            }
            // X11 fallback: overwrite with empty string
            let _ = pipe_to("xclip", &["-selection", "clipboard"], b"");
        }
    });
}

fn pipe_to(program: &str, args: &[&str], input: &[u8]) -> Result<(), DeliveryError> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|_| DeliveryError::NoTool)?;
    if let Some(stdin) = child.stdin.as_mut() {
        stdin
            .write_all(input)
            .map_err(|e| DeliveryError::Command(e.to_string()))?;
    }
    let status = child
        .wait()
        .map_err(|e| DeliveryError::Command(e.to_string()))?;
    if status.success() {
        Ok(())
    } else {
        Err(DeliveryError::Command(format!("{program} exited with failure")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_sink_always_succeeds() {
        let secret = SecretMaterial::new(b"test".to_vec());
        assert!(NullSink.deliver(&secret).is_ok());
    }
}
