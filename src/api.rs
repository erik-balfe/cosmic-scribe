//! Typed UI backend shared by the HTTP server (`web.rs`) and native GUI (`gui-native`).

use crate::traits::KeyringStore;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

static MODELS_CACHE: OnceLock<String> = OnceLock::new();

#[derive(Debug, Clone, Serialize)]
pub struct HistoryEntry {
    pub file: String,
    pub ts: String,
    pub duration: String,
    pub text: Option<String>,
    pub has_text: bool,
    pub has_stt: bool,
    #[serde(skip_serializing_if = "versions_empty")]
    pub versions: Vec<RecordingVersion>,
}

fn versions_empty(v: &[RecordingVersion]) -> bool {
    v.is_empty()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingVersion {
    #[serde(rename = "type")]
    pub version_type: String,
    pub text: String,
    #[serde(default)]
    pub timestamp: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecordingDetail {
    pub file: String,
    pub ts: String,
    pub duration: String,
    pub lang: String,
    pub text: String,
    pub has_text: bool,
    pub has_stt: bool,
    pub stt: Option<serde_json::Value>,
    pub versions: Vec<RecordingVersion>,
    pub waveform: Vec<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TranscribeResult {
    pub ok: bool,
    pub text: String,
    pub has_stt: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AppConfig {
    pub lang: String,
    pub output_mode: String,
    pub history_time_mode: String,
    /// Full STT URL (xAI REST dialect). Default: `https://api.x.ai/v1/stt`.
    pub stt_endpoint: String,
    pub has_key: bool,
    /// `oauth` | `api_key` | `api_key_env` | `none`
    pub auth_mode: String,
    pub has_correction_key: bool,
    pub correction_model: String,
}

#[derive(Debug, Clone, Default)]
pub struct ConfigUpdate {
    pub lang: Option<String>,
    pub output_mode: Option<String>,
    pub history_time_mode: Option<String>,
    pub stt_endpoint: Option<String>,
    pub key: Option<String>,
    pub correction_key: Option<String>,
    pub correction_model: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelEntry {
    pub id: String,
    pub name: String,
    pub pricing: String,
    pub rec: bool,
}

#[derive(Debug)]
pub enum ApiError {
    NotFound(String),
    BadRequest(String),
    Internal(String),
}

impl ApiError {
    pub fn message(&self) -> String {
        match self {
            ApiError::NotFound(m) | ApiError::BadRequest(m) | ApiError::Internal(m) => m.clone(),
        }
    }
}

pub fn recordings_dir() -> PathBuf {
    crate::lifecycle::recordings_dir()
}

pub fn prune_junk_on_ui_start() {
    let dir = recordings_dir();
    let n = crate::recording::prune_junk_recordings(&dir);
    if n > 0 {
        tracing::info!("pruned {n} junk recording(s) from {}", dir.display());
    }
}

pub fn ts_to_human(parts: &[&str]) -> String {
    if parts.len() < 2 {
        return parts.join(" ");
    }
    let date = parts[0].to_string();
    let time = parts[1].replace('-', ":");
    format!("{date} {time}")
}

fn duration_ms_for_recording(id: &str, raw_path: &Path) -> u64 {
    if let Ok(meta) = std::fs::metadata(raw_path) {
        let from_pcm =
            (meta.len() as f64 / crate::audio_validation::PCM_BYTES_PER_MS).round() as u64;
        if from_pcm > 0 {
            return from_pcm;
        }
    }
    id.rsplit('_')
        .next()
        .and_then(|s| s.strip_suffix("ms"))
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

fn duration_label_for_recording(id: &str, raw_path: &Path) -> String {
    crate::audio_validation::format_duration_ms(duration_ms_for_recording(id, raw_path))
}

pub fn list_history(offset: usize, limit: usize) -> Vec<HistoryEntry> {
    let mut entries = Vec::new();
    let dir = recordings_dir();
    let Ok(read_dir) = std::fs::read_dir(&dir) else {
        return entries;
    };

    let mut raws: Vec<_> = read_dir
        .filter_map(|e| e.ok())
        .filter(|e| {
            let p = e.path();
            p.extension().map(|x| x == "raw").unwrap_or(false)
                && !crate::recording::is_junk_recording(&p)
        })
        .collect();
    raws.sort_by_key(|e| {
        std::fs::metadata(e.path())
            .ok()
            .and_then(|m| m.modified().ok())
    });
    raws.reverse();

    for entry in raws.iter().skip(offset).take(limit) {
        let path = entry.path();
        let file = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let txt = path.with_extension("txt");
        let text = std::fs::read_to_string(&txt).ok();
        let meta = path.with_extension("json");
        let versions = read_versions(&meta);
        let has_text = text.as_ref().is_some_and(|t| !t.trim().is_empty());
        let has_stt = path.with_extension("stt.json").is_file();
        let parts: Vec<&str> = file.split('_').collect();
        entries.push(HistoryEntry {
            file: file.clone(),
            ts: ts_to_human(&parts),
            duration: duration_label_for_recording(&file, &path),
            text,
            has_text,
            has_stt,
            versions,
        });
    }
    entries
}

fn read_versions(meta_path: &Path) -> Vec<RecordingVersion> {
    let Ok(meta_json) = std::fs::read_to_string(meta_path) else {
        return vec![];
    };
    let Ok(meta) = serde_json::from_str::<serde_json::Value>(&meta_json) else {
        return vec![];
    };
    meta.get("versions")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default()
}

pub fn waveform_data(path: &Path) -> Vec<f64> {
    let Ok(raw) = std::fs::read(path) else {
        return vec![];
    };
    if raw.len() < 4 {
        return vec![];
    }
    let samples: Vec<i16> = raw
        .chunks_exact(2)
        .map(|c| i16::from_le_bytes([c[0], c[1]]))
        .collect();
    let bars = 200usize;
    let step = (samples.len() / bars).max(1);
    (0..bars)
        .map(|i| {
            let start = (i * step).min(samples.len());
            let end = ((i + 1) * step).min(samples.len());
            let slice = &samples[start..end];
            let max = slice.iter().map(|s| (*s as i32).abs()).max().unwrap_or(0) as f64;
            (max / 32768.0).min(1.0)
        })
        .collect()
}

/// Reject path escape / weird IDs before joining under `recordings_dir()`.
///
/// Recording basenames look like `2026-06-01_15-13-17_25414ms` (safe charset).
pub fn validate_recording_id(id: &str) -> Result<&str, ApiError> {
    let id = id.trim();
    if id.is_empty() {
        return Err(ApiError::BadRequest("empty recording id".into()));
    }
    // Must be a single path segment (no absolute, no parent, no separators).
    if id.contains('/') || id.contains('\\') || id.contains('\0') || id.contains("..") {
        return Err(ApiError::BadRequest("invalid recording id".into()));
    }
    let as_path = Path::new(id);
    if as_path.components().count() != 1 {
        return Err(ApiError::BadRequest("invalid recording id".into()));
    }
    match as_path.file_name().and_then(|s| s.to_str()) {
        Some(name) if name == id => {}
        _ => return Err(ApiError::BadRequest("invalid recording id".into())),
    }
    // Tight charset: timestamp-style basenames we write ourselves.
    if !id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
    {
        return Err(ApiError::BadRequest("invalid recording id".into()));
    }
    Ok(id)
}

fn recording_path(id: &str, ext: &str) -> Result<PathBuf, ApiError> {
    let id = validate_recording_id(id)?;
    Ok(recordings_dir().join(format!("{id}.{ext}")))
}

pub fn get_recording(id: &str) -> Result<RecordingDetail, ApiError> {
    let id = validate_recording_id(id)?;
    let raw_path = recording_path(id, "raw")?;
    if !raw_path.is_file() {
        return Err(ApiError::NotFound("recording not found".into()));
    }
    let txt_path = recording_path(id, "txt")?;
    let meta_path = recording_path(id, "json")?;
    let text = std::fs::read_to_string(&txt_path).unwrap_or_default();
    let has_text = !text.trim().is_empty();
    let versions = read_versions(&meta_path);
    let parts: Vec<&str> = id.split('_').collect();
    let stt_path = recording_path(id, "stt.json")?;
    let stt = std::fs::read_to_string(&stt_path)
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok());

    Ok(RecordingDetail {
        file: id.to_string(),
        ts: ts_to_human(&parts),
        duration: duration_label_for_recording(id, &raw_path),
        lang: crate::keyring::get_language(),
        text,
        has_text,
        has_stt: stt.is_some(),
        stt,
        versions,
        waveform: waveform_data(&raw_path),
    })
}

pub fn read_audio_pcm(id: &str) -> Result<Vec<u8>, ApiError> {
    let raw_path = recording_path(id, "raw")?;
    std::fs::read(&raw_path).map_err(|e| ApiError::NotFound(e.to_string()))
}

pub fn encode_pcm_to_wav(pcm: &[u8]) -> Vec<u8> {
    let data_len = pcm.len() as u32;
    let sample_rate: u32 = 16000;
    let bits_per_sample: u16 = 16;
    let channels: u16 = 1;
    let byte_rate = sample_rate * u32::from(channels) * u32::from(bits_per_sample) / 8;
    let block_align = channels * bits_per_sample / 8;

    let mut wav = Vec::with_capacity(44 + pcm.len());
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_len).to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&channels.to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&byte_rate.to_le_bytes());
    wav.extend_from_slice(&block_align.to_le_bytes());
    wav.extend_from_slice(&bits_per_sample.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());
    wav.extend_from_slice(pcm);
    wav
}

pub fn delete_recording(id: &str) -> Result<(), ApiError> {
    let id = validate_recording_id(id)?;
    let _ = std::fs::remove_file(recording_path(id, "raw")?);
    let _ = std::fs::remove_file(recording_path(id, "txt")?);
    let _ = std::fs::remove_file(recording_path(id, "json")?);
    let _ = std::fs::remove_file(recording_path(id, "stt.json")?);
    Ok(())
}

pub fn save_user_edit(id: &str, new_text: &str, edit_type: &str) -> Result<(), ApiError> {
    let id = validate_recording_id(id)?;
    let meta_path = recording_path(id, "json")?;
    let mut meta: serde_json::Value = std::fs::read_to_string(&meta_path)
        .ok()
        .and_then(|m| serde_json::from_str(&m).ok())
        .unwrap_or(serde_json::json!({}));

    let ts = chrono::Utc::now().to_rfc3339();
    let versions = if let Some(arr) = meta.get_mut("versions").and_then(|v| v.as_array_mut()) {
        arr
    } else {
        meta["versions"] = serde_json::json!([]);
        meta["versions"].as_array_mut().unwrap()
    };

    versions.push(serde_json::json!({
        "type": edit_type,
        "text": new_text,
        "timestamp": ts,
    }));

    std::fs::write(&meta_path, serde_json::to_string(&meta).unwrap_or_default())
        .map_err(|e| ApiError::Internal(e.to_string()))
}

pub fn transcribe_recording(id: &str) -> Result<TranscribeResult, ApiError> {
    use crate::app::save_stt_artifacts;
    use crate::keyring::ConfigFileKeyring;
    use crate::stt::XaiSttClient;
    use crate::traits::{AudioData, SttClient};
    use std::sync::Arc;

    let id = validate_recording_id(id)?;
    let raw_path = recording_path(id, "raw")?;
    if !raw_path.is_file() {
        return Err(ApiError::NotFound("recording not found".into()));
    }

    let bytes = std::fs::read(&raw_path).map_err(|e| ApiError::Internal(e.to_string()))?;

    // Local presence only — avoid blocking the GUI/event path on OAuth refresh.
    if !crate::keyring::has_any_speech_credentials() {
        return Err(ApiError::BadRequest(
            "No speech credentials — add an API key in Settings or run cosmic-scribe --login"
                .into(),
        ));
    }

    let audio = AudioData::pcm(bytes, 16000, 1, duration_ms_for_recording(id, &raw_path));
    let stt: Arc<dyn SttClient> = Arc::new(XaiSttClient::new(Arc::new(ConfigFileKeyring)));

    // Never nest Tokio runtimes (native GUI calls this from an async task).
    // Own a short-lived runtime on a dedicated thread so both sync HTTP handlers
    // and in-runtime callers are safe.
    let join = std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| e.to_string())?;
        rt.block_on(stt.transcribe(&audio))
            .map_err(|e| e.to_string())
    });
    let result = join
        .join()
        .map_err(|_| ApiError::Internal("transcription thread panicked".into()))?
        .map_err(ApiError::Internal)?;

    save_stt_artifacts(&raw_path, &result);
    Ok(TranscribeResult {
        ok: true,
        text: result.text,
        has_stt: !result.words.is_empty(),
    })
}

