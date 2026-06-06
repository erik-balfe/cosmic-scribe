// ── API key storage ───────────────────────────────────────────
// Stores app config in ~/.local/share/cosmic-scribe/
//   api-key — xAI API key, AES-256-GCM encrypted (0600)
//   lang    — STT language code
//
// Encryption: AES-256-GCM, key derived via HKDF-SHA256 from /etc/machine-id.
// No extra secrets to manage — machine-id is unique per install.
// Non-migrated plaintext keys are read as-is and encrypted on next write.

use crate::traits::KeyringStore;
use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Nonce};
use anyhow::{Context, Result};
use hkdf::Hkdf;
use rand::RngCore;
use sha2::Sha256;
use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

const NONCE_LEN: usize = 12;
const SALT: &[u8] = b"voice-input-api-key-v1";

fn config_dir() -> PathBuf {
    crate::lifecycle::data_dir()
}

fn key_path() -> PathBuf {
    config_dir().join("api-key")
}

fn lang_path() -> PathBuf {
    config_dir().join("lang")
}

fn machine_id() -> Vec<u8> {
    fs::read_to_string("/etc/machine-id")
        .or_else(|_| fs::read_to_string("/var/lib/dbus/machine-id"))
        .unwrap_or_default()
        .trim()
        .as_bytes()
        .to_vec()
}

fn derive_key() -> Result<[u8; 32]> {
    let ikm = machine_id();
    if ikm.is_empty() {
        anyhow::bail!("cannot find machine-id");
    }
    let hkdf = Hkdf::<Sha256>::new(Some(SALT), &ikm);
    let mut key = [0u8; 32];
    hkdf.expand(b"api-key-encryption", &mut key)
        .map_err(|_| anyhow::anyhow!("HKDF expand failed"))?;
    Ok(key)
}

fn encrypt(plaintext: &[u8]) -> Result<Vec<u8>> {
    let key = derive_key()?;
    let cipher =
        Aes256Gcm::new_from_slice(&key).map_err(|_| anyhow::anyhow!("invalid key length"))?;

    let mut nonce_bytes = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|_| anyhow::anyhow!("encryption failed"))?;

    let mut result = Vec::with_capacity(NONCE_LEN + ciphertext.len());
    result.extend_from_slice(&nonce_bytes);
    result.extend_from_slice(&ciphertext);
    Ok(result)
}

fn decrypt(data: &[u8]) -> Result<Vec<u8>> {
    if data.len() < NONCE_LEN + 1 {
        anyhow::bail!("encrypted data too short");
    }
    let key = derive_key()?;
    let cipher =
        Aes256Gcm::new_from_slice(&key).map_err(|_| anyhow::anyhow!("invalid key length"))?;

    let (nonce_bytes, ciphertext) = data.split_at(NONCE_LEN);
    let nonce = Nonce::from_slice(nonce_bytes);

    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| anyhow::anyhow!("decryption failed — wrong machine? key corrupted?"))
}

fn is_encrypted(data: &[u8]) -> bool {
    // Encrypted format: binary with nonce prefix. Plaintext is ASCII.
    data.len() > NONCE_LEN && !data.iter().all(u8::is_ascii)
}

pub fn get_language() -> String {
    crate::env_compat("COSMIC_SCRIBE_LANG", "VOICE_INPUT_LANG")
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            let path = lang_path();
            fs::read_to_string(&path)
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|_| "en".to_string())
        })
}

pub fn set_language(lang: &str) -> Result<()> {
    let path = lang_path();
    fs::write(&path, lang.trim())?;
    tracing::info!("language set to '{lang}' ({})", path.display());
    Ok(())
}

fn output_mode_path() -> PathBuf {
    config_dir().join("output-mode")
}

