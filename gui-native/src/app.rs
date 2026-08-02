use crate::audio::AudioPlayer;
use crate::time::{format_duration, recording_time_labels};
use crate::ux::{
    active_auth_detail, active_auth_title, base_transcript_label, history_after_probe,
    history_has_more_after_page, history_preview, page_padding_tokens, settings_is_dirty,
    settings_save_visible, should_save_edit, show_more_after_list, show_version_switcher,
    stored_api_key_status, tray_capsule_rgb, tray_state_caption, tray_state_title,
    version_tab_label, HISTORY_PAGE_SIZE,
};
use cosmic::app::Task;
use cosmic::iced::clipboard;
use cosmic::iced::widget::text_editor;
use cosmic::iced::{Alignment, Background, Border, Color, Length, Subscription, Vector};
use cosmic::prelude::*;
use cosmic::widget::{
    self, divider, icon, nav_bar, segmented_button, segmented_control, settings, toaster, Toast,
    Toasts,
};
use cosmic::{executor, theme, Core};
use cosmic_scribe::api::{self, AppConfig, ConfigUpdate, HistoryEntry, RecordingDetail};
use std::time::Duration;

const POLL_MS: u64 = 2000;

/// COSMIC-style segmented choices (value, short label).
const OUTPUT_MODES: &[(&str, &str)] = &[
    ("wtype", "Type into focus"),
    ("clipboard", "Clipboard only"),
];
const TIME_MODES: &[(&str, &str)] = &[("relative", "Relative"), ("absolute", "Absolute")];

fn build_output_mode_model() -> segmented_button::SingleSelectModel {
    let mut builder = segmented_button::Model::builder();
    for (mode, label) in OUTPUT_MODES {
        builder = builder.insert(|b| b.text(*label).data(*mode));
    }
    let mut model = builder.build();
    model.activate_position(0);
    model
}

fn build_history_time_mode_model() -> segmented_button::SingleSelectModel {
    let mut model = segmented_button::Model::builder()
        .insert(|b| b.text("Relative").data("relative"))
        .insert(|b| b.text("Absolute").data("absolute"))
        .build();
    model.activate_position(0);
    model
}