pub fn correct_recording(
    id: &str,
    text: &str,
    marked: &[String],
    kept: &[String],
) -> Result<String, ApiError> {
    let id = validate_recording_id(id)?;
    let correction_key = crate::keyring::get_correction_key();
    if correction_key.is_empty() {
        return Err(ApiError::BadRequest(
            "OpenRouter API key not configured. Add it in Settings.".into(),
        ));
    }

    let prompt = format!(
        "You are correcting a speech-to-text transcription from Cosmic Scribe.\n\
         The user spoke into a microphone, and an STT model transcribed it.\n\
         The user has marked some words/phrases:\n\
         - RED (must change): {}\n\
         - GREEN (must keep): {}\n\
         - UNMARKED (can change if needed): everything else\n\n\
         Full transcript:\n{text}\n\n\
         Rules:\n\
         1. Words marked RED are likely wrong — replace with phonetically similar\n\
            words that fit the context better.\n\
         2. Words marked GREEN must NOT change.\n\
         3. Unmarked words may be adjusted slightly for grammar/flow.\n\
         4. Maintain the overall meaning and style.\n\n\
         Return ONLY the corrected full transcript. No explanation.",
        if marked.is_empty() {
            "none".into()
        } else {
            marked.join(", ")
        },
        if kept.is_empty() {
            "none".into()
        } else {
            kept.join(", ")
        },
    );

    let model = crate::keyring::get_correction_model();
    let model = if model.is_empty() {
        "deepseek/deepseek-chat".to_string()
    } else {
        model
    };
    let prompt_owned = prompt.clone();
    let key_owned = correction_key.clone();
    let model_for_thread = model.clone();
    let handle = std::thread::spawn(move || -> Result<String, String> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| e.to_string())?;
        let resp = client
            .post("https://openrouter.ai/api/v1/chat/completions")
            .header("Authorization", format!("Bearer {key_owned}"))
            .header(
                "HTTP-Referer",
                "https://github.com/erik-balfe/cosmic-scribe",
            )
            .header("X-Title", "Cosmic Scribe")
            .json(&serde_json::json!({
                "model": model_for_thread,
                "messages": [{"role": "user", "content": prompt_owned}],
                "temperature": 0.3,
            }))
            .send()
            .map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body: serde_json::Value = resp.json().unwrap_or(serde_json::json!({}));
            let msg = body["error"]["message"].as_str().unwrap_or("unknown");
            let hint = if msg.to_lowercase().contains("authentication")
                || msg.to_lowercase().contains("not found")
            {
                " — check that your key is an OpenRouter key (starts with sk-or-v1-) and is active."
            } else {
                ""
            };
            return Err(format!("OpenRouter {status}: {msg}{hint}"));
        }

        let json: serde_json::Value = resp.json().map_err(|e| e.to_string())?;
        json["choices"][0]["message"]["content"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| format!("unexpected response: {json}"))
    });
    let result = handle
        .join()
        .unwrap_or_else(|_| Err("correction thread panicked".into()));

    match result {
        Ok(corrected) => {
            let meta_path = recording_path(id, "json")?;
            let mut meta: serde_json::Value = std::fs::read_to_string(&meta_path)
                .ok()
                .and_then(|m| serde_json::from_str(&m).ok())
                .unwrap_or(serde_json::json!({}));
            if meta.get("versions").and_then(|v| v.as_array()).is_none() {
                meta["versions"] = serde_json::json!([]);
            }
            let versions = meta["versions"].as_array_mut().unwrap();
            versions.push(serde_json::json!({
                "type": "llm_correction",
                "text": corrected,
                "marked": marked,
                "model": model,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }));
            let _ = std::fs::write(&meta_path, serde_json::to_string(&meta).unwrap_or_default());
            Ok(corrected)
        }
        Err(e) => Err(ApiError::Internal(e)),
    }
}

