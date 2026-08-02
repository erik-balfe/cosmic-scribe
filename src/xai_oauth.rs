//! Cosmic Scribe's own xAI OAuth session (SuperGrok / X Premium+).
//!
//! Device-code login against `auth.x.ai` so STT can use **subscription quota**
//! instead of a pay-per-token API key. Tokens live only under
//! `~/.local/share/cosmic-scribe/xai-oauth.json` (AES encrypted, mode 0600).
//!
//! Public OAuth client id is the desktop/CLI device-code client (no secret).
//! Discovery: `https://auth.x.ai/.well-known/openid-configuration`

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const DISCOVERY_URL: &str = "https://auth.x.ai/.well-known/openid-configuration";
const DEVICE_CODE_URL: &str = "https://auth.x.ai/oauth2/device/code";
/// Public device-code OAuth client (no client secret).
const CLIENT_ID: &str = "b1a00492-073a-47ea-816f-4c329264a828";
const SCOPE: &str = "openid profile email offline_access grok-cli:access api:access";
const API_BASE: &str = "https://api.x.ai/v1";
/// Refresh this many seconds before JWT `exp` (access tokens are ~6h).
const REFRESH_SKEW_SECS: u64 = 3600;

/// Serialize refresh so concurrent STT calls do not race single-use refresh tokens.
static REFRESH_LOCK: Mutex<()> = Mutex::new(());

/// In-process access token cache: (token, jwt_exp_unix). Avoids decrypt+disk on every STT.
static TOKEN_CACHE: Mutex<Option<(String, u64)>> = Mutex::new(None);

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OAuthTokens {
    access_token: String,
    refresh_token: String,
    #[serde(default)]
    id_token: String,
    #[serde(default)]
    expires_in: Option<u64>,
    #[serde(default)]
    token_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredAuth {
    version: u32,
    auth_mode: String,
    tokens: OAuthTokens,
    #[serde(default)]
    token_endpoint: String,
    #[serde(default)]
    last_refresh_unix: u64,
    #[serde(default)]
    api_base: String,
}

fn store_path() -> PathBuf {
    crate::lifecycle::data_dir().join("xai-oauth.json")
}

fn http_client() -> Result<reqwest::blocking::Client> {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(15))
        .build()
        .context("build HTTP client")
}

fn validate_xai_url(url: &str, field: &str) -> Result<()> {
    let u = url.trim();
    if u.is_empty() {
        bail!("{field} is empty");
    }
    let parsed = reqwest::Url::parse(u).with_context(|| format!("invalid {field}"))?;
    if parsed.scheme() != "https" {
        bail!("{field} must be https");
    }
    let host = parsed.host_str().unwrap_or("");
    // Only the auth issuer hosts — not api.x.ai (never send refresh tokens there).
    if host != "auth.x.ai" && host != "accounts.x.ai" {
        bail!("{field} host must be auth.x.ai or accounts.x.ai (got {host})");
    }
    Ok(())
}

fn discovery(client: &reqwest::blocking::Client) -> Result<(String, String)> {
    let resp = client
        .get(DISCOVERY_URL)
        .header("Accept", "application/json")
        .send()
        .context("xAI OIDC discovery request failed")?;
    if !resp.status().is_success() {
        bail!("xAI OIDC discovery HTTP {}", resp.status());
    }
    let v: serde_json::Value = resp.json().context("xAI OIDC discovery JSON")?;
    let auth_ep = v
        .get("authorization_endpoint")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    let token_ep = v
        .get("token_endpoint")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    if auth_ep.is_empty() || token_ep.is_empty() {
        bail!("xAI OIDC discovery missing endpoints");
    }
    validate_xai_url(&auth_ep, "authorization_endpoint")?;
    validate_xai_url(&token_ep, "token_endpoint")?;
    Ok((auth_ep, token_ep))
}

