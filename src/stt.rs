// ── Cloud STT client (xAI REST dialect by default) ────────────
// Default: POST https://api.x.ai/v1/stt  (Bearer key or plan OAuth)
// Endpoint is configurable (`get_stt_endpoint`) for same-dialect
// proxies / self-hosted mirrors — **not** a full OpenAI swap.
//
// OpenAI Whisper uses POST /v1/audio/transcriptions + `model` field and a
// different response shape for word timings. That needs a separate dialect
// (contributor path: docs/STT_PROVIDERS.md), not only a base URL.
//
// Upload prefers Opus-in-OGG (speech bitrate) when ffmpeg is available —
// much smaller than raw PCM/WAV. Falls back to WAV if encode fails.
// Local .raw recordings stay uncompressed for playback fidelity.

use crate::traits::{AudioData, SttClient, SttResult};
use anyhow::{Context, Result};
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::Duration;

/// Speech-oriented Opus bitrate (OGG/Opus; 24k is plenty for dictation).
const OPUS_BITRATE: &str = "24k";

/// Payload prepared for multipart STT upload.
#[derive(Debug, Clone)]
pub struct UploadAudio {
    pub bytes: Vec<u8>,
    pub file_name: &'static str,
    pub mime: &'static str,
    pub codec: &'static str,
    pub pcm_bytes: usize,
}

impl UploadAudio {
    pub fn compression_ratio(&self) -> f64 {
        if self.bytes.is_empty() {
            return 0.0;
        }
        self.pcm_bytes as f64 / self.bytes.len() as f64
    }
}

pub struct XaiSttClient {
    client: reqwest::Client,
    api_key: Arc<dyn crate::traits::KeyringStore>,
    /// When set, overrides live config (tests). Otherwise re-read endpoint each request.
    base_url_override: Option<String>,
}

impl XaiSttClient {
    pub fn new(api_key: Arc<dyn crate::traits::KeyringStore>) -> Self {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(15))
            // Match app-level STT budget; long takes need upload+server headroom.
            .timeout(Duration::from_secs(180))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            client,
            api_key,
            base_url_override: None,
        }
    }

    /// Explicit endpoint (tests, or callers that already resolved config).
    pub fn with_base_url(api_key: Arc<dyn crate::traits::KeyringStore>, url: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            base_url_override: Some(url),
        }
    }

    fn endpoint(&self) -> String {
        self.base_url_override
            .clone()
            .unwrap_or_else(crate::keyring::get_stt_endpoint)
    }

    async fn resolve_bearer(&self) -> Result<String> {
        let store = self.api_key.clone();
        // Keyring may block on OAuth refresh (network). Keep it off the async worker.
        tokio::task::spawn_blocking(move || store.get_api_key())
            .await
            .context("credential task join")?
    }

    async fn post_stt(
        &self,
        bearer: &str,
        upload: &UploadAudio,
        lang: &str,
    ) -> Result<reqwest::Response> {
        let part = reqwest::multipart::Part::bytes(upload.bytes.clone())
            .file_name(upload.file_name)
            .mime_str(upload.mime)?;
        // Option fields must precede `file` (xAI multipart rule).
        let form = reqwest::multipart::Form::new()
            .text("format", "true")
            .text("language", lang.to_string())
            .part("file", part);

        let url = self.endpoint();
        self.client
            .post(&url)
            .header("Authorization", format!("Bearer {bearer}"))
            .multipart(form)
            .send()
            .await
            .context("STT request failed")
    }

    async fn parse_stt_response(response: reqwest::Response) -> Result<SttResult> {
        let status = response.status();
        let body = response
            .text()
            .await
            .context("failed to read STT response")?;
        if !status.is_success() {
            anyhow::bail!("STT API error {status}: {body}");
        }
        let json: serde_json::Value =
            serde_json::from_str(&body).context("invalid STT response JSON")?;
        SttResult::from_api_json(json).with_context(|| format!("invalid STT payload: {body}"))
    }
}

