// ── Unix socket IPC ───────────────────────────────────────────
// Enables `cosmic-scribe --trigger` / `--cancel` to talk to running daemon.
// Socket path: $XDG_RUNTIME_DIR/cosmic-scribe.sock
//
// Wire protocol: one line per connection, commands TOGGLE | CANCEL.

use anyhow::Context;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::mpsc;

pub fn socket_path() -> std::path::PathBuf {
    std::env::var("XDG_RUNTIME_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("/tmp"))
        .join(format!("{}.sock", crate::APP_SLUG))
}

/// Start listening for CLI/desktop shortcut connections.
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
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    match reader.read_line(&mut line).await {
        Ok(0) => {}
        Ok(_) => {
            let cmd = line.trim();
            match cmd {
                "TOGGLE" => {
                    tracing::info!("IPC: received toggle");
                    tx.send(crate::state::Event::Toggle).ok();
                }
                "CANCEL" => {
                    tracing::info!("IPC: received cancel");
                    tx.send(crate::state::Event::Cancel).ok();
                }
                other if !other.is_empty() => {
                    tracing::warn!("IPC: unknown command {other:?}");
                }
                _ => {}
            }
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

/// Connect to running daemon and send a one-line command (`TOGGLE` / `CANCEL`).
pub async fn send_command(cmd: &str) -> anyhow::Result<()> {
    let line = format!("{cmd}\n");
    let paths = [socket_path(), legacy_socket_path()];
    let mut last_err = None;
    for path in paths {
        match UnixStream::connect(&path).await {
            Ok(mut stream) => {
                stream.write_all(line.as_bytes()).await?;
                stream.flush().await?;
                return Ok(());
            }
            Err(e) => last_err = Some((path, e)),
        }
    }
    let (path, e) = last_err.expect("at least one socket path");
    Err(e).with_context(|| format!("failed to connect to IPC socket at {}", path.display()))
}

/// Connect to running daemon and send toggle command.
pub async fn send_toggle() -> anyhow::Result<()> {
    send_command("TOGGLE").await
}

/// Connect to running daemon and send cancel (abort recording / STT).
pub async fn send_cancel() -> anyhow::Result<()> {
    send_command("CANCEL").await
}
