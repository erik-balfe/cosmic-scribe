// ── Structured logging ────────────────────────────────────────
// Uses tracing for structured, async-aware logging.
// Set RUST_LOG=cosmic_scribe=debug for verbose output.
//
// Log events cover the full lifecycle:
//   state_transition, audio_captured, transcript_ready, text_injected,
//   error, validation_failed

use crate::state::AppState;
use tracing::{debug, error, info, warn};

pub struct LogCtx {
    recording_id: String,
}

impl Default for LogCtx {
    fn default() -> Self {
        Self::new()
    }
}

impl LogCtx {
    pub fn new() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| format!("{:x}", d.as_millis()))
            .unwrap_or_else(|_| "unknown".into());
        Self { recording_id: id }
    }

    pub fn state_transition(&self, from: &AppState, to: &AppState) {
        info!(
            recording_id = %self.recording_id,
            from = %from,
            to = %to,
            "state transition",
        );
    }

    pub fn audio_captured(&self, bytes: usize, duration_ms: u64) {
        info!(
            recording_id = %self.recording_id,
            bytes = bytes,
            duration_ms = duration_ms,
            "audio captured",
        );
    }

    pub fn transcription_request(&self) {
        debug!(recording_id = %self.recording_id, "sending to STT API");
    }

    pub fn transcription_received(&self, text: &str) {
        info!(
            recording_id = %self.recording_id,
            text_len = text.len(),
            preview = %text.chars().take(80).collect::<String>(),
            "transcript received",
        );
    }

    pub fn text_injected(&self) {
        info!(recording_id = %self.recording_id, "text injected");
    }

    pub fn validation_error(&self, detail: &str) {
        warn!(recording_id = %self.recording_id, detail = %detail, "audio validation failed");
    }

    pub fn stt_error(&self, detail: &str) {
        error!(recording_id = %self.recording_id, detail = %detail, "STT API error");
    }

    pub fn audio_error(&self, detail: &str) {
        error!(recording_id = %self.recording_id, detail = %detail, "audio capture error");
    }

    pub fn injection_error(&self, detail: &str) {
        error!(recording_id = %self.recording_id, detail = %detail, "text injection error");
    }
}

fn env_filter() -> tracing_subscriber::EnvFilter {
    tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
}

pub fn init() {
    tracing_subscriber::fmt()
        .with_env_filter(env_filter())
        .with_ansi(true)
        .init();
}

#[derive(Clone)]
struct DaemonLogWriter(std::sync::Arc<std::sync::Mutex<std::fs::File>>);

struct DaemonLogWriterGuard(std::sync::Arc<std::sync::Mutex<std::fs::File>>);

impl std::io::Write for DaemonLogWriterGuard {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).flush()
    }
}

impl<'a> tracing_subscriber::fmt::writer::MakeWriter<'a> for DaemonLogWriter {
    type Writer = DaemonLogWriterGuard;

    fn make_writer(&'a self) -> Self::Writer {
        DaemonLogWriterGuard(self.0.clone())
    }
}

/// Daemon mode: append to `~/.local/share/cosmic-scribe/daemon.log` (survives null stderr).
pub fn init_daemon() {
    use std::fs::OpenOptions;
    use std::io::Write;

    let path = crate::lifecycle::daemon_log_path();
    let file = OpenOptions::new().create(true).append(true).open(&path);

    match file {
        Ok(mut f) => {
            let ts = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
            let _ = writeln!(
                f,
                "\n=== daemon start {ts} pid={} ppid={} ===",
                std::process::id(),
                std::env::var("PPID").unwrap_or_else(|_| "?".into())
            );
            let writer = DaemonLogWriter(std::sync::Arc::new(std::sync::Mutex::new(f)));
            tracing_subscriber::fmt()
                .with_env_filter(env_filter())
                .with_ansi(false)
                .with_writer(writer)
                .init();
            info!(log = %path.display(), "daemon logging to file");
        }
        Err(e) => {
            eprintln!("daemon log unavailable ({}): {e}", path.display());
            init();
        }
    }
}
