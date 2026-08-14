//! Pure UX helpers — no iced types. Testable without a display.

use cosmic_scribe::api::RecordingVersion;

/// Whether saving an edit should create a new version.
///
/// `text_editor` often appends a trailing newline; treat end-trimmed text as equal
/// so Save does not invent a clone of the same transcript.
pub fn should_save_edit(current: &str, edited: &str) -> bool {
    normalize_edit_text(current) != normalize_edit_text(edited)
}

fn normalize_edit_text(s: &str) -> &str {
    s.trim_end_matches(['\n', '\r', ' ', '\t'])
}

/// Page content padding [top, right, bottom, left] design tokens (px).
pub fn page_padding_tokens() -> (u16, u16, u16, u16) {
    (16, 24, 16, 24)
}

/// Max description length on a segmented/toggle settings row.
#[cfg_attr(not(test), allow(dead_code))]
pub fn settings_description_ok(description: &str) -> bool {
    description.chars().count() <= 90
}

pub fn tray_state_title(state: &str) -> &'static str {
    match state {
        "recording" => "Recording",
        "recognizing" => "Recognizing",
        _ => "Idle",
    }
}

pub fn tray_state_caption(state: &str) -> &'static str {
    match state {
        "recording" => "Red — microphone is on.",
        "recognizing" => "Blue — turning speech into text.",
        _ => "Ready to record.",
    }
}

/// RGB for tray capsule legend (idle / red / blue).
pub fn tray_capsule_rgb(state: &str) -> (f32, f32, f32) {
    match state {
        "recording" => (0.90, 0.22, 0.22),
        "recognizing" => (0.25, 0.55, 0.95),
        _ => (0.85, 0.85, 0.88),
    }
}

/// User-facing label for the base STT transcript tab (index 0).
pub fn base_transcript_label() -> &'static str {
    "Transcript"
}

/// User-facing label for a stored version (edits / AI), 1-based display index.
pub fn version_tab_label(version: &RecordingVersion, display_index: usize) -> String {
    let kind = match version.version_type.as_str() {
        "user_edit" => "Your edit",
        "llm_correction" => "AI fix",
        other if other.is_empty() => "Version",
        other => other,
    };
    if display_index <= 1 {
        kind.to_string()
    } else {
        format!("{kind} {display_index}")
    }
}

/// Whether to show the version switcher at all (only if multiple sources).
pub fn show_version_switcher(version_count: usize) -> bool {
    version_count >= 1 // versions vec length; base is separate → show if versions non-empty
}

/// Header Save is shown only after config has loaded once and something is dirty.
pub fn settings_save_visible(settings_loaded: bool, dirty: bool) -> bool {
    settings_loaded && dirty
}

/// Settings dirty: any field differs from last saved snapshot.
pub fn settings_is_dirty(
    lang: &str,
    output_mode: &str,
    history_time_mode: &str,
    stt_endpoint: &str,
    api_key_typed: &str,
    saved_lang: &str,
    saved_output: &str,
    saved_time: &str,
    saved_stt_endpoint: &str,
    api_key_was_empty: bool,
    analytics_opt_in: bool,
    saved_analytics_opt_in: bool,
) -> bool {
    if lang != saved_lang {
        return true;
    }
    if output_mode != saved_output {
        return true;
    }
    if history_time_mode != saved_time {
        return true;
    }
    if stt_endpoint.trim() != saved_stt_endpoint.trim() {
        return true;
    }
    if analytics_opt_in != saved_analytics_opt_in {
        return true;
    }
    // Typed key only dirties if non-empty (blank means “keep stored”).
    if !api_key_typed.is_empty() {
        return true;
    }
    let _ = api_key_was_empty;
    false
}

pub use cosmic_scribe::product_copy::{
    access_how_speech_works, active_auth_detail, active_auth_title, analytics_item_description,
    sign_in_item_description, sign_in_item_title,
};

/// Show-more control belongs after list items when more pages exist.
pub fn show_more_after_list(has_more: bool, entry_count: usize) -> bool {
    has_more && entry_count > 0
}

/// Default history page size (initial load and “Show more”).
pub const HISTORY_PAGE_SIZE: usize = 20;

/// After a full-list probe fetch that requested `display_limit + 1` rows:
/// returns `(how_many_to_keep, has_more)`.
///
/// Poll refresh must use this (not `returned >= display_limit`) so exhausting
/// pages stays exhausted when the user already loaded every take.
pub fn history_after_probe(returned_count: usize, display_limit: usize) -> (usize, bool) {
    if returned_count > display_limit {
        (display_limit, true)
    } else {
        (returned_count, false)
    }
}

/// Whether another page may exist after a non-probe page fetch of `page_size`.
pub fn history_has_more_after_page(returned: usize, page_size: usize) -> bool {
    returned >= page_size
}

