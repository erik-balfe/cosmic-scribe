// ── Unix socket IPC ───────────────────────────────────────────
// Enables `voice-input --trigger` to talk to running applet.
// Socket path: $XDG_RUNTIME_DIR/voice-input.sock

use tokio::io::AsyncWriteExt;
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::mpsc;

pub fn socket_path() -> std::path::PathBuf {
    std::env::var("XDG_RUNTIME_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("/tmp"))
        .join("voice-input.sock")
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

/// Connect to running applet and send toggle command.
pub async fn send_toggle() -> anyhow::Result<()> {
    let path = socket_path();
    let mut stream = UnixStream::connect(&path).await?;
    stream.write_all(b"TOGGLE\n").await?;
    stream.flush().await?;
    Ok(())
}
