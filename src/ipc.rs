// ── Unix socket IPC ───────────────────────────────────────────
// Enables `cosmic-scribe --trigger` to talk to running daemon.
// Socket path: $XDG_RUNTIME_DIR/cosmic-scribe.sock

use anyhow::Context;
use tokio::io::AsyncWriteExt;
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::mpsc;

pub fn socket_path() -> std::path::PathBuf {
    std::env::var("XDG_RUNTIME_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("/tmp"))
        .join(format!("{}.sock", crate::APP_SLUG))
}

/// Start listening for --trigger connections. Sends "TOGGLE\n" messages as events.
pub async fn spawn_listener(tx: mpsc::UnboundedSender<crate::state::Event>) {
    let path = socket_path();
    let _ = std::fs::remove_file(&path);

    let listener = match UnixListener::bind(&path) {
        Ok(l) => l,
        Err(e) => {
            tracing::error!("failed to bind IPC socket at {}: {e}", path.display());
            return;
        }
    };

    tracing::info!("IPC listening on {}", path.display());

    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let tx = tx.clone();
                tokio::spawn(handle_connection(stream, tx));
            }
            Err(e) => {
                tracing::error!("IPC accept error: {e}");
            }
        }
    }
}

async fn handle_connection(stream: UnixStream, tx: mpsc::UnboundedSender<crate::state::Event>) {
    let mut buf = [0u8; 16];
    stream.readable().await.ok();
    match stream.try_read(&mut buf) {
        Ok(n) => {
            let msg = String::from_utf8_lossy(&buf[..n]);
            if msg.trim() == "TOGGLE" {
                tracing::info!("IPC: received toggle");
                tx.send(crate::state::Event::Toggle).ok();
            }
        }
        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
            // No data yet — client might have disconnected
        }
        Err(e) => {
            tracing::warn!("IPC read error: {e}");
        }
    }
}

fn legacy_socket_path() -> std::path::PathBuf {
    std::env::var("XDG_RUNTIME_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("/tmp"))
        .join("voice-input.sock")
}

/// Connect to running daemon and send toggle command.
pub async fn send_toggle() -> anyhow::Result<()> {
    let paths = [socket_path(), legacy_socket_path()];
    let mut last_err = None;
    for path in paths {
        match UnixStream::connect(&path).await {
            Ok(mut stream) => {
                stream.write_all(b"TOGGLE\n").await?;
                stream.flush().await?;
                return Ok(());
            }
            Err(e) => last_err = Some((path, e)),
        }
    }
    let (path, e) = last_err.expect("at least one socket path");
    Err(e).with_context(|| format!("failed to connect to IPC socket at {}", path.display()))
}