/// Decode JWT `exp` claim (seconds since epoch). Non-JWT tokens return None.
fn jwt_exp(token: &str) -> Option<u64> {
    let mut parts = token.split('.');
    let _h = parts.next()?;
    let payload_b64 = parts.next()?;
    let _s = parts.next()?;
    if parts.next().is_some() {
        return None;
    }
    let bytes = base64_url_decode(payload_b64)?;
    let v: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    v.get("exp")?.as_u64()
}

fn base64_url_decode(s: &str) -> Option<Vec<u8>> {
    let mut std = s.replace('-', "+").replace('_', "/");
    while !std.len().is_multiple_of(4) {
        std.push('=');
    }
    fn val(c: u8) -> Option<u8> {
        match c {
            b'A'..=b'Z' => Some(c - b'A'),
            b'a'..=b'z' => Some(c - b'a' + 26),
            b'0'..=b'9' => Some(c - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }
    let bytes = std.as_bytes();
    let mut out = Vec::with_capacity(bytes.len() * 3 / 4);
    let mut i = 0;
    while i + 4 <= bytes.len() {
        let (a, b, c, d) = (bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]);
        if a == b'=' {
            break;
        }
        let va = val(a)?;
        let vb = val(b)?;
        out.push((va << 2) | (vb >> 4));
        if c != b'=' {
            let vc = val(c)?;
            out.push(((vb & 0x0f) << 4) | (vc >> 2));
            if d != b'=' {
                let vd = val(d)?;
                out.push(((vc & 0x03) << 6) | vd);
            }
        }
        i += 4;
    }
    Some(out)
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn access_token_needs_refresh(access_token: &str) -> bool {
    match jwt_exp(access_token) {
        Some(exp) => now_unix() + REFRESH_SKEW_SECS >= exp,
        // Unknown shape: do not thrash refresh; STT 401 path can force_refresh.
        None => false,
    }
}

/// True if this bearer looks like an OAuth JWT (not an `xai-…` API key).
pub fn looks_like_oauth_bearer(token: &str) -> bool {
    !token.is_empty() && !token.starts_with("xai-") && token.matches('.').count() == 2
}

fn write_store(auth: &StoredAuth) -> Result<()> {
    let path = store_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_vec_pretty(auth).context("serialize oauth store")?;
    let encrypted = crate::keyring::encrypt_bytes(&json)?;
    let mut file = fs::File::create(&path)?;
    file.write_all(&encrypted)?;
    let mut perms = file.metadata()?.permissions();
    perms.set_mode(0o600);
    fs::set_permissions(&path, perms)?;
    Ok(())
}

fn read_store() -> Result<Option<StoredAuth>> {
    let path = store_path();
    if !path.exists() {
        return Ok(None);
    }
    let data = fs::read(&path).with_context(|| format!("read {}", path.display()))?;
    let plain = if crate::keyring::looks_encrypted(&data) {
        crate::keyring::decrypt_bytes(&data)?
    } else {
        data
    };
    let auth: StoredAuth = serde_json::from_slice(&plain).context("parse xai-oauth store")?;
    if auth.tokens.access_token.is_empty() || auth.tokens.refresh_token.is_empty() {
        return Ok(None);
    }
    Ok(Some(auth))
}

pub fn is_logged_in() -> bool {
    read_store().ok().flatten().is_some()
}

pub fn clear() -> Result<()> {
    if let Ok(mut c) = TOKEN_CACHE.lock() {
        *c = None;
    }
    let path = store_path();
    if path.exists() {
        fs::remove_file(&path)?;
        tracing::info!("xAI OAuth credentials cleared");
    }
    Ok(())
}

fn cache_put(access: &str) {
    let exp = jwt_exp(access).unwrap_or(now_unix().saturating_add(3600));
    if let Ok(mut c) = TOKEN_CACHE.lock() {
        *c = Some((access.to_string(), exp));
    }
}

fn cache_get_if_fresh() -> Option<String> {
    let c = TOKEN_CACHE.lock().ok()?;
    let (tok, exp) = c.as_ref()?;
    if now_unix() + REFRESH_SKEW_SECS >= *exp {
        return None;
    }
    Some(tok.clone())
}

fn refresh_tokens(client: &reqwest::blocking::Client, auth: &StoredAuth) -> Result<StoredAuth> {
    let _guard = REFRESH_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    // Re-read under lock: another task may have refreshed already.
    if let Ok(Some(fresh)) = read_store() {
        if fresh.tokens.access_token != auth.tokens.access_token
            && !access_token_needs_refresh(&fresh.tokens.access_token)
        {
            return Ok(fresh);
        }
    }

    let token_endpoint = if !auth.token_endpoint.is_empty() {
        validate_xai_url(&auth.token_endpoint, "token_endpoint")?;
        auth.token_endpoint.clone()
    } else {
        discovery(client)?.1
    };

    let resp = client
        .post(&token_endpoint)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Accept", "application/json")
        .form(&[
            ("grant_type", "refresh_token"),
            ("client_id", CLIENT_ID),
            ("refresh_token", auth.tokens.refresh_token.as_str()),
        ])
        .send()
        .context("xAI token refresh request failed")?;

    let status = resp.status();
    let body = resp.text().unwrap_or_default();
    if status.as_u16() == 403 {
        bail!(
            "xAI OAuth refresh HTTP 403 — this account may not allow speech access via sign-in. \
             Use an API key (`--set-key`) or a SuperGrok / X Premium+ plan. Detail: {body}"
        );
    }
    if !status.is_success() {
        if body.contains("invalid_grant") {
            bail!(
                "xAI OAuth refresh failed (invalid_grant) — re-run `cosmic-scribe --login`. {body}"
            );
        }
        bail!("xAI OAuth refresh HTTP {status}: {body}");
    }

    let v: serde_json::Value = serde_json::from_str(&body).context("refresh JSON")?;
    let access = v
        .get("access_token")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    let refresh = v
        .get("refresh_token")
        .and_then(|x| x.as_str())
        .map(str::to_string)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| auth.tokens.refresh_token.clone());
    if access.is_empty() {
        bail!("xAI refresh response missing access_token");
    }

    let mut next = auth.clone();
    next.tokens.access_token = access;
    next.tokens.refresh_token = refresh;
    next.tokens.id_token = v
        .get("id_token")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    next.tokens.expires_in = v.get("expires_in").and_then(|x| x.as_u64());
    next.tokens.token_type = v
        .get("token_type")
        .and_then(|x| x.as_str())
        .unwrap_or("Bearer")
        .to_string();
    next.token_endpoint = token_endpoint;
    next.last_refresh_unix = now_unix();
    write_store(&next)?;
    Ok(next)
}

/// Return a usable Bearer access token, refreshing if near expiry.
///
/// **Blocking** (network). Call from a blocking thread or CLI, not on a
/// latency-sensitive async worker without `spawn_blocking`.
pub fn access_token() -> Result<String> {
    if let Some(tok) = cache_get_if_fresh() {
        return Ok(tok);
    }

    let client = http_client()?;
    let auth = read_store()?
        .context("no xAI OAuth session — run `cosmic-scribe --login` (SuperGrok / X Premium+)")?;

    if !access_token_needs_refresh(&auth.tokens.access_token) {
        cache_put(&auth.tokens.access_token);
        return Ok(auth.tokens.access_token);
    }

    tracing::info!("xAI OAuth access token near expiry; refreshing");
    let next = refresh_tokens(&client, &auth)?;
    cache_put(&next.tokens.access_token);
    Ok(next.tokens.access_token)
}

/// Force refresh (e.g. after HTTP 401 from STT). Blocking.
pub fn force_refresh() -> Result<String> {
    let client = http_client()?;
    let auth = read_store()?.context("no xAI OAuth session")?;
    let next = refresh_tokens(&client, &auth)?;
    cache_put(&next.tokens.access_token);
    Ok(next.tokens.access_token)
}

/// Ensure access token is valid **before** STT (call when recording starts).
///
/// Runs refresh in a background thread if near expiry so stop→upload never waits
/// on OAuth. Same idea as keeping a warm token on Android.
pub fn warm_token_background() {
    if !is_logged_in() {
        return;
    }
    thread::Builder::new()
        .name("xai-oauth-warm".into())
        .spawn(|| match access_token() {
            Ok(_) => tracing::debug!("xAI OAuth token warm"),
            Err(e) => tracing::debug!("xAI OAuth warm skipped: {e}"),
        })
        .ok();
}

/// Periodic keep-warm for long-lived daemon (every ~50 minutes while idle).
pub async fn keep_warm_loop() {
    // First warm shortly after boot (after tray/etc.).
    tokio::time::sleep(Duration::from_secs(5)).await;
    loop {
        let _ = tokio::task::spawn_blocking(access_token).await;
        // Access tokens ~6h; refresh skew 1h — warm well before that.
        tokio::time::sleep(Duration::from_secs(50 * 60)).await;
    }
}

/// Interactive device-code login. Cosmic Scribe's own session only.
pub fn login_device_code(open_browser: bool) -> Result<()> {
    let client = http_client()?;
    let (_auth_ep, token_endpoint) = discovery(&client)?;

    let resp = client
        .post(DEVICE_CODE_URL)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Accept", "application/json")
        .form(&[("client_id", CLIENT_ID), ("scope", SCOPE)])
        .send()
        .context("xAI device-code request failed")?;
    if !resp.status().is_success() {
        let body = resp.text().unwrap_or_default();
        bail!("xAI device-code request failed: {body}");
    }
    let device: serde_json::Value = resp.json().context("device-code JSON")?;
    let device_code = device
        .get("device_code")
        .and_then(|x| x.as_str())
        .context("missing device_code")?
        .to_string();
    let user_code = device
        .get("user_code")
        .and_then(|x| x.as_str())
        .unwrap_or("?")
        .to_string();
    let verification = device
        .get("verification_uri_complete")
        .and_then(|x| x.as_str())
        .or_else(|| device.get("verification_uri").and_then(|x| x.as_str()))
        .unwrap_or("https://accounts.x.ai/oauth2/device")
        .to_string();
    let expires_in = device
        .get("expires_in")
        .and_then(|x| x.as_u64())
        .unwrap_or(1800);
    let mut interval = device
        .get("interval")
        .and_then(|x| x.as_u64())
        .unwrap_or(5)
        .max(1);

    eprintln!();
    eprintln!("Sign in with SuperGrok or X Premium+:");
    eprintln!("  1. Open: {verification}");
    eprintln!("  2. If prompted, enter code: {user_code}");
    if open_browser {
        match open_url(&verification) {
            Ok(true) => eprintln!("  (Opened browser)"),
            Ok(false) => eprintln!("  (Could not open browser — use the URL above)"),
            Err(e) => eprintln!("  (Browser open failed: {e})"),
        }
    }
    eprintln!("Waiting for approval (polling every {interval}s, up to {expires_in}s)...");

    let deadline = Instant::now() + Duration::from_secs(expires_in);
    let tokens = loop {
        if Instant::now() >= deadline {
            bail!("Timed out waiting for xAI device authorization");
        }
        let resp = client
            .post(&token_endpoint)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("Accept", "application/json")
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ("client_id", CLIENT_ID),
                ("device_code", device_code.as_str()),
            ])
            .send()
            .context("device token poll failed")?;

        if resp.status().is_success() {
            let v: serde_json::Value = resp.json().context("token JSON")?;
            let access = v
                .get("access_token")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            let refresh = v
                .get("refresh_token")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            if access.is_empty() || refresh.is_empty() {
                bail!("token response missing access_token or refresh_token");
            }
            break OAuthTokens {
                access_token: access,
                refresh_token: refresh,
                id_token: v
                    .get("id_token")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string(),
                expires_in: v.get("expires_in").and_then(|x| x.as_u64()),
                token_type: v
                    .get("token_type")
                    .and_then(|x| x.as_str())
                    .unwrap_or("Bearer")
                    .to_string(),
            };
        }

        let body = resp.text().unwrap_or_default();
        let err: serde_json::Value = serde_json::from_str(&body).unwrap_or(serde_json::Value::Null);
        let code = err.get("error").and_then(|x| x.as_str()).unwrap_or("");
        match code {
            "authorization_pending" => {
                thread::sleep(Duration::from_secs(interval));
                continue;
            }
            "slow_down" => {
                interval = (interval + 1).min(30);
                thread::sleep(Duration::from_secs(interval));
                continue;
            }
            _ => {
                let desc = err
                    .get("error_description")
                    .and_then(|x| x.as_str())
                    .unwrap_or(body.as_str());
                bail!("xAI device authorization failed: {desc}");
            }
        }
    };

    let auth = StoredAuth {
        version: 1,
        auth_mode: "oauth_device_code".into(),
        tokens,
        token_endpoint,
        last_refresh_unix: now_unix(),
        api_base: API_BASE.into(),
    };
    write_store(&auth)?;
    eprintln!();
    eprintln!("Signed in. Speech recognition will use your plan access.");
    eprintln!("  Stored: {}", store_path().display());
    Ok(())
}