pub fn get_config() -> AppConfig {
    // Local-only presence — never OAuth network refresh just to paint Settings.
    let has_key = crate::keyring::has_any_speech_credentials();
    AppConfig {
        lang: crate::keyring::get_language(),
        output_mode: crate::keyring::get_output_mode(),
        history_time_mode: crate::keyring::get_history_time_mode(),
        stt_endpoint: crate::keyring::get_stt_endpoint(),
        has_key,
        auth_mode: crate::xai_oauth::auth_status_label().to_string(),
        has_correction_key: !crate::keyring::get_correction_key().is_empty(),
        correction_model: crate::keyring::get_correction_model(),
    }
}

/// Whether a pay-per-token API key file is stored (not OAuth, not env-only).
pub fn has_stored_api_key() -> bool {
    crate::keyring::has_stored_api_key()
}

/// Remove the stored API key file. Does not sign out of OAuth or clear env keys.
pub fn clear_stored_api_key() -> Result<(), ApiError> {
    use crate::traits::KeyringStore;
    crate::keyring::ConfigFileKeyring
        .clear()
        .map_err(|e| ApiError::Internal(e.to_string()))
}

pub fn save_config(update: &ConfigUpdate) -> Result<(), ApiError> {
    if let Some(lang) = update.lang.as_deref() {
        crate::keyring::set_language(lang).map_err(|e| ApiError::Internal(e.to_string()))?;
    }
    if let Some(mode) = update.output_mode.as_deref() {
        // Refuse empty — UI defaults must not wipe a prior clipboard choice.
        if !mode.trim().is_empty() {
            crate::keyring::set_output_mode(mode)
                .map_err(|e| ApiError::BadRequest(e.to_string()))?;
        }
    }
    if let Some(mode) = update.history_time_mode.as_deref() {
        crate::keyring::set_history_time_mode(mode)
            .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    }
    if let Some(endpoint) = update.stt_endpoint.as_deref() {
        crate::keyring::set_stt_endpoint(endpoint)
            .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    }
    if let Some(key) = update.key.as_deref() {
        if !key.is_empty() {
            crate::keyring::ConfigFileKeyring
                .set_api_key(key)
                .map_err(|e| ApiError::Internal(e.to_string()))?;
        }
    }
    if let Some(ck) = update.correction_key.as_deref() {
        if !ck.is_empty() {
            crate::keyring::set_correction_key(ck)
                .map_err(|e| ApiError::Internal(e.to_string()))?;
        }
    }
    if let Some(cm) = update.correction_model.as_deref() {
        crate::keyring::set_correction_model(cm).map_err(|e| ApiError::Internal(e.to_string()))?;
    }
    Ok(())
}

