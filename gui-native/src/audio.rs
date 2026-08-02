use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

struct PlayerState {
    child: Option<Child>,
    temp_wav: Option<PathBuf>,
    started_at: Option<Instant>,
    /// Playback offset at last pause / seek (seconds into the file).
    pause_offset: Duration,
    duration: Duration,
    playing: bool,
}

impl Default for PlayerState {
    fn default() -> Self {
        Self {
            child: None,
            temp_wav: None,
            started_at: None,
            pause_offset: Duration::ZERO,
            duration: Duration::ZERO,
            playing: false,
        }
    }
}

#[derive(Clone)]
pub struct AudioPlayer {
    inner: Arc<Mutex<PlayerState>>,
}

impl Default for AudioPlayer {
    fn default() -> Self {
        Self {
            inner: Arc::new(Mutex::new(PlayerState::default())),
        }
    }
}

impl AudioPlayer {
    /// True when a WAV is still on disk (safe to toggle/seek without reloading).
    pub fn has_loaded(&self) -> bool {
        self.inner
            .lock()
            .map(|s| s.temp_wav.is_some() && !s.duration.is_zero())
            .unwrap_or(false)
    }

    /// Decode PCM to a temp WAV and set duration, without starting playback.
    pub fn load_only(&self, pcm: &[u8]) -> Result<f32, String> {
        let mut state = self.inner.lock().map_err(|e| e.to_string())?;
        Self::stop_inner(&mut state, true);

        let wav = cosmic_scribe::api::encode_pcm_to_wav(pcm);
        let duration = Duration::from_secs_f32(pcm_duration_secs(pcm));
        let temp = std::env::temp_dir().join(format!("cosmic-scribe-{}.wav", std::process::id()));
        std::fs::write(&temp, &wav).map_err(|e| e.to_string())?;

        state.temp_wav = Some(temp);
        state.started_at = None;
        state.pause_offset = Duration::ZERO;
        state.duration = duration;
        state.playing = false;
        Ok(duration.as_secs_f32())
    }

    pub fn load_and_play(&self, pcm: &[u8]) -> Result<f32, String> {
        let mut state = self.inner.lock().map_err(|e| e.to_string())?;
        Self::stop_inner(&mut state, true);

        let wav = cosmic_scribe::api::encode_pcm_to_wav(pcm);
        let duration = Duration::from_secs_f32(pcm_duration_secs(pcm));
        let temp = std::env::temp_dir().join(format!("cosmic-scribe-{}.wav", std::process::id()));
        std::fs::write(&temp, &wav).map_err(|e| e.to_string())?;

        let child = spawn_player(&temp, 0.0)?;
        state.child = Some(child);
        state.temp_wav = Some(temp);
        state.started_at = Some(Instant::now());
        state.pause_offset = Duration::ZERO;
        state.duration = duration;
        state.playing = true;
        Ok(duration.as_secs_f32())
    }

    /// Pause if playing; resume (or restart from start if finished) if paused.
    pub fn toggle(&self) -> (bool, f32, f32) {
        let Ok(mut state) = self.inner.lock() else {
            return (false, 0.0, 0.0);
        };
        if state.duration.is_zero() || state.temp_wav.is_none() {
            return (false, 0.0, 0.0);
        }
        if state.playing {
            if let Some(started) = state.started_at {
                state.pause_offset += started.elapsed();
            }
            Self::kill_child(&mut state);
            state.playing = false;
            state.started_at = None;
            // Cap offset at duration
            if state.pause_offset > state.duration {
                state.pause_offset = state.duration;
            }
        } else {
            // Restart from beginning if we finished (or are at the end).
            let at_end = state.pause_offset + Duration::from_millis(80) >= state.duration;
            if at_end {
                state.pause_offset = Duration::ZERO;
            }
            if let Some(path) = state.temp_wav.clone() {
                let start = state.pause_offset.as_secs_f32();
                match spawn_player(&path, start) {
                    Ok(child) => {
                        state.child = Some(child);
                        state.started_at = Some(Instant::now());
                        state.playing = true;
                    }
                    Err(_) => {
                        // Leave paused at current offset; UI can re-load PCM.
                        state.playing = false;
                        state.started_at = None;
                    }
                }
            }
        }
        let pos = Self::position_inner(&state);
        (state.playing, pos, state.duration.as_secs_f32())
    }

    /// Stop and rewind to start (keeps loaded audio for Play again).
    pub fn stop_to_start(&self) -> (bool, f32, f32) {
        let Ok(mut state) = self.inner.lock() else {
            return (false, 0.0, 0.0);
        };
        Self::kill_child(&mut state);
        state.playing = false;
        state.started_at = None;
        state.pause_offset = Duration::ZERO;
        (false, 0.0, state.duration.as_secs_f32())
    }

