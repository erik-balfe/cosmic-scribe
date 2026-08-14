pub const APP_SLUG: &str = "cosmic-scribe";

/// Prefer `COSMIC_SCRIBE_*`; fall back to legacy `VOICE_INPUT_*` env names.
pub fn env_compat(primary: &str, legacy: &str) -> Option<String> {
    std::env::var(primary)
        .ok()
        .or_else(|| std::env::var(legacy).ok())
}

pub mod analytics;
pub mod api;
pub mod app;
pub mod audio;
pub mod audio_validation;
pub mod cli_help;
pub mod injector;
pub mod ipc;
pub mod keyring;
pub mod lifecycle;
pub mod logging;
pub mod product_copy;
pub mod recording;
pub mod state;
pub mod stt;
pub mod traits;
pub mod tray;
mod tray_theme;
pub mod web;
pub mod xai_oauth;

pub use state::{AppState, Command, Event};
pub use traits::*;
