use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DaemonRequest {
    Status,
    Stop,
    ShowTui,
    PasteSelected { entry_id: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DaemonResponse {
    Ok,
    Error { message: String },
    Status { running: bool, version: String },
}

/// Sends a request to the daemon over its Unix socket and blocks for the
/// response. Intended for callers (like comboauth-tray) that don't run an
/// async runtime themselves — call this from a background thread.
pub fn send_request(request: &DaemonRequest) -> Result<DaemonResponse, Box<dyn std::error::Error>> {
    use std::io::{Read, Write};
    use std::os::unix::net::UnixStream;

    let mut stream = UnixStream::connect(socket_path())?;
    stream.write_all(&serde_json::to_vec(request)?)?;
    stream.shutdown(std::net::Shutdown::Write)?;

    let mut buf = Vec::new();
    stream.read_to_end(&mut buf)?;
    Ok(serde_json::from_slice(&buf)?)
}

pub fn socket_path() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        let tmp = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_string());
        PathBuf::from(tmp).join("comboauth.sock")
    }
    #[cfg(target_os = "linux")]
    {
        let runtime = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string());
        PathBuf::from(runtime).join("comboauth").join("daemon.sock")
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        PathBuf::from("/tmp/comboauth.sock")
    }
}
