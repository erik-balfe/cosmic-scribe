// ── Web UI ───────────────────────────────────────────────────
// Serves embedded Svelte SPA + JSON API.
// SPA is built from web/ via `npm run build` and embedded with rust-embed.

use crate::traits::KeyringStore;
use rust_embed::RustEmbed;
use std::io::{Read, Write};
use std::net::TcpListener;

#[derive(RustEmbed)]
#[folder = "web/dist"]
struct Assets;

fn recordings_dir() -> std::path::PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| "/tmp".into())
        .join("voice-input")
        .join("recordings")
}

fn serve_asset(path: &str) -> Option<(String, Vec<u8>)> {
    let path = path.trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };
    if let Some(file) = Assets::get(path) {
        let mime = mime_guess(path);
        Some((mime, file.data.to_vec()))
    } else {
        // SPA routing: serve index.html for client-side routes
        Assets::get("index.html").map(|f| ("text/html".into(), f.data.to_vec()))
    }
}

fn mime_guess(path: &str) -> String {
    match path.rsplit('.').next() {
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "application/javascript",
        Some("css") => "text/css",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        _ => "application/octet-stream",
    }
    .into()
}

fn handle_api(req: &str, body: &str) -> (String, String, String) {
    let path = req.split_whitespace().nth(1).unwrap_or("/");

    match (req.split_whitespace().next().unwrap_or(""), path) {
        ("GET", p) if p.starts_with("/api/history") => api_history(p),
        ("GET", p) if p.starts_with("/api/recording/") && p.contains("/waveform") => {
            api_waveform(p)
        }
        ("GET", p) if p.starts_with("/api/recording/") && p.contains("/audio") => api_audio(p),
        ("GET", p) if p.starts_with("/api/recording/") => api_recording_get(p),
        ("POST", p) if p.ends_with("/delete") => api_delete(p),
        ("POST", p) if p.ends_with("/correct") => api_correct(p, body),
        ("POST", p) if p.ends_with("/edit") => api_edit(p, body),
        ("GET", "/api/config") => api_config_get(),
        ("POST", "/api/config") => api_config_post(body),
        ("GET", "/api/models") => api_models(),
        _ => (
            "404 Not Found".into(),
            "text/plain".into(),
            "not found".into(),
        ),
    }
}

fn ts_to_human(parts: &[&str]) -> String {
    if parts.len() < 2 {
        return parts.join(" ");
    }
    let date = parts[0].to_string();
    let time = parts[1].replace('-', ":");
    format!("{date} {time}")
}

fn api_history(path: &str) -> (String, String, String) {
    let query: std::collections::HashMap<String, String> = path
        .split('?')
        .nth(1)
        .unwrap_or("")
        .split('&')
        .filter_map(|p| p.split_once('='))
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    let offset: usize = query
        .get("offset")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let limit: usize = query
        .get("limit")
        .and_then(|v| v.parse().ok())
        .unwrap_or(20);

    let mut entries = Vec::new();
    let dir = recordings_dir();
    if let Ok(read_dir) = std::fs::read_dir(&dir) {
        let mut raws: Vec<_> = read_dir
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map(|x| x == "raw").unwrap_or(false))
            .collect();
        raws.sort_by_key(|e| {
            std::fs::metadata(e.path())
                .ok()
                .and_then(|m| m.modified().ok())
        });
        raws.reverse();
        for entry in raws.iter().skip(offset).take(limit) {
            let path = entry.path();
            let stem = path.file_stem().unwrap_or_default().to_string_lossy();
            let txt = path.with_extension("txt");
            let text = std::fs::read_to_string(&txt).ok();
            let meta = path.with_extension("json");
            let meta_json = std::fs::read_to_string(&meta).ok();
            let has_text = text.is_some();
            let has_stt = path.with_extension("stt.json").is_file();
            let parts: Vec<&str> = stem.split('_').collect();
            let ts = ts_to_human(&parts);
            let dur = parts.last().unwrap_or(&"0").replace("ms", "");
            let dur_secs = dur.parse::<u64>().unwrap_or(0) / 1000;
            let mut entry = serde_json::json!({
                "file": stem,
                "ts": ts,
                "duration": format!("{}s", dur_secs),
                "text": text,
                "has_text": has_text,
                "has_stt": has_stt,
            });
            if let Some(meta_v) =
                meta_json.and_then(|m| serde_json::from_str::<serde_json::Value>(&m).ok())
            {
                entry["versions"] = meta_v
                    .get("versions")
                    .cloned()
                    .unwrap_or(serde_json::json!([]));
            }
            entries.push(entry);
        }
    }
    let json = serde_json::Value::Array(entries).to_string();
    ("200 OK".into(), "application/json".into(), json)
}