pub fn stored_api_key_status(has_stored: bool) -> &'static str {
    if has_stored {
        "Saved on this computer"
    } else {
        "None saved"
    }
}

/// List-row transcript preview: one clean line + human word-count (no character spam).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryPreview {
    pub text_line: String,
    pub stats: String,
}

/// Human-readable word count for list scanning.
///
/// Real Cosmic Scribe takes are mostly tens–hundreds of words; keep exact counts
/// under 1 000 (easy to compare), then compact SI-style for longer dictation.
pub fn format_word_count(words: usize) -> String {
    match words {
        0 => String::new(),
        1 => "1 word".into(),
        n if n < 1000 => format!("{n} words"),
        n if n < 10_000 => {
            let tenths = (n + 50) / 100; // one decimal of k
            if tenths % 10 == 0 {
                format!("{}k words", tenths / 10)
            } else {
                format!("{}.{}k words", tenths / 10, tenths % 10)
            }
        }
        n => format!("{}k words", (n + 500) / 1000),
    }
}

pub fn history_preview(text: Option<&str>, max_chars: usize) -> HistoryPreview {
    let Some(raw) = text.map(str::trim).filter(|t| !t.is_empty()) else {
        return HistoryPreview {
            text_line: "No transcript".into(),
            stats: String::new(),
        };
    };
    let words = raw.split_whitespace().count();
    let stats = format_word_count(words);

    // Single line for the list; truncate at the end so the cut is obvious.
    let collapsed = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    let text_line = if collapsed.chars().count() <= max_chars {
        collapsed
    } else {
        let take = max_chars.saturating_sub(1).max(1);
        let head: String = collapsed.chars().take(take).collect();
        format!("{head}…")
    };
    HistoryPreview { text_line, stats }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmic_scribe::api::RecordingVersion;

    #[test]
    fn save_edit_skips_identical() {
        assert!(!should_save_edit("hello", "hello"));
        assert!(!should_save_edit("hello", "hello\n"));
        assert!(!should_save_edit("hello\n", "hello"));
        assert!(should_save_edit("hello", "hello!"));
    }

    #[test]
    fn description_budget() {
        assert!(settings_description_ok("Clipboard only — you paste."));
        assert!(!settings_description_ok(&"x".repeat(100)));
    }

    #[test]
    fn tray_rgb_distinct() {
        let idle = tray_capsule_rgb("idle");
        let rec = tray_capsule_rgb("recording");
        let recg = tray_capsule_rgb("recognizing");
        assert_ne!(idle, rec);
        assert_ne!(rec, recg);
        assert!(rec.0 > rec.2);
        assert!(recg.2 > recg.0);
    }

    #[test]
    fn page_padding_symmetric_sides() {
        let (t, r, b, l) = page_padding_tokens();
        assert_eq!(t, b);
        assert_eq!(r, l);
        assert!(r >= t);
    }

    #[test]
    fn version_labels_are_human() {
        assert_eq!(base_transcript_label(), "Transcript");
        let v = RecordingVersion {
            version_type: "user_edit".into(),
            text: "x".into(),
            timestamp: None,
        };
        assert_eq!(version_tab_label(&v, 1), "Your edit");
        assert_eq!(version_tab_label(&v, 2), "Your edit 2");
        let ai = RecordingVersion {
            version_type: "llm_correction".into(),
            text: "y".into(),
            timestamp: None,
        };
        assert_eq!(version_tab_label(&ai, 1), "AI fix");
    }

    #[test]
    fn version_switcher_hidden_without_edits() {
        assert!(!show_version_switcher(0));
        assert!(show_version_switcher(1));
    }

    #[test]
    fn settings_dirty_detects_mode_change() {
        let ep = "https://api.x.ai/v1/stt";
        assert!(!settings_is_dirty(
            "en",
            "clipboard",
            "relative",
            ep,
            "",
            "en",
            "clipboard",
            "relative",
            ep,
            true,
            false,
            false
        ));
        assert!(settings_is_dirty(
            "en",
            "wtype",
            "relative",
            ep,
            "",
            "en",
            "clipboard",
            "relative",
            ep,
            true,
            false,
            false
        ));
        assert!(settings_is_dirty(
            "en",
            "clipboard",
            "relative",
            ep,
            "sk-secret",
            "en",
            "clipboard",
            "relative",
            ep,
            true,
            false,
            false
        ));
        assert!(settings_is_dirty(
            "en",
            "clipboard",
            "relative",
            "http://localhost:8080/v1/stt",
            "",
            "en",
            "clipboard",
            "relative",
            ep,
            true,
            false,
            false
        ));
        assert!(settings_is_dirty(
            "en",
            "clipboard",
            "relative",
            ep,
            "",
            "en",
            "clipboard",
            "relative",
            ep,
            true,
            true,
            false
        ));
    }

    #[test]
    fn sign_in_is_ordinary_path_not_optional_hero() {
        assert_eq!(sign_in_item_title(), "Sign in");
        assert!(!sign_in_item_title()
            .to_ascii_lowercase()
            .contains("optional"));
        assert!(sign_in_item_description().contains("SuperGrok"));
        assert!(settings_description_ok(sign_in_item_description()));
        let access = access_how_speech_works();
        assert!(access.starts_with("Sign in"));
        assert!(access.contains("API key"));
        assert!(settings_description_ok(access));
        let none = active_auth_detail("none").to_ascii_lowercase();
        let sign_at = none.find("sign in").expect("sign in first");
        let key_at = none.find("api key").expect("api key second");
        assert!(sign_at < key_at);
        assert!(settings_description_ok(analytics_item_description()));
        assert!(analytics_item_description()
            .to_ascii_lowercase()
            .contains("off by default"));
    }

    #[test]
    fn settings_save_hidden_until_loaded_and_dirty() {
        // Skeptic: clean --settings must not show active Save before/without dirty prefs.
        assert!(!settings_save_visible(false, true));
        assert!(!settings_save_visible(false, false));
        assert!(!settings_save_visible(true, false));
        assert!(settings_save_visible(true, true));
    }

    #[test]
    fn show_more_only_with_items_and_has_more() {
        assert!(show_more_after_list(true, 20));
        assert!(!show_more_after_list(true, 0));
        assert!(!show_more_after_list(false, 20));
    }

    #[test]
    fn history_probe_preserves_exhausted_state() {
        // User loaded 25 total (page 20 + 5); refresh requests display_limit=25, probe=26.
        // API returns 25 → no more; Show more must stay hidden.
        let (keep, has_more) = history_after_probe(25, 25);
        assert_eq!(keep, 25);
        assert!(!has_more);
        assert!(!show_more_after_list(has_more, keep));

        // Exactly one more exists beyond the displayed window.
        let (keep, has_more) = history_after_probe(26, 25);
        assert_eq!(keep, 25);
        assert!(has_more);
        assert!(show_more_after_list(has_more, keep));
    }

    #[test]
    fn history_probe_initial_page() {
        // Initial load display_limit=20, probe returns 21 → has more, keep 20.
        let (keep, has_more) = history_after_probe(21, HISTORY_PAGE_SIZE);
        assert_eq!(keep, 20);
        assert!(has_more);

        // Fewer than a page total.
        let (keep, has_more) = history_after_probe(7, HISTORY_PAGE_SIZE);
        assert_eq!(keep, 7);
        assert!(!has_more);
    }

    #[test]
    fn history_page_append_has_more() {
        assert!(history_has_more_after_page(20, HISTORY_PAGE_SIZE));
        assert!(!history_has_more_after_page(5, HISTORY_PAGE_SIZE));
        assert!(!history_has_more_after_page(0, HISTORY_PAGE_SIZE));
    }

    #[test]
    fn auth_labels_are_clear() {
        assert_eq!(active_auth_title("oauth"), "Signed in");
        assert!(active_auth_detail("none")
            .to_lowercase()
            .contains("api key"));
        assert_eq!(stored_api_key_status(true), "Saved on this computer");
        assert_eq!(stored_api_key_status(false), "None saved");
        // Keep status lines plain — no sales / fee phrasing, no brand spam.
        let oauth = active_auth_detail("oauth").to_lowercase();
        assert!(!oauth.contains("fee"));
        assert!(!oauth.contains("extra"));
        assert!(!oauth.contains("free"));
        assert!(!oauth.contains("subscribe"));
        assert!(!oauth.contains("grok"));
        assert!(!oauth.contains("xai"));
    }

    #[test]
    fn history_preview_end_ellipsis_and_stats() {
        let empty = history_preview(None, 40);
        assert_eq!(empty.text_line, "No transcript");
        assert!(empty.stats.is_empty());

        let short = history_preview(Some("hello world"), 40);
        assert_eq!(short.text_line, "hello world");
        assert_eq!(short.stats, "2 words");
        assert!(!short.stats.contains("character"));

        let long = "word ".repeat(30);
        let prev = history_preview(Some(&long), 20);
        assert!(prev.text_line.ends_with('…'), "got {}", prev.text_line);
        assert_eq!(prev.stats, "30 words");
    }

    #[test]
    fn format_word_count_human() {
        assert_eq!(format_word_count(0), "");
        assert_eq!(format_word_count(1), "1 word");
        assert_eq!(format_word_count(44), "44 words"); // typical real take
        assert_eq!(format_word_count(250), "250 words");
        assert_eq!(format_word_count(1000), "1k words");
        assert_eq!(format_word_count(1200), "1.2k words");
        assert_eq!(format_word_count(15_000), "15k words");
    }
}
