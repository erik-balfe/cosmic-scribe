// ── Web UI ───────────────────────────────────────────────────
// Serves embedded Svelte SPA + JSON API.
// SPA is built from web/ via `npm run build` and embedded with rust-embed.

use crate::api::{self, ApiError};
use rust_embed::RustEmbed;
use std::io::Write;
use std::net::TcpListener;
use std::path::PathBuf;

#[derive(RustEmbed)]
#[folder = "web/dist"]
struct Assets;

fn serve_asset(path: &str) -> Option<(String, Vec<u8>)> {
    let path = path.trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };
    if let Some(file) = Assets::get(path) {
        let mime = mime_guess(path);
        Some((mime, file.data.to_vec()))
    } else {
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

fn api_ok_json<T: serde::Serialize>(value: &T) -> (String, String, String) {
    (
        "200 OK".into(),
        "application/json".into(),
        serde_json::to_string(value).unwrap_or_else(|_| "{}".into()),
    )
}

fn api_error(err: ApiError) -> (String, String, String) {
    let (status, msg) = match &err {
        ApiError::NotFound(m) => ("404 Not Found", m),
        ApiError::BadRequest(m) => ("400 Bad Request", m),
        ApiError::Internal(m) => ("500 Internal Server Error", m),
    };
    let json = serde_json::json!({ "ok": false, "error": msg }).to_string();
    (status.into(), "application/json".into(), json)
}

fn recording_id_from_path(path: &str, suffix: &str) -> String {
    path.strip_prefix("/api/recording/")
        .unwrap_or("")
        .trim_end_matches(suffix)
        .trim_end_matches('/')
        .to_string()
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
        ("POST", p) if p.ends_with("/transcribe") => api_transcribe(p),
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
    let entries = api::list_history(offset, limit);
    api_ok_json(&entries)
}

fn api_waveform(path: &str) -> (String, String, String) {
    let id = recording_id_from_path(path, "/waveform");
    match api::validate_recording_id(&id) {
        Ok(safe) => {
            let raw_path = api::recordings_dir().join(format!("{safe}.raw"));
            let wf = serde_json::json!({"waveform": api::waveform_data(&raw_path)});
            api_ok_json(&wf)
        }
        Err(e) => api_error(e),
    }
}

fn api_recording_get(path: &str) -> (String, String, String) {
    let id = recording_id_from_path(path, "");
    match api::get_recording(&id) {
        Ok(detail) => api_ok_json(&detail),
        Err(e) => api_error(e),
    }
}

fn api_edit(path: &str, body: &str) -> (String, String, String) {
    let id = recording_id_from_path(path, "/edit");
    let v: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => return ("400 Bad Request".into(), "text/plain".into(), e.to_string()),
    };
    let new_text = v.get("text").and_then(|t| t.as_str()).unwrap_or("");
    let edit_type = v
        .get("type")
        .and_then(|t| t.as_str())
        .unwrap_or("user_edit");
    match api::save_user_edit(&id, new_text, edit_type) {
        Ok(()) => (
            "200 OK".into(),
            "application/json".into(),
            r#"{"ok":true}"#.into(),
        ),
        Err(e) => api_error(e),
    }
}

fn api_transcribe(path: &str) -> (String, String, String) {
    let id = recording_id_from_path(path, "/transcribe");
    match api::transcribe_recording(&id) {
        Ok(result) => api_ok_json(&result),
        Err(e) => api_error(e),
    }
}

fn api_delete(path: &str) -> (String, String, String) {
    let id = recording_id_from_path(path, "/delete");
    match api::delete_recording(&id) {
        Ok(()) => (
            "200 OK".into(),
            "application/json".into(),
            r#"{"ok":true}"#.into(),
        ),
        Err(e) => api_error(e),
    }
}

fn api_correct(path: &str, body: &str) -> (String, String, String) {
    let id = recording_id_from_path(path, "/correct");
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

    match api::correct_recording(&id, text, &marked, &kept) {
        Ok(corrected) => (
            "200 OK".into(),
            "application/json".into(),
            serde_json::json!({"ok":true,"text":corrected}).to_string(),
        ),
        Err(e) => {
            let status = match &e {
                ApiError::BadRequest(_) => "400 Bad Request",
                _ => "502 Bad Gateway",
            };
            (
                status.into(),
                "application/json".into(),
                serde_json::json!({"error": e.message()}).to_string(),
            )
        }
    }
}