fn api_waveform(path: &str) -> (String, String, String) {
    let id = path
        .strip_prefix("/api/recording/")
        .unwrap_or("")
        .trim_end_matches("/waveform");
    let raw_path = recordings_dir().join(format!("{id}.raw"));
    let wf = serde_json::json!({"waveform": waveform_data(&raw_path)}).to_string();
    ("200 OK".into(), "application/json".into(), wf)
}

fn api_recording_get(path: &str) -> (String, String, String) {
    let id = path
        .strip_prefix("/api/recording/")
        .unwrap_or("")
        .trim_end_matches('/');
    let raw_path = recordings_dir().join(format!("{id}.raw"));
    let txt_path = recordings_dir().join(format!("{id}.txt"));
    let meta_path = recordings_dir().join(format!("{id}.json"));

    let text = std::fs::read_to_string(&txt_path).ok().unwrap_or_default();
    let meta = std::fs::read_to_string(&meta_path)
        .ok()
        .and_then(|m| serde_json::from_str::<serde_json::Value>(&m).ok())
        .unwrap_or(serde_json::json!({}));

    let parts: Vec<&str> = id.split('_').collect();
    let ts = format!(
        "{}T{}",
        parts.first().unwrap_or(&""),
        parts.get(1).unwrap_or(&"")
    );
    let dur = parts.last().unwrap_or(&"0").replace("ms", "");
    let dur_secs = dur.parse::<u64>().unwrap_or(0) / 1000;

    let stt_path = recordings_dir().join(format!("{id}.stt.json"));
    let stt = std::fs::read_to_string(&stt_path)
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok());

    let json = serde_json::json!({
        "file": id,
        "ts": ts,
        "duration": format!("{}s", dur_secs),
        "lang": crate::keyring::get_language(),
        "text": text,
        "has_stt": stt.is_some(),
        "stt": stt,
        "versions": meta.get("versions").cloned().unwrap_or(serde_json::json!([])),
        "waveform": waveform_data(&raw_path),
    })
    .to_string();
    ("200 OK".into(), "application/json".into(), json)
}

fn waveform_data(path: &std::path::Path) -> Vec<f64> {
    let Ok(raw) = std::fs::read(path) else {
        return vec![];
    };
    if raw.len() < 4 {
        return vec![];
    }
    let samples: Vec<i16> = raw
        .chunks_exact(2)
        .map(|c| i16::from_le_bytes([c[0], c[1]]))
        .collect();
    let bars = 200usize;
    let step = (samples.len() / bars).max(1);
    (0..bars)
        .map(|i| {
            let start = (i * step).min(samples.len());
            let end = ((i + 1) * step).min(samples.len());
            let slice = &samples[start..end];
            let max = slice.iter().map(|s| (*s as i32).abs()).max().unwrap_or(0) as f64;
            (max / 32768.0).min(1.0)
        })
        .collect()
}

