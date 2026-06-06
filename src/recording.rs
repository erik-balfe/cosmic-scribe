//! Recording filename helpers and junk-artifact detection (test fixtures, too-short orphans).

use crate::audio_validation::MIN_DURATION_MS;
use std::path::{Path, PathBuf};

/// Orphan clips shorter than this (no `.txt` / `.stt.json`) are accidental noise.
pub const ORPHAN_MAX_MS: u64 = 4_000;

/// Parse duration from stem like `2026-06-06_22-11-27_2000ms`.
pub fn duration_ms_from_stem(stem: &str) -> Option<u64> {
    let part = stem.rsplit('_').next()?;
    part.strip_suffix("ms")?.parse().ok()
}

fn has_transcript_artifacts(raw_path: &Path) -> bool {
    raw_path.with_extension("txt").is_file() || raw_path.with_extension("stt.json").is_file()
}

/// Test / invalid artifacts that should not appear in history.
pub fn is_junk_recording(raw_path: &Path) -> bool {
    let stem = match raw_path.file_stem().and_then(|s| s.to_str()) {
        Some(s) => s,
        None => return true,
    };
    let Some(dur_ms) = duration_ms_from_stem(stem) else {
        return true;
    };

    if dur_ms < MIN_DURATION_MS {
        return true;
    }

    if has_transcript_artifacts(raw_path) {
        return false;
    }

    // Saved but never transcribed — drop short accidental / test clips.
    if dur_ms < ORPHAN_MAX_MS {
        return true;
    }

    // `cargo test` fixture: 2000ms silence, 64000 bytes, no transcript.
    if dur_ms == 2000 {
        if let Ok(meta) = std::fs::metadata(raw_path) {
            if meta.len() == 64_000 {
                return true;
            }
        }
    }

    false
}

fn recording_sibling_paths(raw_path: &Path) -> Vec<PathBuf> {
    let stem = raw_path.with_extension("");
    let stem_str = stem.to_string_lossy();
    vec![
        raw_path.to_path_buf(),
        stem.with_extension("txt"),
        stem.with_extension("json"),
        PathBuf::from(format!("{stem_str}.stt.json")),
    ]
}

pub fn remove_recording_files(raw_path: &Path) {
    for p in recording_sibling_paths(raw_path) {
        if p.exists() {
            let _ = std::fs::remove_file(p);
        }
    }
}

/// Remove junk `.raw` and sidecar files. Returns number of recordings removed.
pub fn prune_junk_recordings(dir: &Path) -> usize {
    let Ok(read_dir) = std::fs::read_dir(dir) else {
        return 0;
    };
    let mut removed = 0;
    for entry in read_dir.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("raw") {
            continue;
        }
        if !is_junk_recording(&path) {
            continue;
        }
        remove_recording_files(&path);
        removed += 1;
    }
    removed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn junk_detects_short_and_test_fixture() {
        let dir = std::env::temp_dir().join(format!("cs-junk-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let short = dir.join("2026-06-06_12-00-00_100ms.raw");
        let fixture = dir.join("2026-06-06_12-00-01_2000ms.raw");
        let orphan = dir.join("2026-06-06_12-00-02_1053ms.raw");
        let kept = dir.join("2026-06-06_12-00-03_12000ms.raw");
        std::fs::write(&short, [0u8; 100]).unwrap();
        std::fs::write(&fixture, vec![0u8; 64_000]).unwrap();
        std::fs::write(&orphan, vec![0u8; 28_000]).unwrap();
        std::fs::write(&kept, vec![0u8; 120_000]).unwrap();
        assert!(is_junk_recording(&short));
        assert!(is_junk_recording(&fixture));
        assert!(is_junk_recording(&orphan));
        assert!(!is_junk_recording(&kept));
        let _ = std::fs::remove_dir_all(dir);
    }
}
