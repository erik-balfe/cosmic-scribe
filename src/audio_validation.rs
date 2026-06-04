// ── Audio validation ───────────────────────────────────────────
// Pure functions. No IO. Testable independently.
//
// Handles:
//   - Empty audio (0 bytes)
//   - Too short (< MIN_DURATION_MS)
//   - Too long (> MAX_FILE_BYTES for API limits)
//
// Called in execute_commands to reject bad audio before STT.

/// Minimum audio duration in milliseconds before we bother transcribing.
pub const MIN_DURATION_MS: u64 = 500;

/// Maximum audio file size for xAI STT API (500 MB).
/// In practice, for min quality PCM16 16kHz mono: 32000 bytes/s
/// A 60-second recording = ~1.9 MB. Keep a generous limit.
pub const MAX_FILE_BYTES: u64 = 50 * 1024 * 1024; // 50 MB

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

/// Validate audio data. Returns Ok if valid, Err with description if not.
pub fn validate_audio(bytes: &[u8], duration_ms: u64) -> Result<(), ValidationError> {
    if bytes.is_empty() {
        return Err(ValidationError::Empty);
    }

    if duration_ms < MIN_DURATION_MS {
        return Err(ValidationError::TooShort {
            duration_ms,
            threshold_ms: MIN_DURATION_MS,
        });
    }

    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err(ValidationError::TooLong {
            bytes: bytes.len() as u64,
            max_bytes: MAX_FILE_BYTES,
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_audio() {
        let err = validate_audio(&[], 0).unwrap_err();
        assert_eq!(err, ValidationError::Empty);
        assert_eq!(err.to_string(), "no audio captured");
    }

    #[test]
    fn test_too_short() {
        let err = validate_audio(&[0u8; 3200], 100).unwrap_err();
        assert!(matches!(err, ValidationError::TooShort { .. }));
        assert!(err.to_string().contains("100ms"));
    }

    #[test]
    fn test_valid_audio() {
        assert!(validate_audio(&[0u8; 3200], 600).is_ok());
    }

    #[test]
    fn test_boundary_short() {
        // exactly at threshold
        assert!(validate_audio(&[0u8; 3200], MIN_DURATION_MS).is_ok());
        // just below threshold
        assert!(validate_audio(&[0u8; 3200], MIN_DURATION_MS - 1).is_err());
    }

    #[test]
    fn test_too_long() {
        let huge = vec![0u8; (MAX_FILE_BYTES + 1).try_into().unwrap()];
        let err = validate_audio(&huge, 1000000).unwrap_err();
        assert!(matches!(err, ValidationError::TooLong { .. }));
    }
}