fn api_edit(path: &str, body: &str) -> (String, String, String) {
    let id = path
        .strip_prefix("/api/recording/")
        .unwrap_or("")
        .trim_end_matches("/edit");
    let v: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => return ("400 Bad Request".into(), "text/plain".into(), e.to_string()),
    };
    let new_text = v.get("text").and_then(|t| t.as_str()).unwrap_or("");
    let edit_type = v
        .get("type")
        .and_then(|t| t.as_str())
        .unwrap_or("user_edit");

    let meta_path = recordings_dir().join(format!("{id}.json"));
    let mut meta: serde_json::Value = std::fs::read_to_string(&meta_path)
        .ok()
        .and_then(|m| serde_json::from_str(&m).ok())
        .unwrap_or(serde_json::json!({}));

    let ts = chrono::Utc::now().to_rfc3339();
    let versions = if let Some(arr) = meta.get_mut("versions").and_then(|v| v.as_array_mut()) {
        arr
    } else {
        meta["versions"] = serde_json::json!([]);
        meta["versions"].as_array_mut().unwrap()
    };

    versions.push(serde_json::json!({
        "type": edit_type,
        "text": new_text,
        "timestamp": ts,
    }));

    let _ = std::fs::write(&meta_path, serde_json::to_string(&meta).unwrap_or_default());
    (
        "200 OK".into(),
        "application/json".into(),
        r#"{"ok":true}"#.into(),
    )
}

fn api_delete(path: &str) -> (String, String, String) {
    let id = path
        .strip_prefix("/api/recording/")
        .unwrap_or("")
        .trim_end_matches("/delete");
    let raw = recordings_dir().join(format!("{id}.raw"));
    let txt = recordings_dir().join(format!("{id}.txt"));
    let meta = recordings_dir().join(format!("{id}.json"));
    let stt = recordings_dir().join(format!("{id}.stt.json"));
    let _ = std::fs::remove_file(&raw);
    let _ = std::fs::remove_file(&txt);
    let _ = std::fs::remove_file(&meta);
    let _ = std::fs::remove_file(&stt);
    (
        "200 OK".into(),
        "application/json".into(),
        r#"{"ok":true}"#.into(),
    )
}

fn api_correct(path: &str, body: &str) -> (String, String, String) {
    let id = path
        .strip_prefix("/api/recording/")
        .unwrap_or("")
        .trim_end_matches("/correct");
    let v: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => return ("400 Bad Request".into(), "text/plain".into(), e.to_string()),
    };

    let text = v.get("text").and_then(|t| t.as_str()).unwrap_or("");
    let marked: Vec<String> = v
        .get("marked")
        .and_then(|m| m.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let kept: Vec<String> = v
        .get("kept")
        .and_then(|k| k.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let correction_key = crate::keyring::get_correction_key();

    if correction_key.is_empty() {
        return (
            "400 Bad Request".into(),
            "application/json".into(),
            serde_json::json!({"error": "OpenRouter API key not configured. Add it in Settings."})
                .to_string(),
        );
    }

    let prompt = format!(
        "You are correcting a speech-to-text transcription from Cosmic Scribe.\n\
         The user spoke into a microphone, and an STT model transcribed it.\n\
         The user has marked some words/phrases:\n\
         - RED (must change): {}\n\
         - GREEN (must keep): {}\n\
         - UNMARKED (can change if needed): everything else\n\n\
         Full transcript:\n{text}\n\n\
         Rules:\n\
         1. Words marked RED are likely wrong — replace with phonetically similar\n\
            words that fit the context better.\n\
         2. Words marked GREEN must NOT change.\n\
         3. Unmarked words may be adjusted slightly for grammar/flow.\n\
         4. Maintain the overall meaning and style.\n\n\
         Return ONLY the corrected full transcript. No explanation.",
        if marked.is_empty() {
            "none".into()
        } else {
            marked.join(", ")
        },
        if kept.is_empty() {
            "none".into()
        } else {
            kept.join(", ")
        },
    );

    // We're called from a tokio runtime context, so reqwest::blocking's internal
    // runtime can panic. Hop to a fresh OS thread that has no runtime.
    let model = crate::keyring::get_correction_model();
    let model = if model.is_empty() {
        "deepseek/deepseek-chat".to_string()
    } else {
        model
    };
    let prompt_owned = prompt.clone();
    let key_owned = correction_key.clone();
    let model_for_thread = model.clone();
    let handle = std::thread::spawn(move || -> Result<String, String> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| e.to_string())?;
        let resp = client
            .post("https://openrouter.ai/api/v1/chat/completions")
            .header("Authorization", format!("Bearer {key_owned}"))
            .header(
                "HTTP-Referer",
                "https://github.com/erik-balfe/cosmic-scribe",
            )
            .header("X-Title", "Cosmic Scribe")
            .json(&serde_json::json!({
                "model": model_for_thread,
                "messages": [{"role": "user", "content": prompt_owned}],
                "temperature": 0.3,
            }))
            .send()
            .map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body: serde_json::Value = resp.json().unwrap_or(serde_json::json!({}));
            let msg = body["error"]["message"].as_str().unwrap_or("unknown");
            let hint = if msg.to_lowercase().contains("authentication")
                || msg.to_lowercase().contains("not found")
            {
                " — check that your key is an OpenRouter key (starts with sk-or-v1-) and is active."
            } else {
                ""
            };
            return Err(format!("OpenRouter {}: {}{}", status, msg, hint));
        }

        let json: serde_json::Value = resp.json().map_err(|e| e.to_string())?;
        json["choices"][0]["message"]["content"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| format!("unexpected response: {json}"))
    });
    let result = handle
        .join()
        .unwrap_or_else(|_| Err("correction thread panicked".into()));

    match result {
        Ok(corrected) => {
            // Save as new version
            let meta_path = recordings_dir().join(format!("{id}.json"));
            let mut meta: serde_json::Value = std::fs::read_to_string(&meta_path)
                .ok()
                .and_then(|m| serde_json::from_str(&m).ok())
                .unwrap_or(serde_json::json!({}));
            if meta.get("versions").and_then(|v| v.as_array()).is_none() {
                meta["versions"] = serde_json::json!([]);
            }
            let versions = meta["versions"].as_array_mut().unwrap();
            versions.push(serde_json::json!({
                "type": "llm_correction",
                "text": corrected,
                "marked": marked,
                "model": model,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }));
            let _ = std::fs::write(&meta_path, serde_json::to_string(&meta).unwrap_or_default());
            (
                "200 OK".into(),
                "application/json".into(),
                serde_json::json!({"ok":true,"text":corrected}).to_string(),
            )
        }
        Err(e) => (
            "502 Bad Gateway".into(),
            "application/json".into(),
            serde_json::json!({"error": e}).to_string(),
        ),
    }
}