#[async_trait::async_trait]
impl SttClient for XaiSttClient {
    async fn transcribe(&self, audio: &AudioData) -> Result<SttResult> {
        use std::time::Instant;
        let t0 = Instant::now();

        let key = self.resolve_bearer().await?;
        let t_auth = t0.elapsed();

        // Encode off the async runtime (ffmpeg can take hundreds of ms on long takes).
        let audio_owned = audio.clone();
        let upload = tokio::task::spawn_blocking(move || encode_for_upload(&audio_owned))
            .await
            .context("encode task join")??;
        let t_encode = t0.elapsed();

        let lang = crate::keyring::get_language();
        tracing::info!(
            pcm_bytes = upload.pcm_bytes,
            upload_bytes = upload.bytes.len(),
            codec = upload.codec,
            ratio = format!("{:.1}x", upload.compression_ratio()),
            duration_ms = audio.duration_ms,
            auth_ms = t_auth.as_millis() as u64,
            encode_ms = (t_encode - t_auth).as_millis() as u64,
            "STT upload prep"
        );

        let t_net = Instant::now();
        let response = self.post_stt(&key, &upload, &lang).await?;
        let status = response.status();
        let upload_ms = t_net.elapsed().as_millis() as u64;

        // OAuth access tokens are short-lived; one refresh+retry on **401 only**.
        // 403 is usually plan/scope denial — refresh does not help and burns latency.
        if status.as_u16() == 401
            && crate::xai_oauth::is_logged_in()
            && crate::xai_oauth::looks_like_oauth_bearer(&key)
        {
            tracing::info!("STT got 401; trying OAuth refresh + retry");
            let new_key = tokio::task::spawn_blocking(crate::xai_oauth::force_refresh)
                .await
                .context("OAuth refresh task join")??;
            let t_retry = Instant::now();
            let response = self.post_stt(&new_key, &upload, &lang).await?;
            let result = Self::parse_stt_response(response).await;
            tracing::info!(
                upload_ms,
                retry_upload_ms = t_retry.elapsed().as_millis() as u64,
                total_ms = t0.elapsed().as_millis() as u64,
                codec = upload.codec,
                "STT finished after OAuth retry"
            );
            return result;
        }

        let result = Self::parse_stt_response(response).await;
        tracing::info!(
            upload_ms,
            total_ms = t0.elapsed().as_millis() as u64,
            ok = result.is_ok(),
            codec = upload.codec,
            upload_bytes = upload.bytes.len(),
            "STT finished"
        );
        result
    }
}

/// Prefer progressive Opus from capture; else encode after stop; else WAV.
///
/// Override: `COSMIC_SCRIBE_STT_CODEC=wav|opus` (default: opus when ffmpeg works).
pub fn encode_for_upload(audio: &AudioData) -> Result<UploadAudio> {
    let prefer_wav = crate::env_compat("COSMIC_SCRIBE_STT_CODEC", "VOICE_INPUT_STT_CODEC")
        .map(|s| s.eq_ignore_ascii_case("wav"))
        .unwrap_or(false);

    // Progressive path: encode ran during arecord — do not re-encode after stop.
    if !prefer_wav {
        if let Some(pre) = &audio.pre_encoded {
            if !pre.bytes.is_empty() {
                return Ok(UploadAudio {
                    pcm_bytes: audio.bytes.len(),
                    bytes: pre.bytes.clone(),
                    file_name: match pre.file_name.as_str() {
                        "recording.ogg" => "recording.ogg",
                        "recording.wav" => "recording.wav",
                        _ => "recording.ogg",
                    },
                    mime: match pre.mime.as_str() {
                        "audio/ogg" => "audio/ogg",
                        "audio/wav" => "audio/wav",
                        _ => "audio/ogg",
                    },
                    codec: if pre.codec.contains("progressive") {
                        "opus-progressive"
                    } else {
                        "opus"
                    },
                });
            }
        }
    }

    if !prefer_wav {
        match encode_opus_ogg(audio) {
            Ok(upload) if !upload.bytes.is_empty() => {
                // Only keep Opus if it actually shrinks (pathological tiny clips).
                if upload.bytes.len() + 256 < audio.bytes.len() {
                    return Ok(upload);
                }
                tracing::debug!(
                    opus = upload.bytes.len(),
                    pcm = audio.bytes.len(),
                    "Opus not smaller; using WAV"
                );
            }
            Ok(_) => tracing::warn!("Opus encode produced empty output; using WAV"),
            Err(e) => tracing::warn!("Opus encode failed ({e}); using WAV"),
        }
    }

    encode_wav_upload(audio)
}

fn encode_wav_upload(audio: &AudioData) -> Result<UploadAudio> {
    let bytes = encode_wav(audio)?;
    Ok(UploadAudio {
        pcm_bytes: audio.bytes.len(),
        bytes,
        file_name: "recording.wav",
        mime: "audio/wav",
        codec: "wav",
    })
}

