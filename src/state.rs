// ── Pure state machine ─────────────────────────────────────────
// No IO, no async. Deterministic transitions producing Command lists.
// Everything testable with assert_eq!.

use strum::Display;

#[derive(Debug, Clone, PartialEq, Eq, Display)]
pub enum AppState {
    Idle,
    Recording,
    Transcribing,
    Inserting,
    Error { message: String, recoverable: bool },
}

impl AppState {
    pub fn can_cancel(&self) -> bool {
        matches!(self, Self::Recording | Self::Transcribing | Self::Inserting)
    }

    pub fn is_active(&self) -> bool {
        !matches!(self, Self::Idle)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Toggle,
    ToggleTray,
    Cancel,
    AudioCaptured { bytes: Vec<u8>, duration_ms: u64 },
    TranscriptReady(String),
    TextInserted,
    Error(String),
}

#[derive(Debug, Clone)]
pub enum Command {
    StartCapture,
    StopCapture,
    Transcribe(Vec<u8>),
    CopyToClipboard(String),
    InjectText(String),
    SetTrayState(String),
    ShowNotification { title: String, body: String },
    OpenHistory,
    Quit,
}

pub fn transition(state: &AppState, event: &Event) -> (AppState, Vec<Command>) {
    use AppState::*;
    use Command::*;

    match (state, event) {
        // ── Idle ────────────────────────────────────────────
        (Idle, Event::Toggle) | (Idle, Event::ToggleTray) => (
            Recording,
            vec![StartCapture, SetTrayState("recording".into())],
        ),
        (Idle, Event::Cancel) => (Idle, vec![]),
        (Idle, _) => (Idle, vec![]),

        // ── Recording ────────────────────────────────────────
        (Recording, Event::Toggle) | (Recording, Event::ToggleTray) => (
            Transcribing,
            vec![StopCapture, SetTrayState("transcribing".into())],
        ),
        (Recording, Event::Cancel) => (Idle, vec![StopCapture, SetTrayState("idle".into())]),
        (Recording, Event::Error(ref msg)) => (
            Idle,
            vec![
                StopCapture,
                SetTrayState("idle".into()),
                ShowNotification {
                    title: "Recording failed".into(),
                    body: msg.clone(),
                },
            ],
        ),
        (Recording, _) => (Recording, vec![]),

        // ── Transcribing ─────────────────────────────────────
        // Ignore tray/shortcut clicks while STT runs (avoids race → idle + dropped audio).
        (Transcribing, Event::Toggle) | (Transcribing, Event::ToggleTray) => (Transcribing, vec![]),
        (Transcribing, Event::Cancel) => (Idle, vec![SetTrayState("idle".into())]),
        (Transcribing, Event::AudioCaptured { ref bytes, .. }) => {
            (Transcribing, vec![Transcribe(bytes.clone())])
        }
        (Transcribing, Event::TranscriptReady(ref text)) => {
            let t = text.clone();
            (
                Inserting,
                vec![
                    CopyToClipboard(t.clone()),
                    InjectText(t.clone()),
                    SetTrayState("inserting".into()),
                ],
            )
        }
        (Transcribing, Event::Error(ref msg)) => (
            Idle,
            vec![
                SetTrayState("idle".into()),
                ShowNotification {
                    title: "Transcription failed".into(),
                    body: msg.clone(),
                },
            ],
        ),
        (Transcribing, _) => (Transcribing, vec![]),

        // ── Inserting ────────────────────────────────────────
        (Inserting, Event::Toggle) | (Inserting, Event::ToggleTray) => {
            (Idle, vec![SetTrayState("idle".into())])
        }
        (Inserting, Event::Cancel) => (Idle, vec![SetTrayState("idle".into())]),
        (Inserting, Event::TextInserted) => (Idle, vec![SetTrayState("idle".into())]),
        (Inserting, Event::Error(ref msg)) => (
            Idle,
            vec![
                SetTrayState("idle".into()),
                ShowNotification {
                    title: "Insertion failed".into(),
                    body: format!("Text was copied but injection failed: {msg}"),
                },
            ],
        ),
        (Inserting, _) => (Inserting, vec![]),

        // ── Error ────────────────────────────────────────────
        (Error { .. }, Event::Toggle) | (Error { .. }, Event::ToggleTray) => {
            (Idle, vec![SetTrayState("idle".into())])
        }
        (Error { .. }, Event::Cancel) => (Idle, vec![SetTrayState("idle".into())]),
        (Error { .. }, Event::Error(ref msg)) => (
            Error {
                message: msg.clone(),
                recoverable: true,
            },
            vec![SetTrayState("error".into())],
        ),
        (err @ Error { .. }, _) => (err.clone(), vec![]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! assert_commands {
        ($got:expr, $expected:expr) => {
            let got: Vec<Command> = $got;
            let expected: Vec<Command> = $expected;
            assert_eq!(got.len(), expected.len(), "command count mismatch");
            for (g, e) in got.iter().zip(expected.iter()) {
                assert_eq!(
                    std::mem::discriminant(g),
                    std::mem::discriminant(e),
                    "expected {e:?} variant, got {g:?}"
                );
            }
        };
    }

    #[test]
    fn test_idle_toggle_starts_recording() {
        let (s, cmds) = transition(&AppState::Idle, &Event::Toggle);
        assert_eq!(s, AppState::Recording);
        assert_commands!(
            cmds,
            vec![Command::StartCapture, Command::SetTrayState("x".into())]
        );
    }

    #[test]
    fn test_idle_cancel_is_noop() {
        let (s, cmds) = transition(&AppState::Idle, &Event::Cancel);
        assert_eq!(s, AppState::Idle);
        assert!(cmds.is_empty());
    }

    #[test]
    fn test_idle_ignores_other_events() {
        for event in &[
            Event::AudioCaptured {
                bytes: vec![],
                duration_ms: 0,
            },
            Event::TranscriptReady("x".into()),
            Event::TextInserted,
            Event::Error("x".into()),
        ] {
            let (s, cmds) = transition(&AppState::Idle, event);
            assert_eq!(s, AppState::Idle, "unexpected transition on {event:?}");
            assert!(cmds.is_empty());
        }
    }

    #[test]
    fn test_recording_toggle_starts_transcribing() {
        let (s, cmds) = transition(&AppState::Recording, &Event::Toggle);
        assert_eq!(s, AppState::Transcribing);
        assert_commands!(
            cmds,
            vec![Command::StopCapture, Command::SetTrayState("x".into())]
        );
    }

    #[test]
    fn test_recording_cancel_goes_idle() {
        let (s, cmds) = transition(&AppState::Recording, &Event::Cancel);
        assert_eq!(s, AppState::Idle);
        assert_commands!(
            cmds,
            vec![Command::StopCapture, Command::SetTrayState("x".into())]
        );
    }

    #[test]
    fn test_recording_error_returns_idle() {
        let (s, cmds) = transition(&AppState::Recording, &Event::Error("boom".into()));
        assert_eq!(s, AppState::Idle);
        assert!(cmds.len() >= 2);
    }

    #[test]
    fn test_transcribing_toggle_is_ignored() {
        let (s, cmds) = transition(&AppState::Transcribing, &Event::Toggle);
        assert_eq!(s, AppState::Transcribing);
        assert!(cmds.is_empty());
    }

    #[test]
    fn test_transcribing_cancel_goes_idle() {
        let (s, _) = transition(&AppState::Transcribing, &Event::Cancel);
        assert_eq!(s, AppState::Idle);
    }

    #[test]
    fn test_transcribing_audio_triggers_stt() {
        let data = vec![0u8; 100];
        let (s, cmds) = transition(
            &AppState::Transcribing,
            &Event::AudioCaptured {
                bytes: data.clone(),
                duration_ms: 1000,
            },
        );
        assert_eq!(s, AppState::Transcribing);
        assert_commands!(cmds, vec![Command::Transcribe(data)]);
    }

    #[test]
    fn test_transcribing_transcript_starts_inserting() {
        let text = "hello world";
        let (s, cmds) = transition(
            &AppState::Transcribing,
            &Event::TranscriptReady(text.into()),
        );
        assert_eq!(s, AppState::Inserting);
        assert_eq!(cmds.len(), 3);
        assert!(matches!(&cmds[0], Command::CopyToClipboard(t) if t == "hello world"));
        assert!(matches!(&cmds[1], Command::InjectText(t) if t == "hello world"));
        assert!(matches!(&cmds[2], Command::SetTrayState(t) if t == "inserting"));
    }

    #[test]
    fn test_transcribing_error_returns_idle() {
        let (s, cmds) = transition(&AppState::Transcribing, &Event::Error("api fail".into()));
        assert_eq!(s, AppState::Idle);
        assert!(cmds
            .iter()
            .any(|c| matches!(c, Command::SetTrayState(s) if s == "idle")));
    }

    #[test]
    fn test_inserting_text_inserted_goes_idle() {
        let (s, cmds) = transition(&AppState::Inserting, &Event::TextInserted);
        assert_eq!(s, AppState::Idle);
        assert_commands!(cmds, vec![Command::SetTrayState("x".into())]);
    }

    #[test]
    fn test_inserting_toggle_aborts() {
        let (s, _) = transition(&AppState::Inserting, &Event::Toggle);
        assert_eq!(s, AppState::Idle);
    }

    #[test]
    fn test_inserting_cancel_aborts() {
        let (s, _) = transition(&AppState::Inserting, &Event::Cancel);
        assert_eq!(s, AppState::Idle);
    }

    #[test]
    fn test_inserting_error_returns_idle() {
        let (s, _) = transition(&AppState::Inserting, &Event::Error("inject fail".into()));
        assert_eq!(s, AppState::Idle);
    }

    #[test]
    fn test_error_toggle_recovers() {
        let e = AppState::Error {
            message: "x".into(),
            recoverable: true,
        };
        let (s, cmds) = transition(&e, &Event::Toggle);
        assert_eq!(s, AppState::Idle);
        assert_commands!(cmds, vec![Command::SetTrayState("x".into())]);
    }

    #[test]
    fn test_error_cancel_recovers() {
        let e = AppState::Error {
            message: "x".into(),
            recoverable: true,
        };
        let (s, _) = transition(&e, &Event::Cancel);
        assert_eq!(s, AppState::Idle);
    }

    #[test]
    fn test_full_cycle() {
        let state = AppState::Idle;
        let (state, _) = transition(&state, &Event::Toggle);
        assert_eq!(state, AppState::Recording);

        let (state, _) = transition(&state, &Event::Toggle);
        assert_eq!(state, AppState::Transcribing);

        let (state, _) = transition(
            &state,
            &Event::AudioCaptured {
                bytes: vec![0u8; 100],
                duration_ms: 1000,
            },
        );
        assert_eq!(state, AppState::Transcribing);

        let (state, _) = transition(&state, &Event::TranscriptReady("hello".into()));
        assert_eq!(state, AppState::Inserting);

        let (state, _) = transition(&state, &Event::TextInserted);
        assert_eq!(state, AppState::Idle);
    }

    #[test]
    fn test_can_cancel_from_active_states() {
        assert!(!AppState::Idle.can_cancel());
        assert!(AppState::Recording.can_cancel());
        assert!(AppState::Transcribing.can_cancel());
        assert!(AppState::Inserting.can_cancel());
        assert!(!AppState::Error {
            message: "x".into(),
            recoverable: true
        }
        .can_cancel());
    }

    #[test]
    fn test_idle_not_active() {
        assert!(!AppState::Idle.is_active());
        assert!(AppState::Recording.is_active());
        assert!(AppState::Transcribing.is_active());
        assert!(AppState::Inserting.is_active());
        assert!(AppState::Error {
            message: "x".into(),
            recoverable: true
        }
        .is_active());
    }
}