/// `wtype` (default) or `clipboard`. Legacy values `auto`/`always`/`never` are normalized once.
pub fn get_output_mode() -> String {
    let raw = fs::read_to_string(output_mode_path())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "wtype".to_string());
    let mode = match raw.as_str() {
        "clipboard" | "never" => "clipboard",
        "wtype" | "always" | "auto" => "wtype",
        _ => "wtype",
    };
    if mode != raw {
        let _ = fs::write(output_mode_path(), mode);
    }
    mode.to_string()
}

pub fn set_output_mode(mode: &str) -> Result<()> {
    let mode = match mode.trim() {
        "clipboard" => "clipboard",
        "wtype" => "wtype",
        other => anyhow::bail!("output mode must be 'wtype' or 'clipboard', got '{other}'"),
    };
    fs::write(output_mode_path(), mode)?;
    Ok(())
}

fn correction_key_path() -> PathBuf {
    config_dir().join("correction-key")
}

pub fn get_correction_key() -> String {
    crate::env_compat("COSMIC_SCRIBE_CORRECTION_KEY", "VOICE_INPUT_CORRECTION_KEY")
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            let path = correction_key_path();
            let Ok(data) = fs::read(&path) else {
                return String::new();
            };
            if is_encrypted(&data) {
                decrypt(&data)
                    .ok()
                    .and_then(|b| String::from_utf8(b).ok())
                    .unwrap_or_default()
            } else {
                String::from_utf8(data)
                    .unwrap_or_default()
                    .trim()
                    .to_string()
            }
        })
}

pub fn set_correction_key(key: &str) -> Result<()> {
    let path = correction_key_path();
    let encrypted = encrypt(key.trim().as_bytes())?;
    let mut file = fs::File::create(&path)?;
    file.write_all(&encrypted)?;
    let mut perms = file.metadata()?.permissions();
    perms.set_mode(0o600);
    fs::set_permissions(&path, perms)?;
    tracing::info!("correction API key saved (encrypted) to {}", path.display());
    Ok(())
}

fn correction_model_path() -> PathBuf {
    config_dir().join("correction-model")
}

pub fn get_correction_model() -> String {
    fs::read_to_string(correction_model_path())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "deepseek/deepseek-chat-v4".to_string())
}

pub fn set_correction_model(model: &str) -> Result<()> {
    fs::write(correction_model_path(), model.trim())?;
    Ok(())
}

pub struct ConfigFileKeyring;

impl KeyringStore for ConfigFileKeyring {
    fn get_api_key(&self) -> Result<String> {
        if let Some(key) = crate::env_compat("COSMIC_SCRIBE_XAI_API_KEY", "VOICE_INPUT_XAI_API_KEY")
        {
            if !key.is_empty() {
                return Ok(key);
            }
        }
        let path = key_path();
        let data = fs::read(&path).with_context(|| {
            format!(
                "no API key found at {} or in COSMIC_SCRIBE_XAI_API_KEY / VOICE_INPUT_XAI_API_KEY env",
                path.display()
            )
        })?;

        if is_encrypted(&data) {
            let plain = decrypt(&data)?;
            String::from_utf8(plain).context("decrypted key is not valid UTF-8")
        } else {
            // Legacy plaintext — accept but don't upgrade (next set will encrypt)
            String::from_utf8(data)
                .context("API key file contains invalid UTF-8")
                .map(|s| s.trim().to_string())
        }
    }

    fn set_api_key(&self, key: &str) -> Result<()> {
        let path = key_path();
        let encrypted = encrypt(key.trim().as_bytes())?;
        let mut file = fs::File::create(&path)?;
        file.write_all(&encrypted)?;
        let mut perms = file.metadata()?.permissions();
        perms.set_mode(0o600);
        fs::set_permissions(&path, perms)?;
        tracing::info!("API key saved (encrypted) to {}", path.display());
        Ok(())
    }

    fn clear(&self) -> Result<()> {
        let path = key_path();
        if path.exists() {
            fs::remove_file(&path)?;
        }
        Ok(())
    }
}
