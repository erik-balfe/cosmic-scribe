// ── Audio validation ───────────────────────────────────────────
// Pure functions. No IO. Testable independently.
//
// Handles (hard reject only when we are sure):
//   - Empty audio (0 bytes)
//   - Too short (< MIN_DURATION_MS)
//   - Too long (> MAX_FILE_BYTES for API limits)
//
// We do **not** hard-reject on RMS “silence”. Soft speech, quiet mics, and
// pauses look like silence and must not discard real takes. Device mute /
// 0% volume is a separate best-effort warning via pactl (not VAD).

/// Minimum audio duration in milliseconds before we bother transcribing.
pub const MIN_DURATION_MS: u64 = 500;

/// Maximum audio file size for xAI STT API (500 MB).
/// In practice, for min quality PCM16 16kHz mono: 32000 bytes/s
/// A 60-second recording = ~1.9 MB. Keep a generous limit.
pub const MAX_FILE_BYTES: u64 = 50 * 1024 * 1024; // 50 MB

/// PCM16 mono 16 kHz: bytes per millisecond.
pub const PCM_BYTES_PER_MS: f64 = 32.0;

/// RMS scale helper (0.0–1.0). Kept for diagnostics / future soft UI hints only.
pub const SILENCE_RMS_THRESHOLD: f64 = 0.008;

/// Minimum bytes before RMS is meaningful (~1 s at 16 kHz mono S16).
pub const SILENCE_CHECK_MIN_BYTES: usize = 32_000;

#[derive(Debug, Clone, PartialEq)]
pub enum ValidationError {
    Empty,
    TooShort { duration_ms: u64, threshold_ms: u64 },
    TooLong { bytes: u64, max_bytes: u64 },
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "no audio captured"),
            Self::TooShort {
                duration_ms,
                threshold_ms,
            } => {
                write!(
                    f,
                    "recording too short: {duration_ms}ms (min {threshold_ms}ms)"
                )
            }
            Self::TooLong { bytes, max_bytes } => {
                write!(f, "audio too large: {} bytes (max {})", bytes, max_bytes)
            }
        }
    }
}

/// Duration implied by PCM byte length (16 kHz mono S16_LE).
pub fn duration_ms_from_pcm(bytes: &[u8]) -> u64 {
    (bytes.len() as f64 / PCM_BYTES_PER_MS).round() as u64
}

/// Root-mean-square level of PCM16 mono audio (0.0–1.0).
pub fn pcm_rms(bytes: &[u8]) -> f64 {
    if bytes.len() < 2 {
        return 0.0;
    }
    let sum_sq: f64 = bytes
        .chunks_exact(2)
        .map(|c| {
            let s = i16::from_le_bytes([c[0], c[1]]) as f64 / 32768.0;
            s * s
        })
        .sum();
    let n = bytes.len() / 2;
    (sum_sq / n as f64).sqrt()
}

/// True when the buffer is long enough and RMS is below threshold.
///
/// **Do not use this to abort recording or discard takes.** Quiet mics and
/// pauses false-positive. Prefer device mute/volume warnings instead.
pub fn is_silent(bytes: &[u8]) -> bool {
    bytes.len() >= SILENCE_CHECK_MIN_BYTES && pcm_rms(bytes) < SILENCE_RMS_THRESHOLD
}

/// Human-readable duration label from milliseconds (e.g. `6s`, `2m 15s`).
pub fn format_duration_ms(ms: u64) -> String {
    let secs = (ms as f64 / 1000.0).round() as u64;
    if secs < 1 {
        return "<1s".into();
    }
    if secs < 60 {
        return format!("{secs}s");
    }
    let m = secs / 60;
    let s = secs % 60;
    if m < 60 {
        return if s == 0 {
            format!("{m}m")
        } else {
            format!("{m}m {s}s")
        };
    }
    let h = m / 60;
    let rm = m % 60;
    if rm > 0 && s > 0 {
        format!("{h}h {rm}m {s}s")
    } else if rm > 0 {
        format!("{h}h {rm}m")
    } else {
        format!("{h}h")
    }
}