    pub fn seek_fraction(&self, fraction: f32) -> (bool, f32, f32) {
        let Ok(mut state) = self.inner.lock() else {
            return (false, 0.0, 0.0);
        };
        if state.duration.is_zero() {
            return (false, 0.0, 0.0);
        }
        let frac = fraction.clamp(0.0, 1.0);
        let was_playing = state.playing;
        Self::kill_child(&mut state);
        state.pause_offset = state.duration.mul_f32(frac);
        state.started_at = None;
        state.playing = false;
        if was_playing {
            if let Some(path) = state.temp_wav.clone() {
                let start = state.pause_offset.as_secs_f32();
                if let Ok(child) = spawn_player(&path, start) {
                    state.child = Some(child);
                    state.started_at = Some(Instant::now());
                    state.playing = true;
                }
            }
        }
        let pos = Self::position_inner(&state);
        (state.playing, pos, state.duration.as_secs_f32())
    }

    pub fn position(&self) -> (bool, f32, f32) {
        let Ok(mut state) = self.inner.lock() else {
            return (false, 0.0, 0.0);
        };
        if let Some(child) = state.child.as_mut() {
            if let Ok(Some(_status)) = child.try_wait() {
                // Natural end of stream
                state.playing = false;
                state.started_at = None;
                state.child = None;
                state.pause_offset = state.duration;
            }
        }
        (
            state.playing,
            Self::position_inner(&state),
            state.duration.as_secs_f32(),
        )
    }

    pub fn stop(&self) {
        if let Ok(mut state) = self.inner.lock() {
            Self::stop_inner(&mut state, true);
        }
    }

    fn position_inner(state: &PlayerState) -> f32 {
        let mut pos = state.pause_offset;
        if state.playing {
            if let Some(started) = state.started_at {
                pos += started.elapsed();
            }
        }
        pos.as_secs_f32().min(state.duration.as_secs_f32())
    }

    fn kill_child(state: &mut PlayerState) {
        if let Some(mut child) = state.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    fn stop_inner(state: &mut PlayerState, remove_temp: bool) {
        Self::kill_child(state);
        if remove_temp {
            if let Some(path) = state.temp_wav.take() {
                let _ = std::fs::remove_file(path);
            }
            state.duration = Duration::ZERO;
        }
        state.started_at = None;
        state.pause_offset = Duration::ZERO;
        state.playing = false;
    }
}

/// Play WAV from `start_secs` into the file (seek via ffmpeg when needed).
fn spawn_player(path: &PathBuf, start_secs: f32) -> Result<Child, String> {
    if start_secs > 0.05 {
        if let Ok(child) = spawn_ffmpeg_seek(path, start_secs) {
            return Ok(child);
        }
        // Fall through: play from start (progress bar still tracks pause_offset).
    }
    for cmd in ["pw-play", "aplay", "paplay"] {
        if let Ok(child) = Command::new(cmd)
            .arg(path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            return Ok(child);
        }
    }
    Err("no audio player found (tried pw-play, aplay, paplay; seek needs ffmpeg)".into())
}

fn spawn_ffmpeg_seek(path: &PathBuf, start_secs: f32) -> Result<Child, String> {
    // ffmpeg -ss N -i file.wav -f wav - | pw-play -
    let mut ffmpeg = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-ss",
            &format!("{start_secs:.3}"),
            "-i",
            path.to_str().ok_or("path")?,
            "-f",
            "wav",
            "-",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| e.to_string())?;

    let stdout = ffmpeg.stdout.take().ok_or("ffmpeg stdout")?;
    for cmd in ["pw-play", "paplay", "aplay"] {
        let mut player = Command::new(cmd);
        if cmd == "aplay" {
            player.arg("-");
        } else {
            // pw-play / paplay read stdin when given no file args on some systems;
            // pass "-" explicitly where supported.
            player.arg("-");
        }
        if let Ok(child) = player
            .stdin(Stdio::from(stdout))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            // Detach ffmpeg: it dies when pipe closes with player
            let _ = ffmpeg;
            return Ok(child);
        }
        // stdout moved; can't retry other players without re-spawning ffmpeg
        break;
    }
    let _ = ffmpeg.kill();
    Err("could not pipe ffmpeg to audio player".into())
}

pub fn pcm_duration_secs(pcm: &[u8]) -> f32 {
    (pcm.len() / 2) as f32 / 16000.0
}