fn open_url(url: &str) -> Result<bool> {
    let status = std::process::Command::new("xdg-open").arg(url).status();
    match status {
        Ok(s) if s.success() => Ok(true),
        Ok(_) => Ok(false),
        Err(e) => Err(e.into()),
    }
}

/// Auth mode for status / settings: `oauth` | `api_key` | `api_key_env` | `none`
///
/// Order matches effective bearer resolution for **display** (env overrides OAuth):
/// env key → OAuth store present → stored key file → none.
/// Does not refresh tokens or hit the network.
pub fn auth_status_label() -> &'static str {
    if std::env::var("COSMIC_SCRIBE_API_KEY")
        .ok()
        .filter(|s| !s.is_empty())
        .is_some()
        || crate::env_compat("COSMIC_SCRIBE_XAI_API_KEY", "VOICE_INPUT_XAI_API_KEY")
            .filter(|s| !s.is_empty())
            .is_some()
    {
        "api_key_env"
    } else if is_logged_in() {
        "oauth"
    } else if crate::keyring::has_stored_api_key() {
        "api_key"
    } else {
        "none"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base64_url_encode(data: &[u8]) -> String {
        const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
        let mut out = String::new();
        let mut i = 0;
        while i < data.len() {
            let b0 = data[i];
            let b1 = if i + 1 < data.len() { data[i + 1] } else { 0 };
            let b2 = if i + 2 < data.len() { data[i + 2] } else { 0 };
            out.push(T[(b0 >> 2) as usize] as char);
            out.push(T[(((b0 & 3) << 4) | (b1 >> 4)) as usize] as char);
            if i + 1 < data.len() {
                out.push(T[(((b1 & 0x0f) << 2) | (b2 >> 6)) as usize] as char);
            }
            if i + 2 < data.len() {
                out.push(T[(b2 & 0x3f) as usize] as char);
            }
            i += 3;
        }
        out
    }

    #[test]
    fn jwt_exp_parses_dummy() {
        let payload = base64_url_encode(br#"{"exp":9999999999}"#);
        let token = format!("e30.{payload}.sig");
        assert_eq!(jwt_exp(&token), Some(9_999_999_999));
    }

    #[test]
    fn base64_roundtrip_small() {
        let raw = br#"{"exp":123}"#;
        let enc = base64_url_encode(raw);
        let dec = base64_url_decode(&enc).unwrap();
        assert_eq!(dec, raw);
    }

    #[test]
    fn oauth_bearer_heuristic() {
        assert!(looks_like_oauth_bearer("aaa.bbb.ccc"));
        assert!(!looks_like_oauth_bearer("xai-abc123"));
        assert!(!looks_like_oauth_bearer(""));
        assert!(!looks_like_oauth_bearer("not-a-jwt"));
    }
}
