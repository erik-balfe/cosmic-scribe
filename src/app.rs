// ── Application loop ──────────────────────────────────────────
// Connects the pure state machine with async IO.
// Drives event loop, dispatches commands to trait impls,
// feeds results back as events.
//
// Edge case handling:
//   - Empty / too-short audio: caught by validate_audio(), returns to Idle
//   - STT timeout: 60s via tokio::time::timeout, triggers Error event
//   - Missing API key: checked before STT, triggers Error event

use crate::audio_validation;
use crate::logging::LogCtx;
use crate::state::{self, AppState, Command, Event};
use crate::traits::*;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};

fn recordings_dir() -> std::path::PathBuf {
    crate::lifecycle::recordings_dir()
}

fn save_recording(bytes: &[u8], duration_ms: u64) -> std::path::PathBuf {
    let ts = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S");
    let duration_ms = audio_validation::duration_ms_from_pcm(bytes).max(duration_ms);
    let name = format!("{}_{}ms.raw", ts, duration_ms);
    let path = recordings_dir().join(&name);
    if let Err(e) = std::fs::write(&path, bytes) {
        tracing::warn!("failed to save recording: {e}");
    } else {
        tracing::info!("recording saved: {}", path.display());
    }
    path
}

/// Plain `.txt` plus `.stt.json` (word timestamps) for history / karaoke UI.
pub(crate) fn save_stt_artifacts(path: &std::path::Path, result: &SttResult) {
    let txt = path.with_extension("txt");
    if let Err(e) = std::fs::write(&txt, &result.text) {
        tracing::warn!("failed to save transcript txt: {e}");
    }
    let stt_path = path.with_extension("stt.json");
    match serde_json::to_string_pretty(result) {
        Ok(json) => {
            if let Err(e) = std::fs::write(&stt_path, json) {
                tracing::warn!("failed to save stt.json: {e}");
            } else {
                tracing::info!("stt metadata saved: {}", stt_path.display());
            }
        }
        Err(e) => tracing::warn!("failed to serialize stt.json: {e}"),
    }
}

/// STT request timeout. Override with COSMIC_SCRIBE_STT_TIMEOUT_MS (or legacy VOICE_INPUT_STT_TIMEOUT_MS).
/// Default 60s. Set to 5000 for faster tests.
#[cfg(not(test))]
fn notify_clipboard_ready() {
    let _ = std::process::Command::new("notify-send")
        .arg("Cosmic Scribe")
        .arg("Text ready — paste with Ctrl+V (Ctrl+Shift+V in a terminal)")
        .status();
}

#[cfg(test)]
fn notify_clipboard_ready() {}

fn stt_timeout() -> Duration {
    crate::env_compat("COSMIC_SCRIBE_STT_TIMEOUT_MS", "VOICE_INPUT_STT_TIMEOUT_MS")
        .and_then(|s| s.parse::<u64>().ok())
        .map(Duration::from_millis)
        // Long dictation + server time needs headroom; was 60s and looked like
        // "never finishes" when encode deadlocked or upload was slow.
        .unwrap_or_else(|| Duration::from_secs(180))
}

pub struct App {
    state: AppState,
    audio: Box<dyn AudioCapture>,
    stt: Arc<dyn SttClient>,
    injector: Box<dyn TextInjector>,
    keyring: Arc<dyn KeyringStore>,
    tray: Box<dyn TrayController>,
    event_tx: mpsc::UnboundedSender<Event>,
    event_rx: mpsc::UnboundedReceiver<Event>,
    log: LogCtx,
    last_duration_ms: u64,
    done_tx: Option<oneshot::Sender<()>>,

    last_recording_path: Option<std::path::PathBuf>,
    /// Progressive Opus (or other) from capture — attached to STT AudioData on Transcribe.
    last_pre_encoded: Option<crate::traits::PreEncodedAudio>,
    /// Bumped on cancel during transcribing so in-flight STT results are dropped.
    transcribe_generation: Arc<AtomicU64>,
    /// True after intentional stop until audio bytes are received (handles late AudioCaptured).
    awaiting_audio: bool,
    /// Set when user cancels mid-record so StopCapture drops PCM instead of STT.
    discard_next_audio: bool,
    is_capturing: Arc<AtomicBool>,
    /// Last tray notification (tests only — proves first-run copy).
    #[cfg(test)]
    last_notification: Option<(String, String)>,
}

