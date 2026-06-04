// ── Audio capture ─────────────────────────────────────────────
// Two implementations:
//   SubprocessCapture — live recording via arecord
//   FileAudioCapture — reads pre-recorded file (for testing)

use crate::traits::{AudioCapture, AudioData};
use anyhow::{Context, Result};
use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

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

        // 16kHz mono PCM16: 32000 bytes/s
        let duration_ms = (bytes.len() as f64 / 32.0) as u64;

        tracing::info!("file capture: {} bytes, ~{}ms", bytes.len(), duration_ms);

        Ok(AudioData {
            bytes,
            sample_rate: 16000,
            channels: 1,
            duration_ms,
        })
    }
}

// ── Live subprocess capture ───────────────────────────────────

pub struct SubprocessCapture {
    child: Option<Child>,
    buf: Arc<Mutex<Vec<u8>>>,
    reader: Option<JoinHandle<()>>,
    start_time: Option<std::time::Instant>,
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
            buf: Arc::new(Mutex::new(Vec::new())),
            reader: None,
            start_time: None,
        }
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

        self.child = Some(child);
        self.reader = Some(reader);
        self.start_time = Some(std::time::Instant::now());

        tracing::info!("audio capture started (arecord 16kHz mono)");
        Ok(())
    }

    async fn stop(&mut self) -> Result<AudioData> {
        if let Some(ref mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }

        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }

        let duration_ms = self
            .start_time
            .take()
            .map(|t| t.elapsed().as_millis() as u64)
            .unwrap_or(0);

        let bytes = std::mem::take(&mut *self.buf.lock().unwrap());

        let expected = duration_ms as usize * 32;
        if bytes.len() < expected / 2 {
            tracing::warn!(
                "audio truncated: {} bytes, expected ~{} bytes for {}ms — check mic",
                bytes.len(),
                expected,
                duration_ms,
            );
        }

        tracing::info!("audio captured: {}ms, {} bytes", duration_ms, bytes.len());

        Ok(AudioData {
            bytes,
            sample_rate: 16000,
            channels: 1,
            duration_ms,
        })
    }
}
