//! Opt-in, privacy-preserving usage numbers.
//!
//! Default **off**. Never stores transcript text, audio, or a user/account id.
//! When enabled, only aggregate counts live in the data dir. A remote POST
//! happens only if opted in **and** a telemetry URL is set (tests inject a sink).

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeMap;
use std::path::PathBuf;

const OPT_IN_FILE: &str = "analytics-opt-in";
const STATE_FILE: &str = "analytics-state.json";
const MAX_LATENCIES: usize = 256;
const SCHEMA: u32 = 1;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct State {
    takes: u64,
    takes_ok: u64,
    latencies_ms: Vec<u64>,
    duration_buckets: BTreeMap<String, u64>,
    word_buckets: BTreeMap<String, u64>,
    auth: BTreeMap<String, u64>,
    actions: BTreeMap<String, u64>,
}

#[derive(Debug, Clone)]
pub struct AnalyticsStore {
    dir: PathBuf,
}

impl AnalyticsStore {
    pub fn in_data_dir() -> Self {
        Self {
            dir: crate::lifecycle::data_dir(),
        }
    }

    pub fn at(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    pub fn is_enabled(&self) -> bool {
        match std::fs::read_to_string(self.dir.join(OPT_IN_FILE)) {
            Ok(s) => matches!(s.trim(), "1" | "true" | "yes" | "on"),
            Err(_) => false,
        }
    }

    pub fn set_enabled(&self, on: bool) -> Result<()> {
        std::fs::create_dir_all(&self.dir).context("analytics dir")?;
        std::fs::write(self.dir.join(OPT_IN_FILE), if on { "1\n" } else { "0\n" })
            .context("write analytics opt-in")?;
        Ok(())
    }

    /// Record a finished STT attempt. `transcript` is used only to pick a word
    /// bucket and is never persisted.
    pub fn record_take(
        &self,
        duration_ms: u64,
        stop_to_text_ms: u64,
        auth_mode: &str,
        ok: bool,
        transcript: Option<&str>,
    ) {
        if !self.is_enabled() {
            return;
        }
        let mut state = self.load_state();
        state.takes += 1;
        if ok {
            state.takes_ok += 1;
        }
        state.latencies_ms.push(stop_to_text_ms);
        if state.latencies_ms.len() > MAX_LATENCIES {
            let drop = state.latencies_ms.len() - MAX_LATENCIES;
            state.latencies_ms.drain(0..drop);
        }
        *state
            .duration_buckets
            .entry(duration_bucket(duration_ms).into())
            .or_insert(0) += 1;
        let auth = normalize_auth(auth_mode);
        *state.auth.entry(auth.into()).or_insert(0) += 1;
        if let Some(text) = transcript {
            *state
                .word_buckets
                .entry(word_bucket(count_words(text)).into())
                .or_insert(0) += 1;
        }
        self.save_state(&state);
    }

    pub fn record_action(&self, name: &str) {
        if !self.is_enabled() {
            return;
        }
        let mut state = self.load_state();
        let key = sanitize_action(name);
        *state.actions.entry(key).or_insert(0) += 1;
        self.save_state(&state);
    }

    pub fn snapshot(&self) -> Snapshot {
        let state = self.load_state();
        Snapshot {
            enabled: self.is_enabled(),
            takes: state.takes,
            takes_ok: state.takes_ok,
            median_stop_to_text_ms: median(&state.latencies_ms),
            duration_buckets: state.duration_buckets,
            word_buckets: state.word_buckets,
            auth: state.auth,
            actions: state.actions,
        }
    }

    /// Anonymous JSON a maintainer would collect. Must never include text/audio/ids.
    pub fn export_payload(&self) -> serde_json::Value {
        let snap = self.snapshot();
        json!({
            "schema": SCHEMA,
            "app": crate::APP_SLUG,
            "app_version": env!("CARGO_PKG_VERSION"),
            "enabled": snap.enabled,
            "takes": snap.takes,
            "takes_ok": snap.takes_ok,
            "median_stop_to_text_ms": snap.median_stop_to_text_ms,
            "duration_buckets": snap.duration_buckets,
            "word_buckets": snap.word_buckets,
            "auth": snap.auth,
            "actions": snap.actions,
        })
    }

    pub fn summary_line(&self) -> String {
        snapshot_summary(&self.snapshot())
    }

    fn load_state(&self) -> State {
        let path = self.dir.join(STATE_FILE);
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    fn save_state(&self, state: &State) {
        if let Err(e) = std::fs::create_dir_all(&self.dir) {
            tracing::warn!("analytics mkdir: {e}");
            return;
        }
        match serde_json::to_string(state) {
            Ok(s) => {
                if let Err(e) = std::fs::write(self.dir.join(STATE_FILE), s) {
                    tracing::warn!("analytics write: {e}");
                }
            }
            Err(e) => tracing::warn!("analytics serialize: {e}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub enabled: bool,
    pub takes: u64,
    pub takes_ok: u64,
    pub median_stop_to_text_ms: Option<u64>,
    pub duration_buckets: BTreeMap<String, u64>,
    pub word_buckets: BTreeMap<String, u64>,
    pub auth: BTreeMap<String, u64>,
    pub actions: BTreeMap<String, u64>,
}

pub fn snapshot_summary(snap: &Snapshot) -> String {
    if !snap.enabled {
        return "Off — nothing is recorded.".into();
    }
    if snap.takes == 0 {
        return "On — no takes counted yet.".into();
    }
    let med = snap
        .median_stop_to_text_ms
        .map(|ms| format!("{:.1}s median", ms as f64 / 1000.0))
        .unwrap_or_else(|| "no latency yet".into());
    let oauth = snap.auth.get("oauth").copied().unwrap_or(0);
    let key = snap.auth.get("api_key").copied().unwrap_or(0)
        + snap.auth.get("api_key_env").copied().unwrap_or(0);
    format!("{} takes · {med} · sign-in {oauth} / key {key}", snap.takes)
}

pub fn duration_bucket(ms: u64) -> &'static str {
    match ms {
        0..=4999 => "<5s",
        5000..=19999 => "5-20s",
        20000..=59999 => "20-60s",
        _ => ">60s",
    }
}

pub fn word_bucket(words: usize) -> &'static str {
    match words {
        0..=19 => "<20",
        20..=79 => "20-80",
        _ => "80+",
    }
}

pub fn count_words(text: &str) -> usize {
    text.split_whitespace().count()
}

fn normalize_auth(mode: &str) -> &'static str {
    match mode {
        "oauth" => "oauth",
        "api_key_env" => "api_key_env",
        "api_key" => "api_key",
        _ => "other",
    }
}

fn sanitize_action(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .take(32)
        .collect()
}

fn median(xs: &[u64]) -> Option<u64> {
    if xs.is_empty() {
        return None;
    }
    let mut v = xs.to_vec();
    v.sort_unstable();
    Some(v[v.len() / 2])
}

/// Remote submit gate: never send unless opted in.
pub fn maybe_submit<S: TelemetrySink>(store: &AnalyticsStore, sink: &S) -> SubmitOutcome {
    if !store.is_enabled() {
        return SubmitOutcome::SkippedDisabled;
    }
    match sink.send(&store.export_payload()) {
        Ok(()) => SubmitOutcome::Sent,
        Err(e) => SubmitOutcome::Failed(e.to_string()),
    }
}

pub trait TelemetrySink {
    fn send(&self, payload: &serde_json::Value) -> Result<()>;
}

/// HTTP sink — only constructed when a URL is configured.
pub struct HttpTelemetrySink {
    url: String,
}

impl HttpTelemetrySink {
    pub fn from_env() -> Option<Self> {
        let url = std::env::var("COSMIC_SCRIBE_TELEMETRY_URL")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty() && s.starts_with("https://"))?;
        Some(Self { url })
    }

    pub fn url(&self) -> &str {
        &self.url
    }
}

impl TelemetrySink for HttpTelemetrySink {
    fn send(&self, payload: &serde_json::Value) -> Result<()> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()?;
        let resp = client.post(&self.url).json(payload).send()?;
        if !resp.status().is_success() {
            anyhow::bail!("telemetry HTTP {}", resp.status());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubmitOutcome {
    SkippedDisabled,
    Sent,
    Failed(String),
}

pub fn payload_is_anonymous(payload: &serde_json::Value) -> bool {
    let s = payload.to_string().to_ascii_lowercase();
    // Auth *mode* labels (oauth / api_key) are fine. Secrets and content are not.
    let banned = [
        "transcript",
        "user_id",
        "userid",
        "email",
        "oauth_token",
        "bearer ",
        "sk-or-",
        "sk-xai",
    ];
    let no_secrets = !banned.iter().any(|b| s.contains(b));
    let no_audio_blob = !s.contains("\"audio\"");
    no_secrets
        && no_audio_blob
        && payload.get("schema").is_some()
        && payload.get("takes").is_some()
        && payload.get("text").is_none()
}

/// Default store (data dir). Used by the daemon on the real path.
pub fn default_store() -> AnalyticsStore {
    AnalyticsStore::in_data_dir()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Mutex;

    fn tmp_store() -> AnalyticsStore {
        static N: AtomicU64 = AtomicU64::new(0);
        let n = N.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("cs-analytics-{}-{}", std::process::id(), n));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        AnalyticsStore::at(dir)
    }

    #[test]
    fn default_is_off_and_record_is_noop() {
        let store = tmp_store();
        assert!(!store.is_enabled());
        store.record_take(8000, 400, "oauth", true, Some("hello world"));
        store.record_action("cancel");
        let snap = store.snapshot();
        assert_eq!(snap.takes, 0);
        assert!(snap.actions.is_empty());
        assert!(!snap.enabled);
    }

    #[test]
    fn opt_in_persists_and_records_aggregates_only() {
        let store = tmp_store();
        store.set_enabled(true).unwrap();
        assert!(store.is_enabled());
        store.record_take(
            12_000,
            640,
            "oauth",
            true,
            Some("xyzzy-unique-secret-phrase that must never be stored"),
        );
        store.record_take(3_000, 300, "api_key", false, None);
        store.record_action("cancel");

        let snap = store.snapshot();
        assert_eq!(snap.takes, 2);
        assert_eq!(snap.takes_ok, 1);
        assert_eq!(snap.median_stop_to_text_ms, Some(640));
        assert_eq!(snap.duration_buckets.get("5-20s").copied(), Some(1));
        assert_eq!(snap.duration_buckets.get("<5s").copied(), Some(1));
        assert_eq!(snap.auth.get("oauth").copied(), Some(1));
        assert_eq!(snap.actions.get("cancel").copied(), Some(1));

        let payload = store.export_payload();
        let dumped = payload.to_string();
        assert!(
            !dumped.contains("xyzzy-unique-secret-phrase"),
            "transcript leaked: {dumped}"
        );
        assert!(
            payload_is_anonymous(&payload),
            "payload not anonymous: {dumped}"
        );
        assert_eq!(payload["takes"], 2);
        assert_eq!(payload["app"], crate::APP_SLUG);

        store.set_enabled(false).unwrap();
        assert!(!store.is_enabled());
        store.record_take(9_000, 100, "oauth", true, Some("another secret"));
        assert_eq!(store.snapshot().takes, 2, "disabled must not add takes");
    }

    #[test]
    fn submit_skipped_when_disabled() {
        let store = tmp_store();
        struct Probe(Mutex<u32>);
        impl TelemetrySink for Probe {
            fn send(&self, _: &serde_json::Value) -> Result<()> {
                *self.0.lock().unwrap() += 1;
                Ok(())
            }
        }
        let sink = Probe(Mutex::new(0));
        assert_eq!(maybe_submit(&store, &sink), SubmitOutcome::SkippedDisabled);
        assert_eq!(*sink.0.lock().unwrap(), 0);
        store.set_enabled(true).unwrap();
        assert_eq!(maybe_submit(&store, &sink), SubmitOutcome::Sent);
        assert_eq!(*sink.0.lock().unwrap(), 1);
    }

    #[test]
    fn http_sink_requires_https_env() {
        // Safety: test process only; we restore.
        let prev = std::env::var("COSMIC_SCRIBE_TELEMETRY_URL").ok();
        std::env::remove_var("COSMIC_SCRIBE_TELEMETRY_URL");
        assert!(HttpTelemetrySink::from_env().is_none());
        std::env::set_var("COSMIC_SCRIBE_TELEMETRY_URL", "http://insecure.example/t");
        assert!(HttpTelemetrySink::from_env().is_none());
        std::env::set_var(
            "COSMIC_SCRIBE_TELEMETRY_URL",
            "https://telemetry.example/v1",
        );
        let sink = HttpTelemetrySink::from_env().expect("https url");
        assert_eq!(sink.url(), "https://telemetry.example/v1");
        match prev {
            Some(v) => std::env::set_var("COSMIC_SCRIBE_TELEMETRY_URL", v),
            None => std::env::remove_var("COSMIC_SCRIBE_TELEMETRY_URL"),
        }
    }

    #[test]
    fn buckets_and_summary() {
        assert_eq!(duration_bucket(4999), "<5s");
        assert_eq!(duration_bucket(5000), "5-20s");
        assert_eq!(word_bucket(19), "<20");
        assert_eq!(word_bucket(20), "20-80");
        let off = Snapshot {
            enabled: false,
            takes: 0,
            takes_ok: 0,
            median_stop_to_text_ms: None,
            duration_buckets: BTreeMap::new(),
            word_buckets: BTreeMap::new(),
            auth: BTreeMap::new(),
            actions: BTreeMap::new(),
        };
        assert!(snapshot_summary(&off).to_ascii_lowercase().contains("off"));
    }
}