fn api_config_get() -> (String, String, String) {
    api_ok_json(&api::get_config())
}

fn api_config_post(body: &str) -> (String, String, String) {
    match api::save_config_from_json(body) {
        Ok(()) => (
            "200 OK".into(),
            "application/json".into(),
            r#"{"ok":true}"#.into(),
        ),
        Err(e) => api_error(e),
    }
}

fn api_audio(_path: &str) -> (String, String, String) {
    (
        "404 Not Found".into(),
        "text/plain".into(),
        "use /api/recording/:id/audio".into(),
    )
}

fn api_models() -> (String, String, String) {
    (
        "200 OK".into(),
        "application/json".into(),
        api::models_json(),
    )
}

fn serve_audio(stream: &std::net::TcpStream, req: &str) {
    let path = req.split_whitespace().nth(1).unwrap_or("");
    let id = recording_id_from_path(path, "/audio");
    let Ok(raw) = api::read_audio_pcm(&id) else {
        return;
    };

    let wav = api::encode_pcm_to_wav(&raw);
    let header = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: audio/wav\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        wav.len()
    );
    if let Ok(mut s) = stream.try_clone() {
        let _ = s.write_all(header.as_bytes());
        let _ = s.write_all(&wav);
    }
}

fn ui_url(addr: std::net::SocketAddr, start_path: &str) -> String {
    let path = if start_path.is_empty() || start_path == "/" {
        String::new()
    } else {
        start_path.to_string()
    };
    format!("http://{addr}{path}")
}

fn serve_listener(listener: TcpListener) {
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
}

/// Start the embedded UI API server on a background thread. Returns the URL to load (for Tauri / tests).
pub fn spawn_server(start_path: &str) -> anyhow::Result<String> {
    api::prune_junk_on_ui_start();
    std::thread::spawn(api::preload_models_cache);

    let listener = TcpListener::bind("127.0.0.1:0")?;
    let addr = listener.local_addr()?;
    let url = ui_url(addr, start_path);
    std::thread::spawn(move || serve_listener(listener));
    Ok(url)
}

fn ui_lock_path() -> PathBuf {
    crate::lifecycle::data_dir().join("ui-browser.lock")
}

fn process_alive(pid: u32) -> bool {
    crate::lifecycle::process_alive(pid)
}

fn read_ui_lock() -> Option<(u32, String)> {
    let raw = std::fs::read_to_string(ui_lock_path()).ok()?;
    let mut lines = raw.lines();
    let pid: u32 = lines.next()?.trim().parse().ok()?;
    let base = lines.next()?.trim().to_string();
    if base.is_empty() {
        return None;
    }
    Some((pid, base))
}

fn write_ui_lock(pid: u32, base_url: &str) {
    let path = ui_lock_path();
    let _ = std::fs::write(path, format!("{pid}\n{base_url}\n"));
}

fn clear_ui_lock() {
    let _ = std::fs::remove_file(ui_lock_path());
}

fn open_in_browser(url: &str) {
    if crate::env_compat("COSMIC_SCRIBE_NO_BROWSER", "VOICE_INPUT_NO_BROWSER").is_some() {
        return;
    }
    let _ = std::process::Command::new("sh")
        .arg("-c")
        .arg(format!("xdg-open '{url}' >/dev/null 2>&1 &"))
        .spawn();
}

/// Open the browser UI at `start_path`, reusing an existing server when possible.
pub fn open_ui(start_path: &str) -> anyhow::Result<()> {
    if let Some((pid, base)) = read_ui_lock() {
        if process_alive(pid) {
            let url = format!(
                "{base}{}",
                if start_path.is_empty() || start_path == "/" {
                    String::new()
                } else {
                    start_path.to_string()
                }
            );
            open_in_browser(&url);
            return Ok(());
        }
        clear_ui_lock();
    }
    run_at(start_path)
}

pub fn run() -> anyhow::Result<()> {
    run_at("/")
}

