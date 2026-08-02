// ── Audio capture ─────────────────────────────────────────────
// Two implementations:
//   SubprocessCapture — live recording via arecord + progressive Opus
//   FileAudioCapture — reads pre-recorded file (for testing)
//
// Progressive Opus: while arecord runs, PCM is tee'd into a long-lived
// ffmpeg Opus encoder. On stop we only finalize the encoder (~ms–hundreds
// of ms), so STT upload does not wait for a full post-hoc encode of a
// multi-minute take.

use crate::traits::{AudioCapture, AudioData, PreEncodedAudio};
use anyhow::{Context, Result};
use std::io::{Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

/// Speech Opus bitrate for progressive STT upload (xAI accepts OGG/Opus).
const OPUS_BITRATE: &str = "24k";

// ── File-based capture (for testing) ──────────────────────────

pub struct FileAudioCapture {
    path: std::path::PathBuf,
}

impl FileAudioCapture {
    pub fn new(path: std::path::PathBuf) -> Self {
        Self { path }
    }
}

#[async_trait::async_trait]
impl AudioCapture for FileAudioCapture {
    async fn start(&mut self) -> Result<()> {
        tracing::info!("file capture: reading from {}", self.path.display());
        Ok(())
    }

    async fn stop(&mut self) -> Result<AudioData> {
        let bytes = std::fs::read(&self.path)
            .with_context(|| format!("failed to read {}", self.path.display()))?;

        let duration_ms = crate::audio_validation::duration_ms_from_pcm(&bytes);

        tracing::info!("file capture: {} bytes, ~{}ms", bytes.len(), duration_ms);

        Ok(AudioData::pcm(bytes, 16000, 1, duration_ms))
    }
}

// ── Live subprocess capture ───────────────────────────────────

pub struct SubprocessCapture {
    /// arecord process
    child: Option<Child>,
    /// ffmpeg progressive Opus encoder (stdin fed during capture)
    encoder: Option<Child>,
    buf: Arc<Mutex<Vec<u8>>>,
    /// Accumulated OGG/Opus from ffmpeg stdout (filled while recording)
    encoded: Arc<Mutex<Vec<u8>>>,
    /// Tee: arecord → PCM buf + ffmpeg stdin
    reader: Option<JoinHandle<()>>,
    /// Drain ffmpeg stdout into `encoded` (avoids pipe deadlock)
    enc_reader: Option<JoinHandle<()>>,
    start_time: Option<std::time::Instant>,
    progressive: bool,
}

impl Default for SubprocessCapture {
    fn default() -> Self {
        Self::new()
    }
}

impl SubprocessCapture {
    pub fn new() -> Self {
        Self {
            child: None,
            encoder: None,
            buf: Arc::new(Mutex::new(Vec::new())),
            encoded: Arc::new(Mutex::new(Vec::new())),
            reader: None,
            enc_reader: None,
            start_time: None,
            progressive: false,
        }
    }

    /// Start ffmpeg Opus encoder that reads s16le PCM from stdin.
    fn spawn_opus_encoder() -> Result<(Child, ChildStdin, std::process::ChildStdout)> {
        let mut enc = Command::new("ffmpeg")
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-f",
                "s16le",
                "-ar",
                "16000",
                "-ac",
                "1",
                "-i",
                "pipe:0",
                "-c:a",
                "libopus",
                "-b:a",
                OPUS_BITRATE,
                "-application",
                "voip",
                "-f",
                "ogg",
                "pipe:1",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .context("spawn ffmpeg progressive Opus")?;

        let stdin = enc.stdin.take().context("ffmpeg stdin")?;
        let stdout = enc.stdout.take().context("ffmpeg stdout")?;
        Ok((enc, stdin, stdout))
    }
}

#[async_trait::async_trait]
impl AudioCapture for SubprocessCapture {
    async fn start(&mut self) -> Result<()> {
        let mut child = Command::new("arecord")
            .args(["-r", "16000", "-c", "1", "-f", "S16_LE", "-t", "raw"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .context("failed to start arecord. Install: sudo dnf install alsa-utils")?;

        let stdout = child.stdout.take().context("arecord stdout not piped")?;
        let buf = self.buf.clone();
        buf.lock().unwrap().clear();
        self.encoded.lock().unwrap().clear();

        // Progressive Opus: optional — if ffmpeg fails, capture PCM only and encode after stop.
        let progressive = match Self::spawn_opus_encoder() {
            Ok((enc, enc_stdin, enc_stdout)) => {
                let encoded = self.encoded.clone();
                let enc_reader = std::thread::Builder::new()
                    .name("opus-stdout".into())
                    .spawn(move || {
                        let mut out = enc_stdout;
                        let mut chunk = [0u8; 8192];
                        loop {
                            match out.read(&mut chunk) {
                                Ok(0) => break,
                                Ok(n) => {
                                    encoded.lock().unwrap().extend_from_slice(&chunk[..n]);
                                }
                                Err(e) => {
                                    tracing::warn!("ffmpeg stdout read error: {e}");
                                    break;
                                }
                            }
                        }
                    })
                    .context("spawn opus-stdout thread")?;

                let reader = std::thread::Builder::new()
                    .name("arecord-tee".into())
                    .spawn(move || {
                        let mut reader = std::io::BufReader::new(stdout);
                        let mut enc_in = enc_stdin;
                        let mut chunk = [0u8; 8192];
                        loop {
                            match reader.read(&mut chunk) {
                                Ok(0) => break,
                                Ok(n) => {
                                    buf.lock().unwrap().extend_from_slice(&chunk[..n]);
                                    // Best-effort feed encoder; if it dies, keep capturing PCM.
                                    if let Err(e) = enc_in.write_all(&chunk[..n]) {
                                        tracing::warn!(
                                            "progressive Opus stdin write failed: {e} (PCM still saved)"
                                        );
                                        // Drain remaining PCM without encoder.
                                        loop {
                                            match reader.read(&mut chunk) {
                                                Ok(0) => break,
                                                Ok(n2) => {
                                                    buf.lock().unwrap().extend_from_slice(&chunk[..n2]);
                                                }
                                                Err(_) => break,
                                            }
                                        }
                                        break;
                                    }
                                }
                                Err(e) => {
                                    tracing::warn!("arecord read error: {e}");
                                    break;
                                }
                            }
                        }
                        // EOF to ffmpeg so it can finalize the OGG stream.
                        drop(enc_in);
                    })
                    .context("spawn arecord-tee thread")?;

                self.encoder = Some(enc);
                self.enc_reader = Some(enc_reader);
                self.reader = Some(reader);
                true
            }
            Err(e) => {
                tracing::warn!(
                    "progressive Opus unavailable ({e}); will encode after stop if needed"
                );
                let reader = std::thread::spawn(move || {
                    let mut reader = std::io::BufReader::new(stdout);
                    let mut chunk = [0u8; 8192];
                    loop {
                        match reader.read(&mut chunk) {
                            Ok(0) => break,
                            Ok(n) => {
                                buf.lock().unwrap().extend_from_slice(&chunk[..n]);
                            }
                            Err(e) => {
                                tracing::warn!("arecord read error: {e}");
                                break;
                            }
                        }
                    }
                });
                self.reader = Some(reader);
                false
            }
        };

        self.child = Some(child);
        self.progressive = progressive;
        self.start_time = Some(std::time::Instant::now());

        if progressive {
            tracing::info!("audio capture started (arecord 16kHz mono + progressive Opus)");
        } else {
            tracing::info!("audio capture started (arecord 16kHz mono)");
        }
        Ok(())
    }

    async fn stop(&mut self) -> Result<AudioData> {
        let t_stop = std::time::Instant::now();

        // Stop arecord first — tee thread sees EOF and closes ffmpeg stdin.
        if let Some(ref mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }

        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }

        // Encoder finishes after stdin EOF; wait for stdout drain then process exit.
        if let Some(enc_reader) = self.enc_reader.take() {
            let _ = enc_reader.join();
        }
        if let Some(mut enc) = self.encoder.take() {
            let _ = enc.wait();
        }

        let finalize_ms = t_stop.elapsed().as_millis() as u64;

        let wall_ms = self
            .start_time
            .take()
            .map(|t| t.elapsed().as_millis() as u64)
            .unwrap_or(0);

        let bytes = std::mem::take(&mut *self.buf.lock().unwrap());
        let duration_ms = crate::audio_validation::duration_ms_from_pcm(&bytes).max(wall_ms);

        let expected = wall_ms as usize * 32;
        if bytes.len() < expected / 2 {
            tracing::warn!(
                "audio truncated: {} bytes, expected ~{} bytes for {}ms — check mic",
                bytes.len(),
                expected,
                wall_ms,
            );
        }

        let encoded = std::mem::take(&mut *self.encoded.lock().unwrap());
        let progressive = self.progressive;
        self.progressive = false;

        let pre_encoded = if progressive
            && !encoded.is_empty()
            && (encoded.len() + 256 < bytes.len() || bytes.len() < 4096)
        {
            tracing::info!(
                pcm_bytes = bytes.len(),
                opus_bytes = encoded.len(),
                duration_ms,
                finalize_ms,
                ratio = format!("{:.1}x", bytes.len() as f64 / encoded.len().max(1) as f64),
                "progressive Opus ready (encode ran during capture)"
            );
            Some(PreEncodedAudio {
                bytes: encoded,
                file_name: "recording.ogg".into(),
                mime: "audio/ogg".into(),
                codec: "opus-progressive".into(),
            })
        } else {
            if progressive {
                tracing::warn!(
                    opus_bytes = encoded.len(),
                    pcm_bytes = bytes.len(),
                    "progressive Opus empty or not smaller; STT will encode after stop"
                );
            }
            tracing::info!(
                "audio captured: {}ms, {} bytes (finalize {}ms)",
                duration_ms,
                bytes.len(),
                finalize_ms
            );
            None
        };

        Ok(AudioData {
            bytes,
            sample_rate: 16000,
            channels: 1,
            duration_ms,
            pre_encoded,
        })
    }

    fn monitor_buffer(&self) -> Option<std::sync::Arc<std::sync::Mutex<Vec<u8>>>> {
        if self.child.is_some() {
            Some(self.buf.clone())
        } else {
            None
        }
    }
}
