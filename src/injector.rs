// ── Text output ───────────────────────────────────────────────
// Wayland has no API to "set text" in the focused widget. Only:
//   1. clipboard — user pastes with their app binding (terminal: often Ctrl+Shift+V)
//   2. wtype     — virtual keyboard, one key event per character (`-d 0` = no delay)
//
// We do not chain ydotool / Ctrl+V / Shift+Insert — success varies by app and is confusing.

use crate::traits::TextInjector;
use anyhow::{Context, Result};
use std::io::Write;
use std::process::{Command, Stdio};

pub struct WaylandInjector;

#[async_trait::async_trait]
impl TextInjector for WaylandInjector {
    /// Copy to clipboard, then type with wtype (predictable single path).
    async fn inject(&self, text: &str) -> Result<()> {
        copy_to_clipboard(text)?;
        wtype_text(text).context("wtype failed (install wtype; text is on clipboard)")?;
        tracing::info!("text sent via wtype");
        Ok(())
    }

    async fn inject_clipboard(&self, text: &str) -> Result<()> {
        copy_to_clipboard(text)?;
        tracing::info!("transcript on clipboard only");
        Ok(())
    }
}

fn wtype_delay_ms() -> u64 {
    crate::env_compat("COSMIC_SCRIBE_WTYPE_DELAY_MS", "VOICE_INPUT_WTYPE_DELAY_MS")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

fn wtype_text(text: &str) -> Result<()> {
    let mut cmd = Command::new("wtype");
    let delay = wtype_delay_ms();
    if delay > 0 {
        cmd.arg("-d").arg(delay.to_string());
    }
    let status = cmd
        .arg("--")
        .arg(text)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .status()
        .context("failed to run wtype")?;
    if status.success() {
        Ok(())
    } else {
        anyhow::bail!("wtype exited with {status}")
    }
}

fn copy_to_clipboard(text: &str) -> Result<()> {
    let (cmd, args): (&str, &[&str]) = if std::env::var("WAYLAND_DISPLAY").is_ok() {
        ("wl-copy", &[] as &[&str])
    } else {
        ("xclip", &["-selection", "clipboard"])
    };

    let mut child = Command::new(cmd)
        .args(args)
        .stdin(Stdio::piped())
        .spawn()
        .with_context(|| format!("failed to spawn {cmd}"))?;

    if let Some(ref mut stdin) = child.stdin {
        stdin.write_all(text.as_bytes())?;
    }
    let status = child.wait()?;
    if !status.success() {
        anyhow::bail!("{cmd} failed with {status}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wtype_delay_defaults_to_zero() {
        std::env::remove_var("VOICE_INPUT_WTYPE_DELAY_MS");
        assert_eq!(wtype_delay_ms(), 0);
    }
}