fn api_config_get() -> (String, String, String) {
    let has_key =
        crate::traits::KeyringStore::get_api_key(&crate::keyring::ConfigFileKeyring).is_ok();
    let has_correction_key = !crate::keyring::get_correction_key().is_empty();
    let correction_model = crate::keyring::get_correction_model();
    let json = serde_json::json!({
        "lang": crate::keyring::get_language(),
        "output_mode": crate::keyring::get_output_mode(),
        "has_key": has_key,
        "has_correction_key": has_correction_key,
        "correction_model": correction_model,
    })
    .to_string();
    ("200 OK".into(), "application/json".into(), json)
}

fn api_config_post(body: &str) -> (String, String, String) {
    let v: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => return ("400 Bad Request".into(), "text/plain".into(), e.to_string()),
    };
    if let Some(lang) = v.get("lang").and_then(|l| l.as_str()) {
        let _ = crate::keyring::set_language(lang);
    }
    if let Some(mode) = v.get("output_mode").and_then(|m| m.as_str()) {
        let _ = crate::keyring::set_output_mode(mode);
    }
    if let Some(key) = v.get("key").and_then(|k| k.as_str()) {
        if !key.is_empty() {
            let _ = crate::keyring::ConfigFileKeyring.set_api_key(key);
        }
    }
    if let Some(ck) = v.get("correction_key").and_then(|k| k.as_str()) {
        if !ck.is_empty() {
            let _ = crate::keyring::set_correction_key(ck);
        }
    }
    if let Some(cm) = v.get("correction_model").and_then(|m| m.as_str()) {
        let _ = crate::keyring::set_correction_model(cm);
    }
    (
        "200 OK".into(),
        "application/json".into(),
        r#"{"ok":true}"#.into(),
    )
}

