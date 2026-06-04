// ── IO traits ─────────────────────────────────────────────────
// Every external dependency is behind a trait.
// Production: arecord, xAI REST STT, wl-copy + wtype paste.
// Tests: mocks returning canned data.

use anyhow::Result;
use serde::{Deserialize, Serialize};

pub struct AudioData {
    pub bytes: Vec<u8>,
    pub sample_rate: u32,
    pub channels: u16,
    pub duration_ms: u64,
}

#[async_trait::async_trait]
pub trait AudioCapture: Send {
    async fn start(&mut self) -> Result<()>;
    async fn stop(&mut self) -> Result<AudioData>;
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
}

pub trait TrayController: Send {
    fn set_state(&self, state: &str);
}
