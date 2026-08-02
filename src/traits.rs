// ── IO traits ─────────────────────────────────────────────────
// Every external dependency is behind a trait.
// Production: arecord, xAI REST STT, wl-copy + wtype paste.
// Tests: mocks returning canned data.

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Compressed audio produced **during** capture (progressive Opus) for fast STT upload.
#[derive(Clone, Debug)]
pub struct PreEncodedAudio {
    pub bytes: Vec<u8>,
    pub file_name: String,
    pub mime: String,
    pub codec: String,
}

#[derive(Clone, Debug)]
pub struct AudioData {
    /// Raw PCM s16le (kept for local history / playback).
    pub bytes: Vec<u8>,
    pub sample_rate: u32,
    pub channels: u16,
    pub duration_ms: u64,
    /// If set, STT upload should use this instead of encoding after stop.
    pub pre_encoded: Option<PreEncodedAudio>,
}

impl AudioData {
    pub fn pcm(bytes: Vec<u8>, sample_rate: u32, channels: u16, duration_ms: u64) -> Self {
        Self {
            bytes,
            sample_rate,
            channels,
            duration_ms,
            pre_encoded: None,
        }
    }
}

#[async_trait::async_trait]
pub trait AudioCapture: Send {
    async fn start(&mut self) -> Result<()>;
    async fn stop(&mut self) -> Result<AudioData>;

    /// Optional live capture buffer (diagnostics only; not used to abort takes).
    fn monitor_buffer(&self) -> Option<std::sync::Arc<std::sync::Mutex<Vec<u8>>>> {
        None
    }
}

/// Word-level timing from xAI REST STT (`words[]` in API response).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SttWord {
    pub text: String,
    pub start: f64,
    pub end: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speaker: Option<u32>,
}

/// Full transcription result persisted as `{recording}.stt.json` for karaoke UI later.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SttResult {
    pub schema_version: u32,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_secs: Option<f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub words: Vec<SttWord>,
    /// Unmodified API JSON for forward compatibility.
    pub api_response: serde_json::Value,
}

impl SttResult {
    pub fn from_api_json(api: serde_json::Value) -> anyhow::Result<Self> {
        let text = api
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let language = api
            .get("language")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let duration_secs = api.get("duration").and_then(|v| v.as_f64());
        let words = api
            .get("words")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|w| {
                        Some(SttWord {
                            text: w.get("text")?.as_str()?.to_string(),
                            start: w.get("start")?.as_f64()?,
                            end: w.get("end")?.as_f64()?,
                            speaker: w.get("speaker").and_then(|s| s.as_u64()).map(|n| n as u32),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        Ok(Self {
            schema_version: 1,
            text,
            language,
            duration_secs,
            words,
            api_response: api,
        })
    }
}

#[async_trait::async_trait]
pub trait SttClient: Send + Sync {
    async fn transcribe(&self, audio: &AudioData) -> Result<SttResult>;
}

#[async_trait::async_trait]
pub trait TextInjector: Send {
    /// Inject text with wtype + clipboard (for keyboard shortcut)
    async fn inject(&self, text: &str) -> Result<()>;
    /// Clipboard only (for tray click — no focus-stealing)
    async fn inject_clipboard(&self, text: &str) -> Result<()>;
}

pub trait KeyringStore: Send + Sync {
    fn get_api_key(&self) -> Result<String>;
    fn set_api_key(&self, key: &str) -> Result<()>;
    fn clear(&self) -> Result<()>;

    /// Local-only: may a STT request be attempted? Must not hit the network.
    ///
    /// Production: env key / OAuth store present / key file. Never OAuth refresh.
    fn has_local_credentials(&self) -> bool {
        self.get_api_key().map(|k| !k.is_empty()).unwrap_or(false)
    }
}

pub trait TrayController: Send {
    fn set_state(&self, state: &str);
}
