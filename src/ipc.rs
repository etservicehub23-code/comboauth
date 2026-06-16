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
