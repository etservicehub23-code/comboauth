//! comboauth-daemon — background process for global hotkey and paste.
//!
//! Phase 9-B implements the full logic. This stub starts the tokio runtime
//! and binds the IPC socket so the tray can connect to it.

use tokio::net::UnixListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sock = comboauth::ipc::socket_path();
    if let Some(parent) = sock.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // Remove stale socket from a previous run.
    let _ = std::fs::remove_file(&sock);

    let listener = UnixListener::bind(&sock)?;
    eprintln!("comboauth-daemon: listening on {}", sock.display());

    // TODO Phase 9-B: register global hotkey, spawn paste task, accept IPC connections.
    loop {
        let (mut stream, _) = listener.accept().await?;
        tokio::spawn(async move {
            use tokio::io::AsyncReadExt;
            let mut buf = Vec::new();
            let _ = stream.read_to_end(&mut buf).await;
            // TODO: parse DaemonRequest, dispatch, write DaemonResponse
        });
    }
}
