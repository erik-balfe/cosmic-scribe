//! User-visible Settings / first-run strings. Sign-in is the ordinary path.

pub fn sign_in_item_title() -> &'static str {
    "Sign in"
}

pub fn sign_in_item_description() -> &'static str {
    "SuperGrok or X Premium+ — same account as grok.com."
}

pub fn access_how_speech_works() -> &'static str {
    "Sign in with SuperGrok or X Premium+. An API key is a fallback."
}

pub fn analytics_item_description() -> &'static str {
    "Off by default. Counts only — no words, audio, or who you are."
}

pub fn active_auth_title(mode: &str) -> &'static str {
    match mode {
        "oauth" => "Signed in",
        "api_key" => "API key (saved here)",
        "api_key_env" => "API key (from environment)",
        _ => "Not set up yet",
    }
}

pub fn active_auth_detail(mode: &str) -> &'static str {
    match mode {
        "oauth" => "Using your plan for speech recognition.",
        "api_key" => "Using the API key saved on this computer.",
        "api_key_env" => "Using the API key from your environment.",
        _ => "Sign in, or add an API key, to start dictating.",
    }
}

/// Idle / first-run tray notification when nothing is configured.
pub fn setup_needed_notification_title() -> &'static str {
    "Set up speech access"
}

pub fn setup_needed_notification_body() -> &'static str {
    "Sign in with SuperGrok or X Premium+ (cosmic-scribe --login), or add an API key in Settings."
}

/// STT / re-transcribe error when no OAuth session and no key.
pub fn no_credentials_error() -> &'static str {
    "No speech credentials — run cosmic-scribe --login, or add an API key in Settings"
}

/// `--configure` banner: sign-in first, key second. Never “optional: --login”.
pub fn configure_auth_help() -> &'static str {
    "  Sign in:  --login    SuperGrok / X Premium+ (ordinary path)\n  \
     API key:  --set-key  (or paste in Settings)"
}

/// True when a sign-in cue appears before an API-key cue (or only sign-in is present).
pub fn sign_in_comes_before_api_key(s: &str) -> bool {
    let lower = s.to_ascii_lowercase();
    let sign = lower.find("sign in").or_else(|| lower.find("--login"));
    let key = lower.find("api key").or_else(|| lower.find("--set-key"));
    match (sign, key) {
        (Some(a), Some(b)) => a < b,
        (Some(_), None) => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn description_ok(s: &str) -> bool {
        s.chars().count() <= 90
    }

    #[test]
    fn sign_in_is_ordinary_path_not_optional_hero() {
        assert_eq!(sign_in_item_title(), "Sign in");
        assert!(!sign_in_item_title()
            .to_ascii_lowercase()
            .contains("optional"));
        assert!(sign_in_item_description().contains("SuperGrok"));
        assert!(description_ok(sign_in_item_description()));
        let access = access_how_speech_works();
        assert!(access.starts_with("Sign in"));
        assert!(access.contains("API key"));
        assert!(description_ok(access));
        let none = active_auth_detail("none").to_ascii_lowercase();
        let sign_at = none.find("sign in").expect("sign in first");
        let key_at = none.find("api key").expect("api key second");
        assert!(sign_at < key_at);
        assert!(description_ok(analytics_item_description()));
        assert!(analytics_item_description()
            .to_ascii_lowercase()
            .contains("off by default"));
        assert!(!access_how_speech_works().contains("OpenRouter"));
        assert!(!access_how_speech_works().contains("AI fix"));
        assert!(!sign_in_item_description().contains("rewrite"));
    }

    #[test]
    fn first_run_and_empty_state_copy_is_signin_first() {
        for (name, s) in [
            ("notify", setup_needed_notification_body()),
            ("stt_error", no_credentials_error()),
            ("configure", configure_auth_help()),
        ] {
            assert!(
                sign_in_comes_before_api_key(s),
                "{name} must mention sign-in/--login before API key:\n{s}"
            );
            let lower = s.to_ascii_lowercase();
            assert!(
                !lower.contains("optional"),
                "{name} must not call login optional:\n{s}"
            );
            assert!(
                !lower.starts_with("add an api key"),
                "{name} must not lead with the key:\n{s}"
            );
        }
        assert_eq!(setup_needed_notification_title(), "Set up speech access");
    }
}