pub fn save_config_from_json(body: &str) -> Result<(), ApiError> {
    if body.trim().is_empty() {
        return Err(ApiError::BadRequest("empty request body".into()));
    }
    let v: serde_json::Value = serde_json::from_str(body)
        .map_err(|e| ApiError::BadRequest(format!("invalid JSON: {e}")))?;
    let update = ConfigUpdate {
        lang: v.get("lang").and_then(|l| l.as_str()).map(String::from),
        output_mode: v
            .get("output_mode")
            .and_then(|m| m.as_str())
            .map(String::from),
        history_time_mode: v
            .get("history_time_mode")
            .and_then(|m| m.as_str())
            .map(String::from),
        stt_endpoint: v
            .get("stt_endpoint")
            .and_then(|u| u.as_str())
            .map(String::from),
        key: v.get("key").and_then(|k| k.as_str()).map(String::from),
        correction_key: v
            .get("correction_key")
            .and_then(|k| k.as_str())
            .map(String::from),
        correction_model: v
            .get("correction_model")
            .and_then(|m| m.as_str())
            .map(String::from),
    };
    save_config(&update)
}

pub fn preload_models_cache() {
    let _ = MODELS_CACHE.get_or_init(|| {
        let output = std::process::Command::new("curl")
            .args(["-s", "--max-time", "10", "https://models.dev/api.json"])
            .output();
        match output {
            Ok(o) if o.status.success() => {
                let models = parse_models_json(&o.stdout);
                serde_json::Value::Array(models).to_string()
            }
            _ => serde_json::json!({"error": "Failed to fetch models"}).to_string(),
        }
    });
}

