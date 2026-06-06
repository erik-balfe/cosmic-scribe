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

pub fn init() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();
}