pub fn run_at(start_path: &str) -> anyhow::Result<()> {
    api::prune_junk_on_ui_start();
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let addr = listener.local_addr()?;
    let base_url = ui_url(addr, "");
    let url = ui_url(addr, start_path);
    let pid = std::process::id();
    write_ui_lock(pid, &base_url);
    println!("UI: {}", url);
    open_in_browser(&url);

    let result =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| serve_listener(listener)));
    clear_ui_lock();
    if let Err(e) = result {
        std::panic::resume_unwind(e);
    }
    Ok(())
}

fn read_http_request(stream: &std::net::TcpStream) -> Option<(String, String)> {
    use std::io::Read;
    let mut reader = stream.try_clone().ok()?;
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];

    loop {
        let n = reader.read(&mut chunk).ok()?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
        if buf.windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
        if buf.len() > 1_048_576 {
            return None;
        }
    }

    let header_end = buf.windows(4).position(|w| w == b"\r\n\r\n")? + 4;
    let headers = String::from_utf8_lossy(&buf[..header_end]);
    let req_line = headers.lines().next()?.to_string();

    let content_length = headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            if name.trim().eq_ignore_ascii_case("content-length") {
                value.trim().parse::<usize>().ok()
            } else {
                None
            }
        })
        .unwrap_or(0);

    let mut body = buf[header_end..].to_vec();
    while body.len() < content_length {
        let n = reader.read(&mut chunk).ok()?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..n]);
        if body.len() > 1_048_576 {
            return None;
        }
    }
    body.truncate(content_length);
    let body = String::from_utf8_lossy(&body).into_owned();
    Some((req_line, body))
}

fn handle_request(stream: &std::net::TcpStream) {
    let Some((req_line, body)) = read_http_request(stream) else {
        return;
    };

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
        serve_audio(stream, &req_line);
        return;
    } else {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            handle_api(&req_line, &body)
        }));
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

    fn init_tests() {
        crate::lifecycle::init_test_recordings_dir();
    }

    #[test]
    fn test_api_config_post_accepts_wtype() {
        init_tests();
        let body =
            r#"{"lang":"en","output_mode":"wtype","correction_model":"deepseek/deepseek-chat-v4"}"#;
        let (status, ct, resp) = api_config_post(body);
        assert_eq!(status, "200 OK");
        assert_eq!(ct, "application/json");
        let json: serde_json::Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(json["ok"], true);
    }

    #[test]
    fn test_api_config_post_rejects_invalid_output_mode() {
        init_tests();
        let body = r#"{"output_mode":"auto"}"#;
        let (status, _, resp) = api_config_post(body);
        assert_eq!(status, "400 Bad Request");
        let json: serde_json::Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(json["ok"], false);
        assert!(json["error"].as_str().unwrap().contains("wtype"));
    }

    #[test]
    fn test_api_config_post_rejects_empty_body() {
        init_tests();
        let (status, _, _) = api_config_post("");
        assert_eq!(status, "400 Bad Request");
    }

    #[test]
    fn test_read_http_request_reads_split_post_body() {
        init_tests();
        use std::io::{Read, Write};
        use std::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let body = r#"{"lang":"en","output_mode":"wtype"}"#;
        let req = format!(
            "POST /api/config HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
            body.len()
        );

        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut payload = req.into_bytes();
            payload.extend_from_slice(body.as_bytes());
            stream.write_all(&payload).unwrap();
            stream.shutdown(std::net::Shutdown::Write).unwrap();
            let mut discard = [0u8; 1024];
            let _ = stream.read(&mut discard);
        });

        let client = std::net::TcpStream::connect(addr).unwrap();
        let (req_line, read_body) = read_http_request(&client).unwrap();
        assert!(req_line.starts_with("POST /api/config"));
        assert_eq!(read_body, body);
        let _ = client.shutdown(std::net::Shutdown::Both);
    }

    #[test]
    fn test_recording_get_works_with_new_format() {
        init_tests();
        let dir = api::recordings_dir();
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
        assert_eq!(json["ts"], "2026-06-01 15:13:17");

        let _ = std::fs::remove_file(dir.join(format!("{id}.raw")));
        let _ = std::fs::remove_file(dir.join(format!("{id}.txt")));
    }
}
