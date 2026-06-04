// ── xAI Grok STT client ───────────────────────────────────────
// REST API: POST https://api.x.ai/v1/stt
// Docs: https://docs.x.ai/developers/model-capabilities/audio/speech-to-text
//
// Sends audio as multipart/form-data with optional language/format params.
// Returns full SttResult (text + word timestamps); see traits::SttResult.

use crate::traits::{AudioData, SttClient, SttResult};
use anyhow::{Context, Result};
use std::sync::Arc;
use std::time::Duration;

const STT_URL: &str = "https://api.x.ai/v1/stt";

pub struct XaiSttClient {
    client: reqwest::Client,
    api_key: Arc<dyn crate::traits::KeyringStore>,
    base_url: String,
}

impl XaiSttClient {
    pub fn new(api_key: Arc<dyn crate::traits::KeyringStore>) -> Self {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(120))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            client,
            api_key,
            base_url: STT_URL.to_string(),
        }
    }

    #[cfg(test)]
    fn with_base_url(api_key: Arc<dyn crate::traits::KeyringStore>, url: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            base_url: url,
        }
    }
}

#[async_trait::async_trait]
impl SttClient for XaiSttClient {
    async fn transcribe(&self, audio: &AudioData) -> Result<SttResult> {
        let key = self.api_key.get_api_key()?;

        let wav = encode_wav(audio)?;

        let part = reqwest::multipart::Part::bytes(wav)
            .file_name("recording.wav")
            .mime_str("audio/wav")?;

        let lang = crate::keyring::get_language();
        let form = reqwest::multipart::Form::new()
            .text("format", "true")
            .text("language", lang)
            .part("file", part);

        let response = self
            .client
            .post(&self.base_url)
            .header("Authorization", format!("Bearer {key}"))
            .multipart(form)
            .send()
            .await
            .context("STT request failed")?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("STT API error {status}: {body}");
        }

        let body = response
            .text()
            .await
            .context("failed to read STT response")?;
        let json: serde_json::Value =
            serde_json::from_str(&body).context("invalid STT response JSON")?;
        SttResult::from_api_json(json).with_context(|| format!("invalid STT payload: {body}"))
    }
}

/// Encode raw PCM16 audio into WAV format.
fn encode_wav(audio: &AudioData) -> Result<Vec<u8>> {
    use std::io::Cursor;

    let spec = hound::WavSpec {
        channels: audio.channels,
        sample_rate: audio.sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut buf = Cursor::new(Vec::new());
    let mut writer = hound::WavWriter::new(&mut buf, spec)?;

    // Convert raw PCM16 bytes to i16 samples
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
        // Generate 0.1s of 440Hz sine wave at 16kHz
        let sample_rate = 16000u32;
        let duration_samples = (sample_rate as f32 * 0.1) as usize;
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

        let audio = AudioData {
            bytes,
            sample_rate,
            channels: 1,
            duration_ms: 100,
        };

        let wav = encode_wav(&audio).unwrap();
        assert!(wav.len() > 44); // WAV header is 44 bytes
        assert_eq!(&wav[0..4], b"RIFF"); // WAV magic
        assert_eq!(&wav[8..12], b"WAVE");
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

        AudioData {
            bytes,
            sample_rate,
            channels: 1,
            duration_ms: 500,
        }
    }
}