fn parse_models_json(data: &[u8]) -> Vec<serde_json::Value> {
    let json: serde_json::Value = match serde_json::from_slice(data) {
        Ok(v) => v,
        Err(_) => return vec![],
    };
    let providers = match json.as_object() {
        Some(p) => p,
        None => return vec![],
    };
    let mut models = Vec::new();
    let recommended = [
        "deepseek/deepseek-chat-v4",
        "deepseek/deepseek-chat",
        "openai/gpt-4o-mini",
        "x-ai/grok-2",
    ];

    for (pid, pdata) in providers {
        let pname = pdata.get("name").and_then(|n| n.as_str()).unwrap_or(pid);
        if let Some(pmodels) = pdata.get("models").and_then(|m| m.as_object()) {
            for (mid, mdata) in pmodels {
                let name = mdata.get("name").and_then(|n| n.as_str()).unwrap_or(mid);
                if name.len() >= 100 {
                    continue;
                }
                let pricing = mdata
                    .get("pricing")
                    .and_then(|p| p.get("input"))
                    .and_then(|i| i.as_str())
                    .unwrap_or("?");
                let full_id = format!("{pid}/{mid}");
                let rec = recommended.contains(&full_id.as_str());
                models.push(serde_json::json!({
                    "id": full_id,
                    "name": format!("{name} ({pname})"),
                    "pricing": format!("${pricing}/1M"),
                    "rec": rec,
                }));
            }
        }
    }
    models.sort_by(|a, b| match (a["rec"].as_bool(), b["rec"].as_bool()) {
        (Some(true), Some(false)) => std::cmp::Ordering::Less,
        (Some(false), Some(true)) => std::cmp::Ordering::Greater,
        _ => a["id"].as_str().cmp(&b["id"].as_str()),
    });
    models
}