fn serve_audio(stream: &std::net::TcpStream, req: &str) {
    let path = req.split_whitespace().nth(1).unwrap_or("");
    let id = path
        .strip_prefix("/api/recording/")
        .unwrap_or("")
        .trim_end_matches("/audio");
    let raw_path = recordings_dir().join(format!("{id}.raw"));
    let Ok(raw) = std::fs::read(&raw_path) else {
        return;
    };

    let wav = encode_pcm_to_wav(&raw);
    let header = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: audio/wav\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        wav.len()
    );
    if let Ok(mut s) = stream.try_clone() {
        let _ = s.write_all(header.as_bytes());
        let _ = s.write_all(&wav);
    }
}

fn encode_pcm_to_wav(pcm: &[u8]) -> Vec<u8> {
    let data_len = pcm.len() as u32;
    let sample_rate: u32 = 16000;
    let bits_per_sample: u16 = 16;
    let channels: u16 = 1;
    let byte_rate = sample_rate * channels as u32 * bits_per_sample as u32 / 8;
    let block_align = channels * bits_per_sample / 8;

    let mut wav = Vec::with_capacity(44 + pcm.len());
    // RIFF header
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_len).to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    // fmt chunk
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes()); // PCM
    wav.extend_from_slice(&channels.to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&byte_rate.to_le_bytes());
    wav.extend_from_slice(&block_align.to_le_bytes());
    wav.extend_from_slice(&bits_per_sample.to_le_bytes());
    // data chunk
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());
    wav.extend_from_slice(pcm);
    wav
}

fn api_audio(_path: &str) -> (String, String, String) {
    (
        "404 Not Found".into(),
        "text/plain".into(),
        "use /api/recording/:id/audio".into(),
    )
}

use std::sync::OnceLock;

static MODELS_CACHE: OnceLock<String> = OnceLock::new();

fn load_models_cache() {
    let _ = MODELS_CACHE.get_or_init(|| {
        let output = std::process::Command::new("curl")
            .args(["-s", "--max-time", "10", "https://models.dev/api.json"])
            .output();
        match output {
            Ok(o) if o.status.success() => {
                let models = parse_models_json(&o.stdout);
                serde_json::Value::Array(models).to_string()
            }
            _ => serde_json::json!({"error": "Failed to fetch models"}).to_string(),
        }
    });
}

fn parse_models_json(data: &[u8]) -> Vec<serde_json::Value> {
    let json: serde_json::Value = match serde_json::from_slice(data) {
        Ok(v) => v,
        Err(_) => return vec![],
    };
    let providers = match json.as_object() {
        Some(p) => p,
        None => return vec![],
    };
    let mut models = Vec::new();
    let recommended = [
        "deepseek/deepseek-chat-v4",
        "deepseek/deepseek-chat",
        "openai/gpt-4o-mini",
        "x-ai/grok-2",
    ];

    for (pid, pdata) in providers {
        let pname = pdata.get("name").and_then(|n| n.as_str()).unwrap_or(pid);
        if let Some(pmodels) = pdata.get("models").and_then(|m| m.as_object()) {
            for (mid, mdata) in pmodels {
                let name = mdata.get("name").and_then(|n| n.as_str()).unwrap_or(mid);
                if name.len() >= 100 {
                    continue;
                }
                let pricing = mdata
                    .get("pricing")
                    .and_then(|p| p.get("input"))
                    .and_then(|i| i.as_str())
                    .unwrap_or("?");
                let full_id = format!("{pid}/{mid}");
                let rec = recommended.contains(&full_id.as_str());
                models.push(serde_json::json!({
                    "id": full_id,
                    "name": format!("{name} ({pname})"),
                    "pricing": format!("${pricing}/1M"),
                    "rec": rec,
                }));
            }
        }
    }
    models.sort_by(|a, b| match (a["rec"].as_bool(), b["rec"].as_bool()) {
        (Some(true), Some(false)) => std::cmp::Ordering::Less,
        (Some(false), Some(true)) => std::cmp::Ordering::Greater,
        _ => a["id"].as_str().cmp(&b["id"].as_str()),
    });
    models
}

fn api_models() -> (String, String, String) {
    load_models_cache();
    let data = MODELS_CACHE.get().cloned().unwrap_or_else(|| "[]".into());
    ("200 OK".into(), "application/json".into(), data)
}