/// Encode PCM s16le → Opus in Ogg via ffmpeg (speech bitrate).
///
/// Must pump stdin and stdout **concurrently**. Writing all PCM first then
/// reading OGG deadlocks once ffmpeg fills the OS pipe buffer (~64KiB) on
/// long takes — which matched daemon logs: no `STT upload prep` until the
/// 60s outer timeout / cancel, and missing `.txt` for ~27s+ recordings.
fn encode_opus_ogg(audio: &AudioData) -> Result<UploadAudio> {
    let rate = audio.sample_rate.to_string();
    let ch = audio.channels.to_string();

    let mut child = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "s16le",
            "-ar",
            &rate,
            "-ac",
            &ch,
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
        .stderr(Stdio::piped())
        .spawn()
        .context("spawn ffmpeg for Opus")?;

    let mut stdin = child.stdin.take().context("ffmpeg stdin")?;
    let mut stdout = child.stdout.take().context("ffmpeg stdout")?;
    let mut stderr = child.stderr.take().context("ffmpeg stderr")?;

    let pcm = audio.bytes.clone();
    let writer = std::thread::Builder::new()
        .name("opus-stdin".into())
        .spawn(move || -> Result<()> {
            stdin.write_all(&pcm).context("write PCM to ffmpeg")?;
            // Close stdin so ffmpeg sees EOF and finishes encoding.
            drop(stdin);
            Ok(())
        })
        .context("spawn opus-stdin thread")?;

    let reader = std::thread::Builder::new()
        .name("opus-stdout".into())
        .spawn(move || -> Result<Vec<u8>> {
            let mut out = Vec::new();
            stdout
                .read_to_end(&mut out)
                .context("read Opus from ffmpeg")?;
            Ok(out)
        })
        .context("spawn opus-stdout thread")?;

    let mut err_buf = Vec::new();
    let _ = stderr.read_to_end(&mut err_buf);

    let write_res = writer
        .join()
        .map_err(|_| anyhow::anyhow!("opus-stdin thread panicked"))?;
    write_res?;

    let bytes = reader
        .join()
        .map_err(|_| anyhow::anyhow!("opus-stdout thread panicked"))??;

    let status = child.wait().context("wait ffmpeg Opus encode")?;
    if !status.success() {
        let err = String::from_utf8_lossy(&err_buf);
        anyhow::bail!("ffmpeg Opus failed: {err}");
    }
    if bytes.is_empty() {
        anyhow::bail!("ffmpeg Opus produced no stdout");
    }

    Ok(UploadAudio {
        pcm_bytes: audio.bytes.len(),
        bytes,
        file_name: "recording.ogg",
        mime: "audio/ogg",
        codec: "opus",
    })
}