pub fn models_json() -> String {
    preload_models_cache();
    MODELS_CACHE.get().cloned().unwrap_or_else(|| "[]".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn init_tests() {
        crate::lifecycle::init_test_recordings_dir();
    }

    #[test]
    fn test_waveform_i16_min_no_panic() {
        init_tests();
        let samples: Vec<u8> = vec![0x00u8, 0x80u8, 0x00, 0x00];
        let dir = recordings_dir();
        std::fs::create_dir_all(&dir).ok();
        let path = dir.join("test_min.raw");
        std::fs::write(&path, &samples).ok();
        let wf = waveform_data(&path);
        assert_eq!(wf.len(), 200);
        assert!(wf.iter().all(|&v| v >= 0.0));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_save_config_accepts_wtype() {
        init_tests();
        let update = ConfigUpdate {
            lang: Some("en".into()),
            output_mode: Some("wtype".into()),
            correction_model: Some("deepseek/deepseek-chat-v4".into()),
            ..Default::default()
        };
        assert!(save_config(&update).is_ok());
    }

    #[test]
    fn test_save_config_rejects_invalid_output_mode() {
        init_tests();
        let update = ConfigUpdate {
            output_mode: Some("auto".into()),
            ..Default::default()
        };
        let err = save_config(&update).unwrap_err();
        assert!(matches!(err, ApiError::BadRequest(_)));
        assert!(err.message().contains("wtype"));
    }

    #[test]
    fn test_validate_recording_id_rejects_path_escape() {
        assert!(validate_recording_id("../etc/passwd").is_err());
        assert!(validate_recording_id("/tmp/x").is_err());
        assert!(validate_recording_id("a/b").is_err());
        assert!(validate_recording_id("").is_err());
        assert!(validate_recording_id("2026-06-01_15-13-17_25414ms").is_ok());
    }

    #[test]
    fn test_get_recording_works_with_new_format() {
        init_tests();
        let dir = recordings_dir();
        std::fs::create_dir_all(&dir).ok();
        let raw = vec![0u8; 32000];
        let id = "2026-06-01_15-13-17_25414ms";
        std::fs::write(dir.join(format!("{id}.raw")), &raw).ok();
        std::fs::write(dir.join(format!("{id}.txt")), "test transcript").ok();

        let detail = get_recording(id).unwrap();
        assert_eq!(detail.file, id);
        assert_eq!(detail.text, "test transcript");
        assert_eq!(detail.ts, "2026-06-01 15:13:17");

        let _ = std::fs::remove_file(dir.join(format!("{id}.raw")));
        let _ = std::fs::remove_file(dir.join(format!("{id}.txt")));
    }
}