pub fn run() -> anyhow::Result<()> {
    // Fetch models in background — first API call triggers actual fetch
    std::thread::spawn(|| {
        load_models_cache();
    });

    let listener = TcpListener::bind("127.0.0.1:0")?;
    let addr = listener.local_addr()?;
    let url = format!("http://{}", addr);
    println!("Settings: {}", url);
    if std::env::var("VOICE_INPUT_NO_BROWSER").is_err() {
        let _ = std::process::Command::new("sh")
            .arg("-c")
            .arg(format!("xdg-open {} >/dev/null 2>&1 &", url))
            .spawn();
    }

    for stream in listener.incoming().flatten() {
        let result =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| handle_request(&stream)));
        if let Err(e) = result {
            let msg = if let Some(s) = e.downcast_ref::<String>() {
                s.clone()
            } else if let Some(s) = e.downcast_ref::<&str>() {
                s.to_string()
            } else {
                "unknown panic".into()
            };
            eprintln!("request handler panicked: {msg}");
        }
    }
    Ok(())
}

fn handle_request(stream: &std::net::TcpStream) {
    let mut buf = [0u8; 8192];
    let mut reader = match stream.try_clone() {
        Ok(r) => r,
        Err(_) => return,
    };
    let n = reader.read(&mut buf).unwrap_or(0);
    let raw = String::from_utf8_lossy(&buf[..n]);
    let req_line = raw.lines().next().unwrap_or("");

    let body_start = raw.find("\r\n\r\n").map(|i| i + 4).unwrap_or(raw.len());
    let body = &raw[body_start..];

    let (status, content_type, body_text) = if req_line.starts_with("GET")
        && !req_line.contains("/api/")
    {
        let path = req_line.split_whitespace().nth(1).unwrap_or("/");
        if let Some((ct, data)) = serve_asset(path) {
            let len = data.len();
            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: {ct}\r\nContent-Length: {len}\r\nConnection: close\r\n\r\n"
            );
            if let Ok(mut s) = stream.try_clone() {
                let _ = s.write_all(header.as_bytes());
                let _ = s.write_all(&data);
            }
        }
        return;
    } else if req_line.contains("/api/recording/") && req_line.contains("/audio") {
        serve_audio(stream, req_line);
        return;
    } else {
        let result =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| handle_api(req_line, body)));
        match result {
            Ok(r) => r,
            Err(e) => {
                let msg = e
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
                    .unwrap_or_else(|| "unknown panic".into());
                (
                    "500 Internal Server Error".into(),
                    "text/plain".into(),
                    format!("panic: {msg}"),
                )
            }
        }
    };

    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body_text}",
        body_text.len()
    );
    if let Ok(mut s) = stream.try_clone() {
        let _ = s.write_all(response.as_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_waveform_i16_min_no_panic() {
        // i16::MIN.abs() panics in debug mode — must handle safely
        let samples: Vec<u8> = vec![0x00u8, 0x80u8, 0x00, 0x00]; // i16::MIN then 0
        let dir = recordings_dir();
        std::fs::create_dir_all(&dir).ok();
        let path = dir.join("test_min.raw");
        std::fs::write(&path, &samples).ok();
        let wf = waveform_data(&path);
        assert_eq!(wf.len(), 200);
        assert!(wf.iter().all(|v| *v >= 0.0));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_recording_get_works_with_new_format() {
        let dir = recordings_dir();
        std::fs::create_dir_all(&dir).ok();
        let raw = vec![0u8; 32000];
        let id = "2026-06-01_15-13-17_25414ms";
        std::fs::write(dir.join(format!("{id}.raw")), &raw).ok();
        std::fs::write(dir.join(format!("{id}.txt")), "test transcript").ok();

        let (status, ct, body) = api_recording_get(&format!("/api/recording/{id}"));
        assert_eq!(status, "200 OK");
        assert_eq!(ct, "application/json");
        let json: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(json["file"], id);
        assert_eq!(json["text"], "test transcript");

        let _ = std::fs::remove_file(dir.join(format!("{id}.raw")));
        let _ = std::fs::remove_file(dir.join(format!("{id}.txt")));
    }
}