fn activate_segmented_value(
    model: &mut segmented_button::SingleSelectModel,
    value: &str,
    choices: &[(&str, &str)],
) {
    for (pos, (mode, _)) in choices.iter().enumerate() {
        if *mode == value {
            model.activate_position(pos as u16);
            return;
        }
    }
    model.activate_position(0);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Page {
    History,
    Settings,
}

#[derive(Clone, Copy, Debug)]
pub struct Flags {
    pub open_settings: bool,
}

#[derive(Clone, Debug)]
pub enum Message {
    /// Full list replace (initial load or poll). `has_more` from probe, not `len >= 20`.
    HistoryLoaded {
        entries: Vec<HistoryEntry>,
        has_more: bool,
    },
    HistoryMoreLoaded(Vec<HistoryEntry>),
    HistoryRefresh,
    LoadMoreHistory,
    OpenRecording(String),
    BackFromDetail,
    /// Confirm permanent delete of this recording file id.
    DeleteHistoryEntry {
        file: String,
    },
    /// Enter delete-confirm for this file id (list or detail).
    ConfirmDelete(String),
    CancelDelete,
    DetailLoaded(RecordingDetail),
    DetailFailed(String),
    SetVersion(usize),
    ToggleEditing,
    CancelEdit,
    EditAction(text_editor::Action),
    SaveEdit,
    Transcribe,
    TranscribeDone(Result<String, String>),
    TogglePlay,
    StopAudio,
    SeekAudio(f32),
    AudioTick,
    /// Copy transcript; `flash_id` identifies which control shows "Copied" briefly.
    CopyText {
        text: String,
        flash_id: String,
    },
    CopiedFlash(String),
    ClearCopiedFlash(String),
    ToastDismiss(toaster::ToastId),
    SettingsLoaded(AppConfig),
    LangChanged(String),
    ApiKeyChanged(String),
    SttEndpointChanged(String),
    OutputModeSelected(segmented_button::Entity),
    TimeModeSelected(segmented_button::Entity),
    LoginXai,
    LogoutXai,
    RefreshAuth,
    ClearApiKey,
    OpenUrl(String),
    SaveSettings,
    SettingsSaved,
    SettingsFailed(String),
    SettingsSavedClear,
}

pub struct App {
    core: Core,
    nav: nav_bar::Model,
    debug_gui: bool,
    history_entries: Vec<HistoryEntry>,
    history_offset: usize,
    history_has_more: bool,
    history_loading: bool,
    history_loading_more: bool,
    /// File id pending delete confirmation (list or detail).
    confirming_delete: Option<String>,
    detail_id: Option<String>,
    detail: Option<RecordingDetail>,
    detail_error: Option<String>,
    active_version: usize,
    editing: bool,
    view_content: text_editor::Content,
    edit_content: text_editor::Content,
    transcribing: bool,
    audio: AudioPlayer,
    audio_playing: bool,
    audio_position: f32,
    audio_duration: f32,
    settings: SettingsForm,
    /// Last loaded/saved preferences — dirty = form differs from this.
    settings_saved_snap: SettingsSnapshot,
    /// False until first SettingsLoaded — prevents false dirty Save before load.
    settings_loaded: bool,
    output_mode_model: segmented_button::SingleSelectModel,
    history_time_mode_model: segmented_button::SingleSelectModel,
    settings_saving: bool,
    settings_saved: bool,
    settings_error: Option<String>,
    /// Which copy control is showing the brief "Copied" label (list file id or "detail").
    copied_flash: Option<String>,
    toasts: Toasts<Message>,
}

#[derive(Clone, Debug)]
struct SettingsForm {
    lang: String,
    api_key: String,
    output_mode: String,
    history_time_mode: String,
    stt_endpoint: String,
    /// Any credential resolvable (OAuth / env / stored key).
    has_key: bool,
    /// Pay-per-token key file present on disk.
    has_stored_api_key: bool,
    auth_mode: String,
}

#[derive(Clone, Debug)]
struct SettingsSnapshot {
    lang: String,
    output_mode: String,
    history_time_mode: String,
    stt_endpoint: String,
}

impl Default for SettingsSnapshot {
    fn default() -> Self {
        // Match SettingsForm defaults so dirty is false before/without load.
        Self {
            lang: "en".into(),
            output_mode: "wtype".into(),
            history_time_mode: "relative".into(),
            stt_endpoint: cosmic_scribe::keyring::DEFAULT_STT_ENDPOINT.into(),
        }
    }
}

impl Default for SettingsForm {
    fn default() -> Self {
        Self {
            lang: "en".into(),
            api_key: String::new(),
            output_mode: "wtype".into(),
            history_time_mode: "relative".into(),
            stt_endpoint: cosmic_scribe::keyring::DEFAULT_STT_ENDPOINT.into(),
            has_key: false,
            has_stored_api_key: false,
            auth_mode: "none".into(),
        }
    }
}

impl App {
    fn settings_dirty(&self) -> bool {
        if !self.settings_loaded {
            return false;
        }
        settings_is_dirty(
            &self.settings.lang,
            &self.settings.output_mode,
            &self.settings.history_time_mode,
            &self.settings.stt_endpoint,
            &self.settings.api_key,
            &self.settings_saved_snap.lang,
            &self.settings_saved_snap.output_mode,
            &self.settings_saved_snap.history_time_mode,
            &self.settings_saved_snap.stt_endpoint,
            !self.settings.has_key,
        )
    }
}

impl App {
    fn active_page(&self) -> Option<Page> {
        self.nav.active_data::<Page>().copied()
    }

    fn on_history_page(&self) -> bool {
        self.active_page() == Some(Page::History) && self.detail_id.is_none()
    }

    fn time_mode_absolute(&self) -> bool {
        self.settings.history_time_mode == "absolute"
    }

    /// Stop player and clear UI progress so the next take does not inherit a stale bar.
    fn reset_audio_ui(&mut self) {
        self.audio.stop();
        self.audio_playing = false;
        self.audio_position = 0.0;
        self.audio_duration = 0.0;
    }

    fn load_history_task(&self, offset: usize, limit: usize) -> Task<Message> {
        cosmic::task::future(async move {
            if offset == 0 {
                // Probe one past the display window so exhausted lists stay exhausted
                // after poll refresh (skeptic: has_more must not flip true from len>=20 alone).
                let raw = api::list_history(0, limit.saturating_add(1));
                let (keep, has_more) = history_after_probe(raw.len(), limit);
                let entries = raw.into_iter().take(keep).collect();
                Message::HistoryLoaded { entries, has_more }
            } else {
                Message::HistoryMoreLoaded(api::list_history(offset, limit))
            }
        })
    }

    fn load_detail_task(&self, id: String) -> Task<Message> {
        cosmic::task::future(async move {
            match api::get_recording(&id) {
                Ok(detail) => Message::DetailLoaded(detail),
                Err(e) => Message::DetailFailed(e.message()),
            }
        })
    }

    fn load_settings_task(&self) -> Task<Message> {
        cosmic::task::future(async move { Message::SettingsLoaded(api::get_config()) })
    }

    fn update_title(&mut self) -> Task<Message> {
        let suffix = if self.debug_gui { " (native)" } else { "" };
        let page = if self.detail_id.is_some() {
            "Recording"
        } else if self.active_page() == Some(Page::Settings) {
            "Settings"
        } else {
            "History"
        };
        let title = format!("Cosmic Scribe{suffix} — {page}");
        self.set_header_title(page.to_string());
        self.set_window_title(title)
    }

    fn version_text(&self) -> String {
        let Some(detail) = &self.detail else {
            return String::new();
        };
        if self.active_version == 0 {
            return detail.text.clone();
        }
        detail
            .versions
            .get(self.active_version - 1)
            .map(|v| v.text.clone())
            .unwrap_or_else(|| detail.text.clone())
    }

    fn toast(&mut self, msg: impl Into<String>) -> Task<Message> {
        self.toasts
            .push(Toast::new(msg.into()).duration(toaster::Duration::Short))
            .map(cosmic::Action::App)
    }
}

impl cosmic::Application for App {
    type Executor = executor::Default;
    type Flags = Flags;
    type Message = Message;
    const APP_ID: &'static str = "com.cosmic-scribe.gui";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, flags: Self::Flags) -> (Self, Task<Self::Message>) {
        let mut nav = nav_bar::Model::default();
        nav.insert()
            .text("History")
            .data(Page::History)
            .icon(icon::from_name("document-open-recent-symbolic").icon());
        nav.insert()
            .text("Settings")
            .data(Page::Settings)
            .icon(icon::from_name("preferences-system-symbolic").icon());

        if flags.open_settings {
            nav.activate_position(1);
        } else {
            nav.activate_position(0);
        }

        let debug_gui = std::env::args().any(|a| a.contains("debug"));

        let mut app = Self {
            core,
            nav,
            debug_gui,
            history_entries: Vec::new(),
            history_offset: 0,
            history_has_more: true,
            history_loading: true,
            history_loading_more: false,
            confirming_delete: None,
            detail_id: None,
            detail: None,
            detail_error: None,
            active_version: 0,
            editing: false,
            view_content: text_editor::Content::new(),
            edit_content: text_editor::Content::new(),
            transcribing: false,
            audio: AudioPlayer::default(),
            audio_playing: false,
            audio_position: 0.0,
            audio_duration: 0.0,
            settings: SettingsForm::default(),
            settings_saved_snap: SettingsSnapshot::default(),
            settings_loaded: false,
            output_mode_model: build_output_mode_model(),
            history_time_mode_model: build_history_time_mode_model(),
            settings_saving: false,
            settings_saved: false,
            settings_error: None,
            copied_flash: None,
            toasts: Toasts::new(Message::ToastDismiss),
        };

        let startup = Task::batch([
            app.load_history_task(0, HISTORY_PAGE_SIZE),
            app.load_settings_task(),
            app.update_title(),
        ]);
        (app, startup)
    }

    fn nav_model(&self) -> Option<&nav_bar::Model> {
        Some(&self.nav)
    }

    fn on_nav_select(&mut self, id: nav_bar::Id) -> Task<Self::Message> {
        self.nav.activate(id);
        self.detail_id = None;
        self.detail = None;
        self.detail_error = None;
        self.confirming_delete = None;
        self.editing = false;
        self.reset_audio_ui();
        if self.active_page() == Some(Page::Settings) {
            return Task::batch([self.load_settings_task(), self.update_title()]);
        }
        self.update_title()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        let mut subs = Vec::new();
        if self.on_history_page() {
            subs.push(
                cosmic::iced::time::every(Duration::from_millis(POLL_MS))
                    .map(|_| Message::HistoryRefresh),
            );
        }
        if self.audio_playing {
            subs.push(
                cosmic::iced::time::every(Duration::from_millis(100)).map(|_| Message::AudioTick),
            );
        }
        Subscription::batch(subs)
    }

    fn update(&mut self, message: Self::Message) -> Task<Self::Message> {
        match message {
            Message::HistoryLoaded { entries, has_more } => {
                self.history_loading = false;
                self.history_offset = entries.len();
                self.history_has_more = has_more;
                self.history_entries = entries;
            }
            Message::HistoryMoreLoaded(mut entries) => {
                self.history_loading_more = false;
                let n = entries.len();
                self.history_has_more = history_has_more_after_page(n, HISTORY_PAGE_SIZE);
                self.history_offset += n;
                self.history_entries.append(&mut entries);
                if n == 0 {
                    self.history_has_more = false;
                }
                // No toast: user pressed Show more; the list updating is enough feedback.
            }
            Message::HistoryRefresh => {
                if !self.on_history_page() {
                    return Task::none();
                }
                // Re-probe from the top with the currently displayed window size.
                let limit = self.history_entries.len().max(HISTORY_PAGE_SIZE);
                return self.load_history_task(0, limit);
            }
            Message::LoadMoreHistory => {
                if self.history_loading_more || !self.history_has_more {
                    return Task::none();
                }
                self.history_loading_more = true;
                return self.load_history_task(self.history_offset, HISTORY_PAGE_SIZE);
            }
            Message::OpenRecording(id) => {
                self.detail_id = Some(id.clone());
                self.detail = None;
                self.detail_error = None;
                self.confirming_delete = None;
                self.active_version = 0;
                self.editing = false;
                self.reset_audio_ui();
                return Task::batch([self.load_detail_task(id), self.update_title()]);
            }
            Message::BackFromDetail => {
                self.detail_id = None;
                self.detail = None;
                self.detail_error = None;
                self.confirming_delete = None;
                self.editing = false;
                self.reset_audio_ui();
                return self.update_title();
            }
            Message::ConfirmDelete(file) => {
                self.confirming_delete = Some(file);
            }
            Message::CancelDelete => {
                self.confirming_delete = None;
            }
            Message::DeleteHistoryEntry { file } => {
                self.confirming_delete = None;
                if let Err(e) = api::delete_recording(&file) {
                    return self.toast(e.message());
                }
                // Always drop by file id (detail path used to pass index=MAX and leave a ghost).
                self.history_entries.retain(|e| e.file != file);
                if self.detail_id.as_deref() == Some(file.as_str()) {
                    self.detail_id = None;
                    self.detail = None;
                    self.reset_audio_ui();
                    return Task::batch([self.toast("Recording deleted"), self.update_title()]);
                }
                return self.toast("Recording deleted");
            }
            Message::DetailLoaded(detail) => {
                self.detail_error = None;
                self.detail = Some(detail);
                let text = self.version_text();
                self.view_content = text_editor::Content::with_text(&text);
                self.edit_content = text_editor::Content::with_text(&text);
                return self.update_title();
            }
            Message::DetailFailed(msg) => {
                self.detail_error = Some(msg);
            }
            Message::SetVersion(idx) => {
                self.active_version = idx;
                self.editing = false;
                let text = self.version_text();
                self.view_content = text_editor::Content::with_text(&text);
                self.edit_content = text_editor::Content::with_text(&text);
            }
            Message::ToggleEditing => {
                self.editing = true;
                let text = self.version_text();
                self.edit_content = text_editor::Content::with_text(&text);
            }
            Message::CancelEdit => {
                // Silent: Cancel is expected and has no side effects.
                self.editing = false;
                let text = self.version_text();
                self.edit_content = text_editor::Content::with_text(&text);
            }
            Message::EditAction(action) => {
                self.edit_content.perform(action);
            }
            Message::SaveEdit => {
                let Some(id) = self.detail_id.clone() else {
                    return Task::none();
                };
                let text = self.edit_content.text();
                let current = self.version_text();
                if !should_save_edit(&current, &text) {
                    // Silent no-op: text unchanged.
                    self.editing = false;
                    return Task::none();
                }
                if let Err(e) = api::save_user_edit(&id, &text, "user_edit") {
                    return self.toast(e.message());
                }
                self.editing = false;
                return Task::batch([self.toast("Saved new version"), self.load_detail_task(id)]);
            }
            Message::Transcribe => {
                let Some(id) = self.detail_id.clone() else {
                    return Task::none();
                };
                self.transcribing = true;
                return cosmic::task::future(async move {
                    let result = api::transcribe_recording(&id)
                        .map(|r| r.text)
                        .map_err(|e| e.message());
                    Message::TranscribeDone(result)
                });
            }
            Message::TranscribeDone(result) => {
                self.transcribing = false;
                match result {
                    Ok(_) => {
                        if let Some(id) = self.detail_id.clone() {
                            return self.load_detail_task(id);
                        }
                    }
                    Err(e) => self.detail_error = Some(e),
                }
            }
            Message::TogglePlay => {
                let Some(id) = self.detail_id.clone() else {
                    return Task::none();
                };
                // Load WAV when nothing is loaded yet (or player was cleared).
                let need_load = self.audio_duration <= 0.0 || !self.audio.has_loaded();
                if need_load {
                    match api::read_audio_pcm(&id) {
                        Ok(pcm) => match self.audio.load_and_play(&pcm) {
                            Ok(dur) => {
                                self.audio_duration = dur;
                                self.audio_playing = true;
                                self.audio_position = 0.0;
                            }
                            Err(e) => {
                                self.reset_audio_ui();
                                return self.toast(e);
                            }
                        },
                        Err(e) => return self.toast(e.message()),
                    }
                } else {
                    let (playing, pos, dur) = self.audio.toggle();
                    self.audio_playing = playing;
                    self.audio_position = pos;
                    self.audio_duration = dur;
                }
            }
            Message::StopAudio => {
                let (_playing, pos, dur) = self.audio.stop_to_start();
                self.audio_playing = false;
                self.audio_position = pos;
                // Keep duration so the bar + Stop stay available for Play again.
                self.audio_duration = dur;
            }
            Message::SeekAudio(secs) => {
                if self.audio_duration <= 0.0 {
                    return Task::none();
                }
                // Ensure WAV is loaded so seek can restart playback from that point.
                if !self.audio.has_loaded() {
                    let Some(id) = self.detail_id.clone() else {
                        return Task::none();
                    };
                    match api::read_audio_pcm(&id) {
                        Ok(pcm) => {
                            if let Err(e) = self.audio.load_only(&pcm) {
                                return self.toast(e);
                            }
                        }
                        Err(e) => return self.toast(e.message()),
                    }
                }
                let fraction = (secs / self.audio_duration).clamp(0.0, 1.0);
                let (playing, pos, dur) = self.audio.seek_fraction(fraction);
                self.audio_playing = playing;
                self.audio_position = pos;
                self.audio_duration = dur;
            }
            Message::AudioTick => {
                let (playing, pos, dur) = self.audio.position();
                self.audio_playing = playing;
                self.audio_position = pos;
                if dur > 0.0 {
                    self.audio_duration = dur;
                }
            }
            Message::OpenUrl(url) => {
                let _ = std::process::Command::new("xdg-open").arg(&url).spawn();
            }
            Message::CopyText { text, flash_id } => {
                if text.trim().is_empty() {
                    return Task::none();
                }
                return Task::batch([
                    clipboard::write(text),
                    cosmic::task::message(Message::CopiedFlash(flash_id)),
                ]);
            }
            Message::CopiedFlash(id) => {
                // Inline flash on the control — no toast (which control is clear).
                self.copied_flash = Some(id.clone());
                return cosmic::task::future(async move {
                    std::thread::sleep(Duration::from_millis(1200));
                    Message::ClearCopiedFlash(id)
                });
            }
            Message::ClearCopiedFlash(id) => {
                if self.copied_flash.as_deref() == Some(id.as_str()) {
                    self.copied_flash = None;
                }
            }
            Message::ToastDismiss(id) => {
                self.toasts.remove(id);
            }
            Message::SettingsLoaded(config) => {
                self.settings.lang = config.lang.clone();
                self.settings.output_mode = config.output_mode.clone();
                self.settings.history_time_mode = config.history_time_mode.clone();
                self.settings.stt_endpoint = config.stt_endpoint.clone();
                self.settings.has_key = config.has_key;
                self.settings.has_stored_api_key = api::has_stored_api_key();
                self.settings.auth_mode = config.auth_mode.clone();
                self.settings.api_key.clear();
                self.settings_saved_snap = SettingsSnapshot {
                    lang: config.lang.clone(),
                    output_mode: config.output_mode.clone(),
                    history_time_mode: config.history_time_mode.clone(),
                    stt_endpoint: config.stt_endpoint.clone(),
                };
                self.settings_loaded = true;
                activate_segmented_value(
                    &mut self.output_mode_model,
                    &config.output_mode,
                    OUTPUT_MODES,
                );
                activate_segmented_value(
                    &mut self.history_time_mode_model,
                    &config.history_time_mode,
                    TIME_MODES,
                );
            }
            Message::LangChanged(v) => self.settings.lang = v,
            Message::ApiKeyChanged(v) => self.settings.api_key = v,
            Message::SttEndpointChanged(v) => self.settings.stt_endpoint = v,
            Message::OutputModeSelected(entity) => {
                self.output_mode_model.activate(entity);
                if let Some(mode) = self.output_mode_model.data::<&str>(entity) {
                    self.settings.output_mode = (*mode).to_string();
                }
            }
            Message::TimeModeSelected(entity) => {
                self.history_time_mode_model.activate(entity);
                if let Some(mode) = self.history_time_mode_model.data::<&str>(entity) {
                    self.settings.history_time_mode = (*mode).to_string();
                }
            }
            Message::LoginXai => {
                let bin = cosmic_scribe::lifecycle::daemon_binary();
                match std::process::Command::new(&bin).arg("--login").spawn() {
                    Ok(_) => {
                        return Task::batch([
                            self.toast("Finish sign-in in the browser, then press Refresh"),
                            cosmic::task::future(async {
                                std::thread::sleep(Duration::from_secs(45));
                                Message::RefreshAuth
                            }),
                        ]);
                    }
                    Err(e) => {
                        return self.toast(format!("Could not start login: {e}"));
                    }
                }
            }
            Message::LogoutXai => match cosmic_scribe::xai_oauth::clear() {
                Ok(()) => {
                    self.settings.auth_mode = "none".into();
                    return Task::batch([self.toast("Signed out"), self.load_settings_task()]);
                }
                Err(e) => return self.toast(format!("Logout failed: {e}")),
            },
            Message::RefreshAuth => {
                return self.load_settings_task();
            }
            Message::ClearApiKey => match api::clear_stored_api_key() {
                Ok(()) => {
                    self.settings.has_stored_api_key = false;
                    self.settings.api_key.clear();
                    return Task::batch([self.toast("API key removed"), self.load_settings_task()]);
                }
                Err(e) => return self.toast(format!("Could not remove API key: {}", e.message())),
            },
            Message::SaveSettings => {
                self.settings_saving = true;
                self.settings_saved = false;
                self.settings_error = None;
                let update = ConfigUpdate {
                    lang: Some(self.settings.lang.clone()),
                    output_mode: Some(self.settings.output_mode.clone()),
                    history_time_mode: Some(self.settings.history_time_mode.clone()),
                    stt_endpoint: Some(self.settings.stt_endpoint.clone()),
                    key: if self.settings.api_key.is_empty() {
                        None
                    } else {
                        Some(self.settings.api_key.clone())
                    },
                    ..Default::default()
                };
                return cosmic::task::future(async move {
                    match api::save_config(&update) {
                        Ok(()) => Message::SettingsSaved,
                        Err(e) => Message::SettingsFailed(e.message()),
                    }
                });
            }
            Message::SettingsSaved => {
                self.settings_saving = false;
                self.settings_saved = true;
                if !self.settings.api_key.is_empty() {
                    self.settings.has_key = true;
                    self.settings.has_stored_api_key = true;
                }
                self.settings.api_key.clear();
                self.settings_saved_snap = SettingsSnapshot {
                    lang: self.settings.lang.clone(),
                    output_mode: self.settings.output_mode.clone(),
                    history_time_mode: self.settings.history_time_mode.clone(),
                    stt_endpoint: self.settings.stt_endpoint.clone(),
                };
                // Reload so auth_mode reflects what STT will actually use.
                return Task::batch([
                    self.toast("Settings saved"),
                    self.load_settings_task(),
                    cosmic::task::future(async {
                        std::thread::sleep(Duration::from_secs(2));
                        Message::SettingsSavedClear
                    }),
                ]);
            }
            Message::SettingsFailed(msg) => {
                self.settings_saving = false;
                self.settings_error = Some(msg.clone());
                return self.toast(format!("Save failed: {msg}"));
            }
            Message::SettingsSavedClear => {
                self.settings_saved = false;
            }
        }
        Task::none()
    }

    fn view(&self) -> cosmic::Element<'_, Self::Message> {
        let spacing = f32::from(cosmic::theme::spacing().space_s);
        let content: cosmic::Element<'_, Message> = if self.detail_id.is_some() {
            self.view_detail(spacing)
        } else if self.active_page() == Some(Page::History) {
            self.view_history(spacing)
        } else {
            self.view_settings(spacing)
        };

        // Ensure page content can expand across the window (not a shrink-to-fit column).
        let content = widget::container(content)
            .width(Length::Fill)
            .height(Length::Fill);

        toaster(&self.toasts, content).into()
    }

    fn header_end(&self) -> Vec<cosmic::Element<'_, Self::Message>> {
        // Minimal chrome: Save only when preferences are dirty (and config has loaded).
        let on_settings = self.active_page() == Some(Page::Settings);
        if !on_settings || !settings_save_visible(self.settings_loaded, self.settings_dirty()) {
            return Vec::new();
        }
        let save = if self.settings_saving {
            widget::button::suggested("Saving…")
        } else if self.settings_saved {
            widget::button::suggested("Saved")
        } else {
            widget::button::suggested("Save").on_press(Message::SaveSettings)
        };
        vec![save.into()]
    }
}