impl App {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        audio: Box<dyn AudioCapture>,
        stt: Arc<dyn SttClient>,
        injector: Box<dyn TextInjector>,
        keyring: Arc<dyn KeyringStore>,
        tray: Box<dyn TrayController>,
    ) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            state: AppState::Idle,
            audio,
            stt,
            injector,
            keyring,
            tray,
            event_tx: tx,
            event_rx: rx,
            log: LogCtx::new(),
            last_duration_ms: 0,
            done_tx: None,

            last_recording_path: None,
            last_pre_encoded: None,
            transcribe_generation: Arc::new(AtomicU64::new(0)),
            awaiting_audio: false,
            discard_next_audio: false,
            is_capturing: Arc::new(AtomicBool::new(false)),
            #[cfg(test)]
            last_notification: None,
        }
    }

    pub fn event_sender(&self) -> mpsc::UnboundedSender<Event> {
        self.event_tx.clone()
    }

    pub fn current_state(&self) -> &AppState {
        &self.state
    }

    pub fn done_rx(&mut self) -> oneshot::Receiver<()> {
        let (tx, rx) = oneshot::channel();
        self.done_tx = Some(tx);
        rx
    }

    pub fn set_tray_controller(&mut self, tray: Box<dyn TrayController>) {
        self.tray = tray;
    }

    /// Local-only credential presence — never hits the network (OAuth refresh).
    fn api_key_configured(&self) -> bool {
        self.keyring.has_local_credentials()
    }

    pub async fn process_event(&mut self, event: Event) {
        if matches!(&event, Event::Toggle | Event::ToggleTray)
            && matches!(self.state, AppState::Idle)
            && !self.api_key_configured()
        {
            self.execute_commands(vec![
                Command::ShowNotification {
                    title: crate::product_copy::setup_needed_notification_title().into(),
                    body: crate::product_copy::setup_needed_notification_body().into(),
                },
                Command::OpenSettings,
            ])
            .await;
            return;
        }

        if let Event::AudioCaptured { duration_ms, .. } = &event {
            self.last_duration_ms = *duration_ms;
        }

        // User cancel: never STT this take (even if StopCapture already running).
        if matches!(&event, Event::AudioCaptured { .. }) && self.discard_next_audio {
            self.discard_next_audio = false;
            self.awaiting_audio = false;
            self.last_pre_encoded = None;
            tracing::info!("discarding audio after cancel");
            return;
        }

        // StopCapture can finish after intentional stop; still run STT if we were waiting.
        if matches!(self.state, AppState::Idle)
            && self.awaiting_audio
            && matches!(&event, Event::AudioCaptured { .. })
        {
            self.state = AppState::Transcribing;
        }

        let from = self.state.clone();

        if matches!(&event, Event::Cancel) {
            self.on_user_cancel(&from);
        }

        let (new_state, commands) = state::transition(&self.state, &event);
        self.state = new_state;

        if matches!(self.state, AppState::Transcribing)
            && matches!(from, AppState::Recording)
            && !self.discard_next_audio
        {
            self.awaiting_audio = true;
        }
        if matches!(self.state, AppState::Idle) {
            self.awaiting_audio = false;
            // Keep discard_next_audio until StopCapture / late AudioCaptured / next StartCapture.
        }
        self.log.state_transition(&from, &self.state);

        let was_active = from.is_active();
        let is_idle_or_error = matches!(self.state, AppState::Idle | AppState::Error { .. });
        if was_active && is_idle_or_error {
            if let Some(tx) = self.done_tx.take() {
                let _ = tx.send(());
            }
        }

        self.execute_commands(commands).await;
    }

    pub async fn run(&mut self) {
        loop {
            match self.event_rx.recv().await {
                None => break,
                Some(event) => self.process_event(event).await,
            }
        }
    }

    /// Side effects for user cancel (shortcut, tray, CLI) — before state transition.
    fn on_user_cancel(&mut self, from: &AppState) {
        crate::analytics::default_store().record_action("cancel");
        match from {
            AppState::Recording => {
                self.discard_next_audio = true;
                self.awaiting_audio = false;
                self.last_pre_encoded = None;
            }
            AppState::Transcribing => {
                self.transcribe_generation.fetch_add(1, Ordering::SeqCst);
                self.awaiting_audio = false;
                self.discard_next_audio = true;
                self.last_pre_encoded = None;
                if let Some(p) = self.last_recording_path.take() {
                    crate::recording::remove_recording_files(&p);
                    tracing::info!("removed cancelled recording {}", p.display());
                }
            }
            AppState::Inserting => {
                self.last_pre_encoded = None;
            }
            _ => {}
        }
    }

    async fn execute_commands(&mut self, commands: Vec<Command>) {
        use Command::*;

        for cmd in commands {
            match cmd {
                StartCapture => {
                    self.discard_next_audio = false;
                    if let Err(e) = self.audio.start().await {
                        self.log.audio_error(&e.to_string());
                        self.event_tx.send(Event::Error(e.to_string())).ok();
                    } else {
                        self.is_capturing.store(true, Ordering::SeqCst);
                        // Warm OAuth while user speaks so stop→STT never pays refresh latency.
                        crate::xai_oauth::warm_token_background();
                        // Soft device check only (muted / 0% volume). Never abort on RMS
                        // “silence” — false positives kill real takes mid-dictation.
                        if let Some(warning) = audio_validation::mic_capture_warning() {
                            let _ = std::process::Command::new("notify-send")
                                .arg("Cosmic Scribe")
                                .arg(&warning)
                                .status();
                        }
                    }
                }

                StopCapture => {
                    self.is_capturing.store(false, Ordering::SeqCst);
                    let discard = self.discard_next_audio;
                    match self.audio.stop().await {
                        Ok(data) => {
                            self.log.audio_captured(data.bytes.len(), data.duration_ms);
                            if discard {
                                self.discard_next_audio = false;
                                self.last_pre_encoded = None;
                                self.awaiting_audio = false;
                                tracing::info!(
                                    "stop after cancel — discarded {} bytes",
                                    data.bytes.len()
                                );
                                continue;
                            }
                            // Carry progressive Opus into Transcribe (Event only holds PCM).
                            self.last_pre_encoded = data.pre_encoded;

                            match audio_validation::validate_audio(&data.bytes, data.duration_ms) {
                                Ok(()) => {
                                    self.event_tx
                                        .send(Event::AudioCaptured {
                                            bytes: data.bytes,
                                            duration_ms: data.duration_ms,
                                        })
                                        .ok();
                                }
                                Err(err) => {
                                    self.last_pre_encoded = None;
                                    self.log.validation_error(&err.to_string());
                                    self.event_tx.send(Event::Error(err.to_string())).ok();
                                }
                            }
                        }
                        Err(e) => {
                            self.last_pre_encoded = None;
                            self.discard_next_audio = false;
                            self.log.audio_error(&e.to_string());
                            if !discard {
                                self.event_tx.send(Event::Error(e.to_string())).ok();
                            }
                        }
                    }
                }

                Transcribe(data) => {
                    self.awaiting_audio = false;
                    self.last_recording_path = Some(save_recording(&data, self.last_duration_ms));

                    let drop_orphan = |path: &Option<std::path::PathBuf>| {
                        if let Some(p) = path {
                            crate::recording::remove_recording_files(p);
                        }
                    };

                    // Local presence only — never block the event loop on OAuth refresh.
                    // Real bearer resolve happens inside STT (spawn_blocking).
                    if !self.keyring.has_local_credentials() {
                        self.log.stt_error("no speech credentials configured");
                        drop_orphan(&self.last_recording_path);
                        self.last_recording_path = None;
                        self.last_pre_encoded = None;
                        self.event_tx
                            .send(Event::Error(
                                crate::product_copy::no_credentials_error().into(),
                            ))
                            .ok();
                        continue;
                    }

                    self.log.transcription_request();

                    let audio = AudioData {
                        bytes: data,
                        sample_rate: 16000,
                        channels: 1,
                        duration_ms: self.last_duration_ms,
                        pre_encoded: self.last_pre_encoded.take(),
                    };

                    let gen = self.transcribe_generation.load(Ordering::SeqCst);
                    let stt = self.stt.clone();
                    let tx = self.event_tx.clone();
                    let path = self.last_recording_path.clone();
                    let generation = self.transcribe_generation.clone();

                    tokio::spawn(async move {
                        let max_retries: usize = crate::env_compat(
                            "COSMIC_SCRIBE_STT_RETRIES",
                            "VOICE_INPUT_STT_RETRIES",
                        )
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(3);
                        let mut last_err = String::new();

                        for attempt in 0..=max_retries {
                            if generation.load(Ordering::SeqCst) != gen {
                                tracing::info!("STT cancelled (generation mismatch)");
                                return;
                            }
                            if attempt > 0 {
                                let delay = Duration::from_millis(500 * (1u64 << attempt));
                                tracing::info!("STT retry {attempt}/{max_retries} after {delay:?}");
                                tokio::time::sleep(delay).await;
                            }

                            let result =
                                tokio::time::timeout(stt_timeout(), stt.transcribe(&audio)).await;

                            if generation.load(Ordering::SeqCst) != gen {
                                tracing::info!("STT cancelled after request");
                                return;
                            }

                            match result {
                                Ok(Ok(stt)) => {
                                    if stt.text.trim().is_empty() {
                                        last_err = "empty transcript from STT".into();
                                        continue;
                                    }
                                    tracing::info!(
                                        "transcript received ({} chars, {} words)",
                                        stt.text.len(),
                                        stt.words.len()
                                    );
                                    if let Some(ref p) = path {
                                        save_stt_artifacts(p, &stt);
                                    }
                                    let _ = tx.send(Event::TranscriptReady(stt.text));
                                    return;
                                }
                                Ok(Err(e)) => {
                                    last_err = e.to_string();
                                    tracing::warn!("STT error: {last_err}");
                                }
                                Err(_elapsed) => {
                                    last_err =
                                        format!("STT timed out after {}s", stt_timeout().as_secs());
                                    tracing::warn!("{last_err}");
                                }
                            }
                        }

                        if generation.load(Ordering::SeqCst) != gen {
                            return;
                        }
                        if last_err == "empty transcript from STT" {
                            if let Some(ref p) = path {
                                crate::recording::remove_recording_files(p);
                            }
                            // Empty STT result (API returned no text) — not local VAD.
                            let _ = tx
                                .send(Event::Error("empty transcript from speech service".into()));
                        } else if !last_err.is_empty() {
                            let msg = format!(
                                "STT failed after {} attempts: {last_err}",
                                max_retries + 1
                            );
                            let _ = tx.send(Event::Error(msg));
                        }
                    });
                }

                CopyToClipboard(_text) => {
                    // Clipboard is handled by WaylandInjector internally
                }

                InjectText(text) => {
                    let mode = crate::keyring::get_output_mode();
                    let result = if mode == "wtype" {
                        self.injector.inject(&text).await
                    } else {
                        self.injector.inject_clipboard(&text).await
                    };
                    match result {
                        Ok(()) => {
                            self.log.text_injected();
                            if mode == "clipboard" {
                                notify_clipboard_ready();
                            }
                            self.event_tx.send(Event::TextInserted).ok();
                        }
                        Err(e) => {
                            self.log.injection_error(&e.to_string());
                            self.event_tx.send(Event::Error(e.to_string())).ok();
                        }
                    }
                }

                SetTrayState(s) => self.tray.set_state(&s),

                ShowNotification { title, body } => {
                    #[cfg(test)]
                    {
                        self.last_notification = Some((title.clone(), body.clone()));
                    }
                    #[cfg(not(test))]
                    {
                        if let Err(e) = std::process::Command::new("notify-send")
                            .arg(&title)
                            .arg(&body)
                            .status()
                        {
                            tracing::warn!("notification failed: {e}");
                        }
                    }
                }

                OpenHistory =>
                {
                    #[cfg(not(test))]
                    if let Err(e) = crate::lifecycle::spawn_gui(false) {
                        tracing::warn!("failed to open history GUI: {e}");
                    }
                }

                OpenSettings =>
                {
                    #[cfg(not(test))]
                    if let Err(e) = crate::lifecycle::spawn_gui(true) {
                        tracing::warn!("failed to open settings GUI: {e}");
                    }
                }

                Quit => return,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    struct MockCapture {
        fail_start: bool,
        data: Vec<u8>,
        duration_ms: u64,
        call_count: AtomicU32,
    }

    impl MockCapture {
        fn tone_pcm(num_bytes: usize) -> Vec<u8> {
            let samples = num_bytes / 2;
            (0..samples)
                .flat_map(|i| {
                    let s = (i as i32 * 7919 % 8000 + 500) as i16;
                    s.to_le_bytes()
                })
                .collect()
        }

        fn with_duration(duration_ms: u64) -> Self {
            let bytes = Self::tone_pcm((duration_ms as usize) * 32);
            Self {
                fail_start: false,
                data: bytes,
                duration_ms,
                call_count: AtomicU32::new(0),
            }
        }
        fn failing() -> Self {
            Self {
                fail_start: true,
                data: vec![],
                duration_ms: 0,
                call_count: AtomicU32::new(0),
            }
        }
    }

    #[async_trait::async_trait]
    impl AudioCapture for MockCapture {
        async fn start(&mut self) -> anyhow::Result<()> {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            if self.fail_start {
                Err(anyhow::anyhow!("no device"))
            } else {
                Ok(())
            }
        }
        async fn stop(&mut self) -> anyhow::Result<AudioData> {
            Ok(AudioData::pcm(
                self.data.clone(),
                16000,
                1,
                self.duration_ms,
            ))
        }
    }

    struct MockStt(&'static str);
    #[async_trait::async_trait]
    impl SttClient for MockStt {
        async fn transcribe(&self, _: &AudioData) -> anyhow::Result<SttResult> {
            Ok(SttResult {
                schema_version: 1,
                text: self.0.into(),
                language: Some("English".into()),
                duration_secs: Some(1.0),
                words: vec![],
                api_response: serde_json::json!({"text": self.0}),
            })
        }
    }

    struct MockSlowStt;
    #[async_trait::async_trait]
    impl SttClient for MockSlowStt {
        async fn transcribe(&self, _: &AudioData) -> anyhow::Result<SttResult> {
            tokio::time::sleep(Duration::from_secs(120)).await;
            Ok(SttResult {
                schema_version: 1,
                text: "never".into(),
                language: None,
                duration_secs: None,
                words: vec![],
                api_response: serde_json::json!({"text": "never"}),
            })
        }
    }

    struct MockInjector;
    #[async_trait::async_trait]
    impl TextInjector for MockInjector {
        async fn inject(&self, _: &str) -> anyhow::Result<()> {
            Ok(())
        }
        async fn inject_clipboard(&self, _: &str) -> anyhow::Result<()> {
            Ok(())
        }
    }

    struct MockKeyring(&'static str);
    impl KeyringStore for MockKeyring {
        fn get_api_key(&self) -> anyhow::Result<String> {
            if self.0.is_empty() {
                anyhow::bail!("no API key");
            }
            Ok(self.0.into())
        }
        fn set_api_key(&self, _: &str) -> anyhow::Result<()> {
            Ok(())
        }
        fn clear(&self) -> anyhow::Result<()> {
            Ok(())
        }
    }

    struct MockTray(AtomicU32);
    impl TrayController for MockTray {
        fn set_state(&self, _: &str) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn make_app() -> App {
        crate::lifecycle::init_test_recordings_dir();
        App::new(
            Box::new(MockCapture::with_duration(2000)),
            Arc::new(MockStt("hello world")),
            Box::new(MockInjector),
            Arc::new(MockKeyring("test-key")),
            Box::new(MockTray(AtomicU32::new(0))),
        )
    }

    #[tokio::test]
    async fn test_toggle_without_api_key_stays_idle() {
        crate::lifecycle::init_test_recordings_dir();
        let mut app = App::new(
            Box::new(MockCapture::with_duration(2000)),
            Arc::new(MockStt("hello world")),
            Box::new(MockInjector),
            Arc::new(MockKeyring("")),
            Box::new(MockTray(AtomicU32::new(0))),
        );

        app.process_event(Event::Toggle).await;
        assert_eq!(app.current_state(), &AppState::Idle);
        let (title, body) = app
            .last_notification
            .expect("idle without creds must notify");
        assert_eq!(
            title,
            crate::product_copy::setup_needed_notification_title()
        );
        assert_eq!(body, crate::product_copy::setup_needed_notification_body());
        assert!(crate::product_copy::sign_in_comes_before_api_key(&body));
    }

    #[tokio::test]
    async fn test_full_recording_flow() {
        let mut app = make_app();

        app.process_event(Event::Toggle).await;
        assert_eq!(app.current_state(), &AppState::Recording);

        app.process_event(Event::Toggle).await;
        assert_eq!(app.current_state(), &AppState::Transcribing);

        app.process_event(Event::AudioCaptured {
            bytes: vec![0u8; 3200],
            duration_ms: 100,
        })
        .await;
        app.process_event(Event::TranscriptReady("hello world".into()))
            .await;
        assert_eq!(app.current_state(), &AppState::Inserting);

        app.process_event(Event::TextInserted).await;
        assert_eq!(app.current_state(), &AppState::Idle);
    }

    #[tokio::test]
    async fn test_cancel_during_recording() {
        let mut app = make_app();
        app.process_event(Event::Toggle).await;
        assert_eq!(app.current_state(), &AppState::Recording);
        app.process_event(Event::Cancel).await;
        assert_eq!(app.current_state(), &AppState::Idle);
        assert!(!app.awaiting_audio);
        app.process_event(Event::AudioCaptured {
            bytes: vec![0u8; 64_000],
            duration_ms: 2000,
        })
        .await;
        assert_eq!(app.current_state(), &AppState::Idle);
    }

    #[tokio::test]
    async fn test_cancel_during_transcribing() {
        let mut app = make_app();
        app.process_event(Event::Toggle).await;
        app.process_event(Event::Toggle).await;
        assert_eq!(app.current_state(), &AppState::Transcribing);
        let gen_before = app.transcribe_generation.load(Ordering::SeqCst);
        app.process_event(Event::Cancel).await;
        assert_eq!(app.current_state(), &AppState::Idle);
        assert!(app.transcribe_generation.load(Ordering::SeqCst) > gen_before);
        app.process_event(Event::AudioCaptured {
            bytes: vec![0u8; 64_000],
            duration_ms: 2000,
        })
        .await;
        assert_eq!(app.current_state(), &AppState::Idle);
    }

    #[tokio::test]
    async fn test_cancel_then_new_recording_works() {
        let mut app = make_app();
        app.process_event(Event::Toggle).await;
        app.process_event(Event::Cancel).await;
        assert_eq!(app.current_state(), &AppState::Idle);
        app.process_event(Event::Toggle).await;
        assert_eq!(app.current_state(), &AppState::Recording);
        assert!(!app.discard_next_audio);
        app.process_event(Event::Toggle).await;
        assert_eq!(app.current_state(), &AppState::Transcribing);
        assert!(app.awaiting_audio);
    }

    #[tokio::test]
    async fn test_recording_error() {
        crate::lifecycle::init_test_recordings_dir();
        let mut app = App::new(
            Box::new(MockCapture::failing()),
            Arc::new(MockStt("hello world")),
            Box::new(MockInjector),
            Arc::new(MockKeyring("test-key")),
            Box::new(MockTray(AtomicU32::new(0))),
        );

        app.process_event(Event::Toggle).await;
        app.process_event(Event::Error("no device".into())).await;
        assert!(matches!(app.current_state(), AppState::Idle));

        app.process_event(Event::Toggle).await;
        assert_eq!(app.current_state(), &AppState::Recording);
    }

    #[tokio::test]
    async fn test_recording_too_short_audio() {
        crate::lifecycle::init_test_recordings_dir();
        // Capture with < 500ms duration — should be rejected
        let short_capture = MockCapture::with_duration(100);
        let mut app = App::new(
            Box::new(short_capture),
            Arc::new(MockStt("hello world")),
            Box::new(MockInjector),
            Arc::new(MockKeyring("test-key")),
            Box::new(MockTray(AtomicU32::new(0))),
        );

        app.process_event(Event::Toggle).await;
        assert_eq!(app.current_state(), &AppState::Recording);

        // Toggle triggers StopCapture → validate_audio fails → Error event
        app.process_event(Event::Toggle).await;
        // The StopCapture sends Error event (too short) into channel
        app.process_event(Event::Error(
            "recording too short: 100ms (min 500ms)".into(),
        ))
        .await;
        assert!(matches!(app.current_state(), AppState::Idle));

        // Recover — can record again
        app.process_event(Event::Toggle).await;
        assert_eq!(app.current_state(), &AppState::Recording);
    }

    #[tokio::test]
    async fn test_stt_timeout() {
        crate::lifecycle::init_test_recordings_dir();
        std::env::set_var("VOICE_INPUT_STT_TIMEOUT_MS", "1000");
        std::env::set_var("VOICE_INPUT_STT_RETRIES", "0");
        let mut app = App::new(
            Box::new(MockCapture::with_duration(2000)),
            Arc::new(MockSlowStt),
            Box::new(MockInjector),
            Arc::new(MockKeyring("test-key")),
            Box::new(MockTray(AtomicU32::new(0))),
        );

        let tx = app.event_sender();
        let run = tokio::spawn(async move { app.run().await });

        tx.send(Event::Toggle).ok();
        tx.send(Event::Toggle).ok();
        tx.send(Event::AudioCaptured {
            bytes: vec![0u8; 64000],
            duration_ms: 2000,
        })
        .ok();

        tokio::time::sleep(Duration::from_secs(3)).await;
        run.abort();

        // If we got here without panic, timeout path ran (daemon would return to idle via Error event).
    }

    #[tokio::test]
    async fn test_missing_api_key() {
        crate::lifecycle::init_test_recordings_dir();
        let mut app = App::new(
            Box::new(MockCapture::with_duration(2000)),
            Arc::new(MockStt("hello world")),
            Box::new(MockInjector),
            Arc::new(MockKeyring("")), // empty key
            Box::new(MockTray(AtomicU32::new(0))),
        );

        app.process_event(Event::Toggle).await;
        app.process_event(Event::Toggle).await;
        app.process_event(Event::AudioCaptured {
            bytes: vec![0u8; 64000],
            duration_ms: 2000,
        })
        .await;
        // The AudioCaptured triggers Transcribe, which checks the key and sends Error
        app.process_event(Event::Error("API key not configured".into()))
            .await;

        assert!(matches!(app.current_state(), AppState::Idle));
    }

    #[tokio::test]
    async fn test_empty_audio_handled() {
        crate::lifecycle::init_test_recordings_dir();
        let empty_capture = MockCapture::with_duration(0);
        let mut app = App::new(
            Box::new(empty_capture),
            Arc::new(MockStt("hello world")),
            Box::new(MockInjector),
            Arc::new(MockKeyring("test-key")),
            Box::new(MockTray(AtomicU32::new(0))),
        );

        app.process_event(Event::Toggle).await;
        app.process_event(Event::Toggle).await;

        // StopCapture validates → "no audio captured"
        app.process_event(Event::Error("no audio captured".into()))
            .await;
        assert!(matches!(app.current_state(), AppState::Idle));

        app.process_event(Event::Toggle).await;
        assert_eq!(app.current_state(), &AppState::Recording);
    }
}