/// Encode raw PCM16 audio into WAV format (lossless container; ~same size as PCM).
pub fn encode_wav(audio: &AudioData) -> Result<Vec<u8>> {
    use std::io::Cursor;

    let spec = hound::WavSpec {
        channels: audio.channels,
        sample_rate: audio.sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut buf = Cursor::new(Vec::new());
    let mut writer = hound::WavWriter::new(&mut buf, spec)?;

    let samples: Vec<i16> = audio
        .bytes
        .chunks_exact(2)
        .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
        .collect();

    for sample in &samples {
        writer.write_sample(*sample)?;
    }
    writer.finalize()?;

    Ok(buf.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestKeyring;
    impl crate::traits::KeyringStore for TestKeyring {
        fn get_api_key(&self) -> Result<String> {
            Ok("test-key".into())
        }
        fn set_api_key(&self, _: &str) -> Result<()> {
            Ok(())
        }
        fn clear(&self) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn test_wav_encoding() {
        let audio = generate_test_audio();
        let wav = encode_wav(&audio).unwrap();
        assert!(wav.len() > 44);
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
    }

    #[test]
    fn test_encode_for_upload_shrinks_with_ffmpeg() {
        // ~1s of tone — long enough for Opus container overhead to still win.
        let sample_rate = 16000u32;
        let duration_samples = sample_rate as usize; // 1s
        let samples: Vec<i16> = (0..duration_samples)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                (t * 440.0 * 2.0 * std::f32::consts::PI)
                    .sin()
                    .mul_add(0.3, 0.0)
                    .mul_add(32767.0, 0.0) as i16
            })
            .collect();
        let bytes: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
        let audio = AudioData::pcm(bytes, sample_rate, 1, 1000);

        let upload = encode_for_upload(&audio).expect("encode");
        assert!(!upload.bytes.is_empty());
        // Prefer Opus when ffmpeg is present; always smaller or equal strategy.
        if upload.codec == "opus" {
            assert!(
                upload.bytes.len() < audio.bytes.len(),
                "opus {} vs pcm {}",
                upload.bytes.len(),
                audio.bytes.len()
            );
            assert_eq!(upload.file_name, "recording.ogg");
            assert!(upload.compression_ratio() > 1.5);
        } else {
            assert_eq!(upload.codec, "wav");
        }
    }

    #[test]
    fn test_opus_encode_does_not_deadlock_on_large_pcm() {
        // ~30s mono 16k PCM (~960KB). The old write-all-then-read path deadlocked
        // once OGG filled the pipe buffer; concurrent I/O must finish quickly.
        let sample_rate = 16000u32;
        let duration_samples = sample_rate as usize * 30;
        let samples: Vec<i16> = (0..duration_samples)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                (t * 220.0 * 2.0 * std::f32::consts::PI)
                    .sin()
                    .mul_add(0.2, 0.0)
                    .mul_add(32767.0, 0.0) as i16
            })
            .collect();
        let bytes: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
        let audio = AudioData::pcm(bytes, sample_rate, 1, 30_000);

        let start = std::time::Instant::now();
        let upload = encode_for_upload(&audio).expect("encode large");
        let elapsed = start.elapsed();
        assert!(
            elapsed < std::time::Duration::from_secs(15),
            "encode took too long (possible deadlock): {elapsed:?}"
        );
        assert!(!upload.bytes.is_empty());
        if upload.codec == "opus" {
            assert!(upload.bytes.len() < audio.bytes.len() / 2);
        }
    }

    #[tokio::test]
    async fn test_stt_client_creation() {
        let client = XaiSttClient::new(std::sync::Arc::new(TestKeyring));
        let _ = client;
    }

    #[tokio::test]
    async fn test_stt_integration_with_mock() {
        use wiremock::matchers::{header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/stt"))
            .and(header("Authorization", "Bearer test-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "text": "hello from mock",
                "duration": 1.5,
                "words": [
                    {"text": "hello", "start": 0.0, "end": 0.4},
                    {"text": "from", "start": 0.4, "end": 0.7},
                    {"text": "mock", "start": 0.7, "end": 1.2}
                ]
            })))
            .mount(&server)
            .await;

        struct MockApiKey;
        impl crate::traits::KeyringStore for MockApiKey {
            fn get_api_key(&self) -> anyhow::Result<String> {
                Ok("test-key".into())
            }
            fn set_api_key(&self, _: &str) -> anyhow::Result<()> {
                Ok(())
            }
            fn clear(&self) -> anyhow::Result<()> {
                Ok(())
            }
        }

        let client = XaiSttClient::with_base_url(
            std::sync::Arc::new(MockApiKey),
            format!("{}/v1/stt", server.uri()),
        );

        let audio = generate_test_audio();
        let result = client.transcribe(&audio).await.unwrap();
        assert_eq!(result.text, "hello from mock");
        assert_eq!(result.words.len(), 3);
    }

    fn generate_test_audio() -> AudioData {
        let sample_rate = 16000u32;
        let duration_samples = (sample_rate as f32 * 0.5) as usize;
        let samples: Vec<i16> = (0..duration_samples)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                (t * 440.0 * 2.0 * std::f32::consts::PI)
                    .sin()
                    .mul_add(0.5, 0.0)
                    .mul_add(32767.0, 0.0) as i16
            })
            .collect();

        let bytes: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();

        AudioData::pcm(bytes, sample_rate, 1, 500)
    }

    #[test]
    fn test_encode_uses_pre_encoded_progressive() {
        let pcm = vec![0u8; 32000]; // 1s silence-ish
        let pre = crate::traits::PreEncodedAudio {
            bytes: b"OggSfake-opus".to_vec(),
            file_name: "recording.ogg".into(),
            mime: "audio/ogg".into(),
            codec: "opus-progressive".into(),
        };
        let mut audio = AudioData::pcm(pcm, 16000, 1, 1000);
        audio.pre_encoded = Some(pre.clone());
        let upload = encode_for_upload(&audio).unwrap();
        assert_eq!(upload.codec, "opus-progressive");
        assert_eq!(upload.bytes, pre.bytes);
    }
}