impl App {
    fn page_pad(&self) -> cosmic::iced::Padding {
        let (t, r, b, l) = page_padding_tokens();
        cosmic::iced::Padding::from([t, r, b, l])
    }

    fn view_history(&self, spacing: f32) -> cosmic::Element<'_, Message> {
        let pad = self.page_pad();
        if self.history_loading && self.history_entries.is_empty() {
            return widget::container(widget::text::body("Loading recordings…"))
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(pad)
                .into();
        }

        if self.history_entries.is_empty() {
            return widget::container(
                widget::column::with_capacity(2)
                    .spacing(spacing)
                    .push(widget::text::title4("No recordings yet"))
                    .push(widget::text::body(
                        "Use your keyboard shortcut or the tray mic to dictate. Recordings appear here.",
                    ))
                    .align_x(Alignment::Center),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(pad)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into();
        }

        // Own column (not list_column): avoids double padding so hover fills the row.
        let mut list = widget::column::with_capacity(self.history_entries.len().saturating_mul(2));
        for (i, entry) in self.history_entries.iter().enumerate() {
            if i > 0 {
                list = list.push(divider::horizontal::default());
            }
            list = list.push(self.history_row(entry, i, spacing));
        }

        let mut scroll_body = widget::column::with_capacity(3)
            .spacing(0)
            .push(
                widget::container(list)
                    .width(Length::Fill)
                    .class(theme::Container::List),
            )
            .width(Length::Fill);

        if show_more_after_list(self.history_has_more, self.history_entries.len()) {
            let label = if self.history_loading_more {
                "Loading more…"
            } else {
                "Show more"
            };
            let more = if self.history_loading_more {
                widget::button::standard(label)
            } else {
                widget::button::standard(label).on_press(Message::LoadMoreHistory)
            };
            scroll_body = scroll_body.push(
                widget::container(more)
                    .width(Length::Fill)
                    .padding([theme::spacing().space_m, 0])
                    .align_x(Alignment::Center),
            );
        }

        widget::container(
            widget::scrollable(scroll_body)
                .height(Length::Fill)
                .width(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(pad)
        .into()
    }

    fn history_row(
        &self,
        entry: &HistoryEntry,
        _index: usize,
        spacing: f32,
    ) -> cosmic::Element<'_, Message> {
        let labels = recording_time_labels(&entry.ts, self.time_mode_absolute());
        let time_label = labels.primary.clone();
        let duration_label = format_duration(&entry.duration);
        let preview = history_preview(entry.text.as_deref(), 80);

        // Actions sit *outside* the open button so nested widgets don't kill row hover.
        // Fixed-width slots avoid layout jump: Copy↔Copied, missing transcript, delete confirm.
        let mut actions = widget::row::with_capacity(3)
            .spacing(theme::spacing().space_xxs)
            .align_y(Alignment::Center);

        // Copy slot always present (spacer when no text) so rows share the same width.
        let copy_slot: cosmic::Element<'_, Message> =
            if let Some(text) = entry.text.clone().filter(|_| entry.has_text) {
                let flash = self.copied_flash.as_deref() == Some(entry.file.as_str());
                // Fixed label width: "Copied" is the longest state.
                let label = if flash { "Copied" } else { "Copy  " };
                let btn = widget::button::text(label).on_press(Message::CopyText {
                    text,
                    flash_id: entry.file.clone(),
                });
                if flash {
                    btn.class(theme::Button::Suggested).into()
                } else {
                    btn.into()
                }
            } else {
                widget::Space::new().width(56).height(1).into()
            };
        actions = actions.push(widget::container(copy_slot).width(Length::Fixed(72.0)));

        if self.confirming_delete.as_deref() == Some(entry.file.as_str()) {
            actions = actions
                .push(
                    widget::button::destructive("Delete").on_press(Message::DeleteHistoryEntry {
                        file: entry.file.clone(),
                    }),
                )
                .push(widget::button::standard("Cancel").on_press(Message::CancelDelete));
        } else {
            actions = actions.push(
                widget::button::icon(icon::from_name("edit-delete-symbolic"))
                    .extra_small()
                    .on_press(Message::ConfirmDelete(entry.file.clone())),
            );
        }

        let mut meta = widget::row::with_capacity(4)
            .spacing(theme::spacing().space_s)
            .align_y(Alignment::Center)
            .push(widget::text::body(time_label))
            .push(widget::text::caption(duration_label));
        if !preview.stats.is_empty() {
            meta = meta.push(widget::text::caption(preview.stats.clone()));
        }
        meta = meta.push(widget::space::horizontal());

        let main = widget::column::with_capacity(2)
            .spacing(2)
            .push(meta)
            .push(widget::text::caption(preview.text_line))
            .width(Length::Fill);

        // Open hit-target: text + chevron only (full-width fill for easy hover).
        let open_body = widget::row::with_capacity(2)
            .spacing(theme::spacing().space_s)
            .align_y(Alignment::Center)
            .push(main)
            .push(icon::from_name("go-next-symbolic").size(16).icon())
            .width(Length::Fill);

        let open = widget::button::custom(open_body)
            .padding([10, 12])
            .width(Length::Fill)
            .on_press(Message::OpenRecording(entry.file.clone()))
            .class(history_row_button_class());

        let _ = spacing;
        widget::row::with_capacity(2)
            .spacing(theme::spacing().space_xxs)
            .align_y(Alignment::Center)
            .push(open)
            .push(widget::container(actions).padding([0, theme::spacing().space_s, 0, 0]))
            .width(Length::Fill)
            .into()
    }

    fn view_detail(&self, spacing: f32) -> cosmic::Element<'_, Message> {
        let pad = self.page_pad();
        let mut col = widget::column::with_capacity(12)
            .spacing(spacing)
            .width(Length::Fill)
            .height(Length::Fill);

        // System back: previous icon + label (aligned), not “←” text hacks.
        col = col.push(
            widget::button::standard("History")
                .leading_icon(icon::from_name("go-previous-symbolic"))
                .on_press(Message::BackFromDetail),
        );

        if let Some(err) = &self.detail_error {
            col = col.push(widget::text::body(err));
            return widget::container(col)
                .padding(pad)
                .width(Length::Fill)
                .into();
        }

        let Some(detail) = &self.detail else {
            col = col.push(widget::text::body("Loading recording…"));
            return widget::container(col)
                .padding(pad)
                .width(Length::Fill)
                .into();
        };

        let labels = recording_time_labels(&detail.ts, self.time_mode_absolute());
        let time_label = labels.primary.clone();
        let text = self.version_text();
        let has_text = !text.trim().is_empty();

        let play_label = if self.audio_playing { "Pause" } else { "Play" };
        let mut toolbar = widget::row::with_capacity(6).spacing(spacing);
        toolbar = toolbar.push(widget::button::standard(play_label).on_press(Message::TogglePlay));
        // Stop while playing or after audio is loaded (bar + rewind).
        if self.audio_playing || self.audio_duration > 0.0 {
            toolbar = toolbar.push(widget::button::standard("Stop").on_press(Message::StopAudio));
        }
        if has_text {
            let flash = self.copied_flash.as_deref() == Some("detail");
            let label = if flash { "Copied" } else { "Copy  " };
            let copy_btn = widget::button::standard(label).on_press(Message::CopyText {
                text: text.clone(),
                flash_id: "detail".into(),
            });
            toolbar = toolbar.push(if flash {
                copy_btn.class(theme::Button::Suggested)
            } else {
                copy_btn
            });
        }
        if self.confirming_delete.as_deref() == Some(detail.file.as_str()) {
            toolbar = toolbar
                .push(widget::button::destructive("Confirm delete").on_press(
                    Message::DeleteHistoryEntry {
                        file: detail.file.clone(),
                    },
                ))
                .push(widget::button::standard("Cancel").on_press(Message::CancelDelete));
        } else {
            toolbar = toolbar.push(
                widget::button::destructive("Delete")
                    .on_press(Message::ConfirmDelete(detail.file.clone())),
            );
        }

        col = col
            .push(
                widget::column::with_capacity(2)
                    .spacing(theme::spacing().space_xxs)
                    .push(widget::text::title3(time_label))
                    .push(widget::text::caption(format!(
                        "{} · {}",
                        labels.tooltip,
                        format_duration(&detail.duration)
                    ))),
            )
            .push(toolbar);

        if self.audio_duration > 0.0 {
            let pos = self.audio_position.clamp(0.0, self.audio_duration);
            col = col.push(widget::text::caption(format!(
                "{} / {}",
                format_time(pos),
                format_time(self.audio_duration)
            )));
            col = col.push(
                widget::slider(0.0..=self.audio_duration, pos, Message::SeekAudio)
                    .step(0.05_f32.max(self.audio_duration / 200.0))
                    .width(Length::Fill),
            );
        }

        if !has_text {
            col = col.push(widget::text::body(
                "No transcript yet. Play the audio if you like, then press Transcribe when online.",
            ));
            let label = if self.transcribing {
                "Transcribing…"
            } else {
                "Transcribe"
            };
            let btn = if self.transcribing {
                widget::button::suggested(label)
            } else {
                widget::button::suggested(label).on_press(Message::Transcribe)
            };
            col = col.push(btn);
        }

        // Only show version switcher when there is more than the base transcript.
        if show_version_switcher(detail.versions.len()) {
            col = col.push(self.version_tabs(detail, spacing));
        }

        if self.editing {
            col = col
                .push(
                    widget::container(
                        widget::text_editor(&self.edit_content)
                            .height(Length::Fill)
                            .on_action(Message::EditAction),
                    )
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .padding(theme::spacing().space_s)
                    .class(theme::Container::List),
                )
                .push(
                    widget::row::with_capacity(2)
                        .spacing(spacing)
                        .push(widget::button::suggested("Save").on_press(Message::SaveEdit))
                        .push(widget::button::standard("Cancel").on_press(Message::CancelEdit)),
                );
            return widget::container(col)
                .padding(pad)
                .width(Length::Fill)
                .height(Length::Fill)
                .into();
        }

        if has_text {
            // Read-only surface (not text_editor) — avoids caret/scroll glitches when viewing.
            // Single outer scroll below keeps layout stable (no nested Fill scroll thrash).
            col = col
                .push(
                    widget::container(widget::text::body(text))
                        .width(Length::Fill)
                        .padding(theme::spacing().space_m)
                        .class(theme::Container::List),
                )
                .push(widget::button::standard("Edit").on_press(Message::ToggleEditing));
        }

        widget::scrollable(widget::container(col).padding(pad).width(Length::Fill))
            .height(Length::Fill)
            .width(Length::Fill)
            .into()
    }

