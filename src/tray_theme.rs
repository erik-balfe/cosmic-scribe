//! Instant tray icon updates when the desktop light/dark theme changes.

use futures_util::StreamExt;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use zbus::zvariant::OwnedValue;

static LAST_DARK_UI: AtomicBool = AtomicBool::new(true);

#[zbus::proxy(
    interface = "org.freedesktop.portal.Settings",
    default_service = "org.freedesktop.portal.Desktop",
    default_path = "/org/freedesktop/portal/desktop"
)]
trait PortalSettings {
    /// `org.freedesktop.appearance` / `color-scheme`: 0=default, 1=dark, 2=light.
    fn read(&self, namespace: &str, key: &str) -> zbus::Result<OwnedValue>;

    #[zbus(signal)]
    fn setting_changed(&self, namespace: &str, key: &str, value: OwnedValue) -> zbus::Result<()>;
}

fn portal_scheme_to_dark(value: &OwnedValue) -> Option<bool> {
    let u: u32 = value.clone().try_into().ok()?;
    match u {
        1 => Some(true),
        2 => Some(false),
        _ => None,
    }
}

async fn read_portal_color_scheme() -> Option<bool> {
    let conn = zbus::Connection::session().await.ok()?;
    let proxy = PortalSettingsProxy::new(&conn).await.ok()?;
    let value = proxy
        .read("org.freedesktop.appearance", "color-scheme")
        .await
        .ok()?;
    portal_scheme_to_dark(&value)
}

fn gsettings_color_scheme() -> Option<bool> {
    let out = Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", "color-scheme"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let scheme = String::from_utf8_lossy(&out.stdout)
        .trim()
        .trim_matches('\'')
        .to_string();
    match scheme.as_str() {
        "prefer-dark" => Some(true),
        "prefer-light" => Some(false),
        "default" => None,
        _ => None,
    }
}

fn gtk_theme_implies_dark() -> bool {
    let Ok(out) = Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", "gtk-theme"])
        .output()
    else {
        return true;
    };
    if !out.status.success() {
        return true;
    }
    let theme = String::from_utf8_lossy(&out.stdout)
        .trim()
        .trim_matches('\'')
        .to_ascii_lowercase();
    theme.contains("dark") || !(theme.contains("light") || theme == "adwaita")
}

/// Sync-safe theme probe (gsettings only). Safe inside tokio `VoiceTray::new`.
pub fn ui_prefers_dark() -> bool {
    gsettings_color_scheme().unwrap_or_else(gtk_theme_implies_dark)
}

/// Full probe including xdg-desktop-portal; use from async tasks only.
pub async fn ui_prefers_dark_async() -> bool {
    read_portal_color_scheme()
        .await
        .or_else(gsettings_color_scheme)
        .unwrap_or_else(gtk_theme_implies_dark)
}

fn notify_theme_change(on_change: &Arc<dyn Fn(bool) + Send + Sync>, dark_ui: bool) {
    let prev = LAST_DARK_UI.swap(dark_ui, Ordering::Relaxed);
    if prev == dark_ui {
        return;
    }
    tracing::debug!(
        "tray theme → {}",
        if dark_ui { "dark ui" } else { "light ui" }
    );
    on_change(dark_ui);
}

async fn sync_theme_async(on_change: &Arc<dyn Fn(bool) + Send + Sync>) {
    notify_theme_change(on_change, ui_prefers_dark_async().await);
}

async fn watch_portal_settings(on_change: Arc<dyn Fn(bool) + Send + Sync>) {
    loop {
        match portal_settings_loop(&on_change).await {
            Ok(()) => tracing::debug!("portal settings stream ended, reconnecting"),
            Err(e) => tracing::warn!("portal settings watch failed: {e:#}"),
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}

async fn portal_settings_loop(on_change: &Arc<dyn Fn(bool) + Send + Sync>) -> anyhow::Result<()> {
    let conn = zbus::Connection::session().await?;
    let proxy = PortalSettingsProxy::new(&conn).await?;
    let mut stream = proxy.receive_setting_changed().await?;
    while let Some(signal) = stream.next().await {
        let args = signal.args()?;
        if *args.namespace() != "org.freedesktop.appearance" || *args.key() != "color-scheme" {
            continue;
        }
        let dark_ui = if let Some(dark) = portal_scheme_to_dark(args.value()) {
            dark
        } else {
            ui_prefers_dark_async().await
        };
        notify_theme_change(on_change, dark_ui);
    }
    Ok(())
}

async fn watch_gsettings_monitor(on_change: Arc<dyn Fn(bool) + Send + Sync>) {
    loop {
        match gsettings_monitor_loop(&on_change).await {
            Ok(()) => tracing::debug!("gsettings monitor exited, restarting"),
            Err(e) => tracing::warn!("gsettings monitor failed: {e:#}"),
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}

async fn gsettings_monitor_loop(on_change: &Arc<dyn Fn(bool) + Send + Sync>) -> anyhow::Result<()> {
    use tokio::io::{AsyncBufReadExt, BufReader};
    use tokio::process::Command;

    let mut child = Command::new("gsettings")
        .args(["monitor", "org.gnome.desktop.interface"])
        .stdout(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow::anyhow!("gsettings monitor missing stdout"))?;
    let mut lines = BufReader::new(stdout).lines();
    while let Some(line) = lines.next_line().await? {
        if !line.contains("color-scheme") && !line.contains("gtk-theme") {
            continue;
        }
        tracing::debug!("gsettings: {line}");
        sync_theme_async(on_change).await;
    }

    child.wait().await?;
    Ok(())
}

/// Subscribe to theme changes; call `on_change` immediately when light/dark flips.
pub async fn run_theme_watchers(on_change: Arc<dyn Fn(bool) + Send + Sync>) {
    notify_theme_change(&on_change, ui_prefers_dark_async().await);
    let portal = watch_portal_settings(on_change.clone());
    let gsettings = watch_gsettings_monitor(on_change);
    tokio::join!(portal, gsettings);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portal_scheme_values_map_correctly() {
        assert_eq!(portal_scheme_to_dark(&OwnedValue::from(1u32)), Some(true));
        assert_eq!(portal_scheme_to_dark(&OwnedValue::from(2u32)), Some(false));
        assert_eq!(portal_scheme_to_dark(&OwnedValue::from(0u32)), None);
    }
}
