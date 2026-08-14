//! User-facing CLI help. Kept in the library so tests can lock `--login` copy.

use crate::APP_SLUG;

pub fn usage_text() -> String {
    format!(
        "\
Cosmic Scribe — speak, then the text is in the focused field

  --help | -h         This help

Service (background daemon + tray):
  --install            Stop daemon, install binary, enable systemd autostart, start
  --install-from=PATH  Install from PATH (default: release/cargo if newer)
  --update             Stop → install from this binary → start
  --update-from=PATH   Same, but copy from PATH (e.g. new release build)
  --start              Start daemon via systemd (or direct if not installed)
  --stop | --quit      Stop daemon (tray goes away)
  --restart            Stop then start daemon
  --status             Running? installed path? IPC socket?
  --uninstall          Stop daemon; remove ~/.local install + autostart
  --purge              With --uninstall: also delete ~/.local/share/{APP_SLUG}/
  --daemon             Run in foreground (used by systemd unit; not for daily use)

Dictation:
  --trigger            Toggle recording on running daemon
  --cancel             Abort recording or STT (bind e.g. Ctrl+Shift+Space)
  --record-once        Record, transcribe, insert text, exit
  --file-input=<path>  Transcribe pre-recorded raw PCM

Setup:
  --login              Sign in (SuperGrok / X Premium+ plan access)
  --logout             Sign out (API key left untouched)
  --no-browser         With --login: print URL only (SSH/headless)
  --configure          Interactive auth + language
  --history            History window
  --settings           Settings window
  --autostart          Enable com.cosmic-scribe.service (graphical-session.target)
  --set-key KEY        Store speech API key (or COSMIC_SCRIBE_API_KEY)
  --clear-key          Remove stored API key
  --set-lang LANG      Set speech language (default: en)

Speech endpoint (xAI dialect; see docs/STT_PROVIDERS.md):
  COSMIC_SCRIBE_STT_URL  Full STT URL (default https://api.x.ai/v1/stt)
"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_documents_login_as_ordinary_path() {
        let t = usage_text();
        assert!(t.contains("--login"));
        assert!(t.contains("SuperGrok"));
        assert!(t.contains("X Premium+"));
        assert!(t.contains("--help"));
        assert!(t.contains("--cancel"));
    }
}