    fn version_tabs(&self, detail: &RecordingDetail, spacing: f32) -> cosmic::Element<'_, Message> {
        let mut row = widget::row::with_capacity(detail.versions.len() + 1).spacing(spacing);
        row = row.push(
            widget::button::standard(base_transcript_label())
                .on_press(Message::SetVersion(0))
                .class(if self.active_version == 0 {
                    cosmic::theme::Button::Suggested
                } else {
                    cosmic::theme::Button::Standard
                }),
        );
        for (i, version) in detail.versions.iter().enumerate() {
            let idx = i + 1;
            let label = version_tab_label(version, idx);
            row = row.push(
                widget::button::standard(label)
                    .on_press(Message::SetVersion(idx))
                    .class(if self.active_version == idx {
                        cosmic::theme::Button::Suggested
                    } else {
                        cosmic::theme::Button::Standard
                    }),
            );
        }
        row.into()
    }

    fn view_settings(&self, _spacing: f32) -> cosmic::Element<'_, Message> {
        let key_placeholder = if self.settings.has_stored_api_key {
            "(key saved — type a new one to replace)"
        } else {
            "paste speech API key…"
        };

        let space = theme::spacing();
        let pad = self.page_pad();

        let output_pick = segmented_control::horizontal(&self.output_mode_model)
            .on_activate(Message::OutputModeSelected);
        let time_pick = segmented_control::horizontal(&self.history_time_mode_model)
            .on_activate(Message::TimeModeSelected);

        // Auth: status always; Sign in only when not OAuth; Sign out only when OAuth.
        let signed_in = self.settings.auth_mode == "oauth";
        let mut account_section = settings::section().title("Account").add(
            settings::item::builder("Connection")
                .description(active_auth_detail(&self.settings.auth_mode))
                .control(widget::text::body(active_auth_title(
                    &self.settings.auth_mode,
                ))),
        );

        if signed_in {
            account_section = account_section.add(
                settings::item::builder("Signed-in account")
                    .description("Sign out leaves any saved API key in place.")
                    .control(
                        widget::row::with_capacity(2)
                            .spacing(space.space_s)
                            .push(
                                widget::button::standard("Refresh").on_press(Message::RefreshAuth),
                            )
                            .push(
                                widget::button::standard("Sign out").on_press(Message::LogoutXai),
                            ),
                    ),
            );
        } else {
            account_section = account_section
                .add(
                    settings::item::builder("Sign in (optional)")
                        .description("Browser login for SuperGrok or X Premium+ plan access.")
                        .control(widget::button::suggested("Sign in").on_press(Message::LoginXai)),
                )
                .add(
                    settings::item::builder("After signing in")
                        .description("Press Refresh when browser sign-in finishes.")
                        .control(
                            widget::button::standard("Refresh").on_press(Message::RefreshAuth),
                        ),
                );
        }

        account_section = account_section.add(
            settings::item::builder("API key on this computer")
                .description("For cloud speech. An environment key takes priority.")
                .control(widget::text::body(stored_api_key_status(
                    self.settings.has_stored_api_key,
                ))),
        );

        if self.settings.has_stored_api_key {
            account_section = account_section.add(
                settings::item::builder("Remove API key")
                    .description("Removes only the key saved here, not plan sign-in.")
                    .control(
                        widget::button::destructive("Remove key").on_press(Message::ClearApiKey),
                    ),
            );
        }

        account_section = account_section.add(
            settings::item::builder("Add or replace API key")
                .description("Paste a speech API key, then Save.")
                .control(
                    widget::text_input::secure_input(
                        key_placeholder,
                        &self.settings.api_key,
                        None,
                        true,
                    )
                    .on_input(Message::ApiKeyChanged)
                    .width(Length::Fixed(280.0)),
                ),
        );

        // API key is the general path; plan sign-in is optional for SuperGrok / Premium+.
        let access_section = settings::section().title("About access").add(
            settings::item::builder("How speech works")
                .description(
                    "Bearer API key for cloud STT. SuperGrok / X Premium+ can sign in for plan access.",
                )
                .control(
                    widget::button::link("Provider notes")
                        .on_press(Message::OpenUrl(
                            "https://github.com/erik-balfe/cosmic-scribe/blob/master/docs/STT_PROVIDERS.md"
                                .into(),
                        )),
                ),
        );

        let speech_section = settings::section()
            .title("Speech")
            .add(
                settings::item::builder("Language")
                    .description("Language code for recognition (en, ru, de, ja…).")
                    .control(
                        widget::text_input::text_input("en", &self.settings.lang)
                            .on_input(Message::LangChanged)
                            .width(Length::Fixed(120.0)),
                    ),
            )
            .add(
                settings::item::builder("STT endpoint")
                    .description(
                        "Full URL for the current dialect (default xAI). Not a full OpenAI swap.",
                    )
                    .control(
                        widget::text_input::text_input(
                            cosmic_scribe::keyring::DEFAULT_STT_ENDPOINT,
                            &self.settings.stt_endpoint,
                        )
                        .on_input(Message::SttEndpointChanged)
                        .width(Length::Fixed(320.0)),
                    ),
            );

        let output_section = settings::section().title("When text is ready").add(
            settings::item::builder("Output")
                .description("Type into the focused app, or only copy for you to paste.")
                .control(output_pick),
        );

        let history_section = settings::section().title("History").add(
            settings::item::builder("Time labels")
                .description("How recording times appear in the list.")
                .control(time_pick),
        );

        // Colored capsules match the tray mic glyph (not mystery symbolic icons).
        let tray_section = settings::section()
            .title("Tray microphone")
            .add(
                settings::item::builder(tray_state_title("idle"))
                    .description(tray_state_caption("idle"))
                    .control(tray_legend_control("idle")),
            )
            .add(
                settings::item::builder(tray_state_title("recording"))
                    .description(tray_state_caption("recording"))
                    .control(tray_legend_control("recording")),
            )
            .add(
                settings::item::builder(tray_state_title("recognizing"))
                    .description(tray_state_caption("recognizing"))
                    .control(tray_legend_control("recognizing")),
            );

        let mut sections = vec![
            account_section.into(),
            access_section.into(),
            speech_section.into(),
            output_section.into(),
            history_section.into(),
            tray_section.into(),
        ];

        if let Some(err) = &self.settings_error {
            sections.push(
                settings::section()
                    .add(
                        settings::item::builder("Could not save")
                            .description(err.clone())
                            .control(widget::space::horizontal().width(Length::Shrink)),
                    )
                    .into(),
            );
        }

        let column = settings::view_column(sections).width(Length::Fill);

        widget::scrollable(widget::container(column).width(Length::Fill).padding(pad))
            .height(Length::Fill)
            .width(Length::Fill)
            .into()
    }
}

