//! Structural checks on the live GitHub landing page.

const LIVE: &str = include_str!("../README.md");

fn first_n_lines(md: &str, n: usize) -> &str {
    let mut seen = 0;
    for (i, ch) in md.char_indices() {
        if ch == '\n' {
            seen += 1;
            if seen >= n {
                return &md[..=i];
            }
        }
    }
    md
}

fn shipped_cell(md: &str) -> Option<&str> {
    for line in md.lines() {
        let t = line.trim();
        if t.starts_with('|') && t.to_ascii_lowercase().contains("**shipped**") {
            let parts: Vec<&str> = t
                .split('|')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .collect();
            return parts.get(1).copied();
        }
    }
    None
}

#[test]
fn live_readme_is_oauth_first_system_dictation() {
    let fold = first_n_lines(LIVE, 20);
    let lower = fold.to_ascii_lowercase();
    assert!(
        fold.contains("SuperGrok"),
        "live README fold must name SuperGrok:\n{fold}"
    );
    assert!(
        lower.contains("x premium") || fold.contains("X Premium"),
        "live README fold must name X Premium+:\n{fold}"
    );
    assert!(
        fold.contains("--login") || lower.contains("sign in"),
        "live README fold must show login / sign-in:\n{fold}"
    );
    assert!(
        !lower.contains("bearer **api key**") && !fold.contains("Optional plan sign-in"),
        "live README must not still lead with API-key-first copy:\n{fold}"
    );
    assert!(
        LIVE.contains("src=\"assets/") || LIVE.contains("src=\"assets/logo"),
        "live README image paths must work from the repo root"
    );
    assert!(LIVE.contains("screenshots/tray-idle.png"));
    assert!(!LIVE.contains("../../assets/"));

    let cell = shipped_cell(LIVE).expect("live README Status Shipped row");
    let cell_l = cell.to_ascii_lowercase();
    for needle in ["whisper", "stream", "pause", "karaoke", "copr"] {
        assert!(
            !cell_l.contains(needle),
            "live Shipped must not claim {needle:?}: {cell}"
        );
    }
    assert!(LIVE.contains("F13") || LIVE.contains("silence"));
    assert!(LIVE.contains("F7") || LIVE.contains("other speech"));
    assert!(LIVE.contains("F14") || LIVE.contains("punctuation"));
    assert!(LIVE.contains("**Not this app**") || LIVE.contains("**Not the product**"));
    assert!(LIVE.contains("## Quick start") || LIVE.contains("## Install"));
    assert!(LIVE.contains("cosmic-scribe --uninstall"));
}