/// Validate audio data. Returns Ok if valid, Err with description if not.
pub fn validate_audio(bytes: &[u8], duration_ms: u64) -> Result<(), ValidationError> {
    if bytes.is_empty() {
        return Err(ValidationError::Empty);
    }

    let effective_ms = duration_ms_from_pcm(bytes).max(duration_ms);

    if effective_ms < MIN_DURATION_MS {
        return Err(ValidationError::TooShort {
            duration_ms: effective_ms,
            threshold_ms: MIN_DURATION_MS,
        });
    }

    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err(ValidationError::TooLong {
            bytes: bytes.len() as u64,
            max_bytes: MAX_FILE_BYTES,
        });
    }

    // No RMS / “silence” hard reject — false positives discard real dictation.

    Ok(())
}

/// Best-effort check of default capture device mute/volume (PulseAudio/PipeWire).
pub fn mic_capture_warning() -> Option<String> {
    let mute = std::process::Command::new("pactl")
        .args(["get-source-mute", "@DEFAULT_SOURCE@"])
        .output()
        .ok()?;
    if String::from_utf8_lossy(&mute.stdout).contains("yes") {
        return Some("Microphone is muted — unmute before dictating".into());
    }

    let vol = std::process::Command::new("pactl")
        .args(["get-source-volume", "@DEFAULT_SOURCE@"])
        .output()
        .ok()?;
    let line = String::from_utf8_lossy(&vol.stdout);
    for token in line.split_whitespace() {
        if let Some(pct) = token.strip_suffix('%') {
            if let Ok(n) = pct.parse::<u32>() {
                if n == 0 {
                    return Some(
                        "Microphone input volume is 0% — raise it before dictating".into(),
                    );
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone_pcm(num_bytes: usize) -> Vec<u8> {
        let samples = num_bytes / 2;
        (0..samples)
            .flat_map(|i| {
                let s = (i as i32 * 7919 % 8000 + 500) as i16;
                s.to_le_bytes()
            })
            .collect()
    }

    #[test]
    fn test_empty_audio() {
        let err = validate_audio(&[], 0).unwrap_err();
        assert_eq!(err, ValidationError::Empty);
        assert_eq!(err.to_string(), "no audio captured");
    }

    #[test]
    fn test_too_short() {
        let err = validate_audio(&tone_pcm(3200), 100).unwrap_err();
        assert!(matches!(err, ValidationError::TooShort { .. }));
        assert!(err.to_string().contains("100ms"));
    }

    #[test]
    fn test_valid_audio() {
        assert!(validate_audio(&tone_pcm(3200), 600).is_ok());
    }

    #[test]
    fn test_boundary_short() {
        assert!(validate_audio(&tone_pcm(3200), MIN_DURATION_MS).is_ok());
        assert!(validate_audio(&tone_pcm(3200), MIN_DURATION_MS - 1).is_err());
    }

    #[test]
    fn test_too_long() {
        let huge = vec![0u8; (MAX_FILE_BYTES + 1).try_into().unwrap()];
        let err = validate_audio(&huge, 1000000).unwrap_err();
        assert!(matches!(err, ValidationError::TooLong { .. }));
    }

    #[test]
    fn test_quiet_pcm_not_rejected() {
        // All-zero buffer is long enough for a take — must still pass validation
        // so quiet mics / pauses never discard audio before STT.
        let quiet = vec![0u8; 64_000];
        assert!(validate_audio(&quiet, 2000).is_ok());
        assert!(is_silent(&quiet)); // diagnostic only
    }

    #[test]
    fn test_pcm_rms_and_duration() {
        assert!(pcm_rms(&[0u8; 4000]) < SILENCE_RMS_THRESHOLD);
        assert!(pcm_rms(&tone_pcm(4000)) > SILENCE_RMS_THRESHOLD);
        assert_eq!(duration_ms_from_pcm(&tone_pcm(64_000)), 2000);
        assert_eq!(format_duration_ms(6500), "7s");
        assert_eq!(format_duration_ms(6000), "6s");
    }
}