fn format_time(secs: f32) -> String {
    let total = secs.max(0.0) as u32;
    format!("{}:{:02}", total / 60, total % 60)
}

/// Visible hover for history open-targets.
///
/// `list_button.hover` is often too close to the List surface to notice; use the
/// same primary hover COSMIC uses for selected list rows.
fn history_row_button_class() -> theme::Button {
    theme::Button::Custom {
        active: Box::new(|_focused, theme| history_row_style(theme, false)),
        disabled: Box::new(|theme| history_row_style(theme, false)),
        hovered: Box::new(|_focused, theme| history_row_style(theme, true)),
        pressed: Box::new(|_focused, theme| history_row_style(theme, true)),
    }
}

fn history_row_style(theme: &cosmic::Theme, hovered: bool) -> widget::button::Style {
    let cosmic = theme.cosmic();
    let bg = if hovered {
        Color::from(cosmic.primary.component.hover)
    } else {
        Color::TRANSPARENT
    };
    widget::button::Style {
        shadow_offset: Vector::default(),
        background: Some(Background::Color(bg)),
        border_radius: cosmic.corner_radii.radius_s.into(),
        border_width: 0.0,
        border_color: Color::TRANSPARENT,
        outline_width: 0.0,
        outline_color: Color::TRANSPARENT,
        icon_color: None,
        text_color: None,
        overlay: None,
    }
}

/// Mic-capsule glyph + short label for Settings tray legend.
fn tray_legend_control<'a, Message: 'a + Clone>(state: &str) -> cosmic::Element<'a, Message> {
    let label = match state {
        "recording" => "Red",
        "recognizing" => "Blue",
        _ => "Idle",
    };
    widget::row::with_capacity(2)
        .spacing(theme::spacing().space_xs)
        .align_y(Alignment::Center)
        .push(tray_capsule_el::<Message>(state))
        .push(widget::text::caption(label))
        .into()
}

/// Mic-capsule glyph for Settings legend (idle / red / blue).
fn tray_capsule_el<'a, Message: 'a>(state: &str) -> cosmic::Element<'a, Message> {
    let (r, g, b) = tray_capsule_rgb(state);
    let color = Color::from_rgb(r, g, b);
    widget::container(widget::Space::new().width(14).height(22))
        .width(14)
        .height(22)
        .class(theme::Container::custom(move |_theme| {
            widget::container::Style {
                background: Some(Background::Color(color)),
                border: Border {
                    radius: 6.0.into(),
                    width: 0.0,
                    color: Color::TRANSPARENT,
                },
                ..Default::default()
            }
        }))
        .into()
}
