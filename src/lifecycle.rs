//! Daemon install path, stop/start/restart/update.

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::APP_SLUG;

const LEGACY_SLUG: &str = "voice-input";

pub fn share_binary() -> PathBuf {
    data_dir().join(APP_SLUG)
}

pub fn wrapper_binary() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".local/bin")
        .join(APP_SLUG)
}

/// Binary used to spawn the daemon (wrapper → share copy).
pub fn daemon_binary() -> PathBuf {
    let wrapper = wrapper_binary();
    if wrapper.exists() {
        wrapper
    } else {
        share_binary()
    }
}

pub fn data_dir() -> PathBuf {
    migrate_legacy_data_dir();
    let dir = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(APP_SLUG);
    std::fs::create_dir_all(&dir).ok();
    dir
}

pub fn recordings_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("COSMIC_SCRIBE_RECORDINGS_DIR") {
        let dir = PathBuf::from(dir);
        std::fs::create_dir_all(&dir).ok();
        return dir;
    }
    let dir = data_dir().join("recordings");
    std::fs::create_dir_all(&dir).ok();
    dir
}

fn gui_lock_path() -> PathBuf {
    data_dir().join("gui.lock")
}

fn daemon_lock_path() -> PathBuf {
    std::env::var("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
        .join(format!("{APP_SLUG}-daemon.lock"))
}

/// Held for the lifetime of a `--daemon` process; released on drop.
pub struct DaemonLockGuard;

impl Drop for DaemonLockGuard {
    fn drop(&mut self) {
        release_daemon_lock();
    }
}

/// Returns `Err(pid)` when another daemon instance already owns the lock.
pub fn try_acquire_daemon_lock() -> Result<DaemonLockGuard, u32> {
    let path = daemon_lock_path();
    if let Ok(raw) = std::fs::read_to_string(&path) {
        if let Ok(pid) = raw.lines().next().unwrap_or("").trim().parse::<u32>() {
            if process_alive(pid) {
                return Err(pid);
            }
        }
    }
    let pid = std::process::id();
    let _ = std::fs::write(&path, format!("{pid}\n"));
    Ok(DaemonLockGuard)
}

pub fn release_daemon_lock() {
    let path = daemon_lock_path();
    if let Ok(raw) = std::fs::read_to_string(&path) {
        if let Ok(lock_pid) = raw.lines().next().unwrap_or("").trim().parse::<u32>() {
            if lock_pid == std::process::id() {
                let _ = std::fs::remove_file(&path);
            }
        }
    }
}

pub fn gui_binary_path(debug: bool) -> PathBuf {
    let name = if debug {
        "cosmic-scribe-gui-debug"
    } else {
        "cosmic-scribe-gui"
    };
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".local/bin")
        .join(name)
}

/// Spawn prod Tauri window (History or Settings). Single instance enforced by `gui.lock`.
pub fn spawn_gui(settings: bool) -> anyhow::Result<()> {
    let bin = gui_binary_path(false);
    if !bin.is_file() {
        anyhow::bail!(
            "cosmic-scribe-gui not installed at {}. Run: ./scripts/install-gui-prod.sh",
            bin.display()
        );
    }
    let mut cmd = Command::new(&bin);
    cmd.env("GTK_APPLICATION_ID", "com.cosmic-scribe.gui");
    cmd.env("GDK_APPLICATION_NAME", "Cosmic Scribe");
    if settings {
        cmd.arg("--settings");
    }
    cmd.spawn()?;
    Ok(())
}

pub fn process_alive(pid: u32) -> bool {
    std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Returns `Err(pid)` when another GUI instance is running (prod or debug).
pub fn try_acquire_gui_lock(_debug: bool) -> Result<(), u32> {
    let path = gui_lock_path();
    if let Ok(raw) = std::fs::read_to_string(&path) {
        if let Ok(pid) = raw.lines().next().unwrap_or("").trim().parse::<u32>() {
            if process_alive(pid) {
                return Err(pid);
            }
        }
    }
    let pid = std::process::id();
    let _ = std::fs::write(&path, format!("{pid}\n"));
    Ok(())
}

pub fn release_gui_lock(_debug: bool) {
    let path = gui_lock_path();
    if let Ok(raw) = std::fs::read_to_string(&path) {
        if let Ok(lock_pid) = raw.lines().next().unwrap_or("").trim().parse::<u32>() {
            if lock_pid == std::process::id() {
                let _ = std::fs::remove_file(&path);
            }
        }
    }
}

/// Returns `Err(pid)` when another GUI instance is running.
pub fn try_acquire_gui_debug_lock() -> Result<(), u32> {
    try_acquire_gui_lock(true)
}

pub fn release_gui_debug_lock() {
    release_gui_lock(true);
}

#[cfg(test)]
pub fn init_test_recordings_dir() {
    use std::sync::Once;
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let base = std::env::temp_dir().join(format!(
            "cosmic-scribe-test-recordings-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&base).ok();
        std::env::set_var("COSMIC_SCRIBE_RECORDINGS_DIR", &base);
    });
}

fn legacy_data_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(LEGACY_SLUG)
}

fn migrate_legacy_data_dir() {
    let old = legacy_data_dir();
    let new = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(APP_SLUG);
    if old.exists() && !new.exists() && std::fs::rename(&old, &new).is_ok() {
        tracing::info!(
            "migrated data directory from {} to {}",
            old.display(),
            new.display()
        );
    }
}

fn legacy_wrapper_binary() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".local/bin")
        .join(LEGACY_SLUG)
}

fn legacy_share_binary() -> PathBuf {
    legacy_data_dir().join(LEGACY_SLUG)
}

fn legacy_autostart_desktop_file() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("autostart")
        .join(format!("{LEGACY_SLUG}.desktop"))
}

fn legacy_socket_path() -> PathBuf {
    dirs::runtime_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(format!("{LEGACY_SLUG}.sock"))
}

fn debug_gui_binary() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".local/bin")
        .join("cosmic-scribe-gui-debug")
}

fn legacy_share_binary_in_current_data_dir() -> PathBuf {
    data_dir().join(LEGACY_SLUG)
}

pub fn remove_legacy_install_artifacts() {
    for path in [
        legacy_wrapper_binary(),
        legacy_share_binary(),
        legacy_autostart_desktop_file(),
        legacy_socket_path(),
        debug_gui_binary(),
        legacy_share_binary_in_current_data_dir(),
    ] {
        if path.exists() {
            let _ = std::fs::remove_file(&path);
            tracing::info!("removed legacy install artifact: {}", path.display());
        }
    }

    for lock in ["ui-browser.lock", "gui-debug.lock"] {
        let path = data_dir().join(lock);
        if path.exists() {
            let _ = std::fs::remove_file(&path);
            tracing::info!("removed stale lock: {}", path.display());
        }
    }
}

pub fn install_binary_from(source: &Path) -> anyhow::Result<PathBuf> {
    migrate_legacy_data_dir();
    let dest = share_binary();
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let tmp = dest.with_extension("new");
    let _ = std::fs::remove_file(&tmp);
    std::fs::copy(source, &tmp)?;
    let mut perms = std::fs::metadata(&tmp)?.permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&tmp, perms)?;

    replace_executable(&dest, &tmp)?;

    let wrapper = wrapper_binary();
    if let Some(parent) = wrapper.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if wrapper.exists() {
        std::fs::remove_file(&wrapper)?;
    }
    std::os::unix::fs::symlink(&dest, &wrapper)?;
    remove_legacy_install_artifacts();
    eprintln!("Installed: {} (from {})", dest.display(), source.display());
    eprintln!("Command: {}", wrapper.display());
    Ok(dest)
}

/// Replace `dest` with `tmp` (already chmod 755). Retries while the old file is still mapped (ETXTBSY).
fn replace_executable(dest: &Path, tmp: &Path) -> anyhow::Result<()> {
    const MAX_ATTEMPTS: u32 = 40;
    for attempt in 0..MAX_ATTEMPTS {
        if dest.exists() {
            match std::fs::remove_file(dest) {
                Ok(()) => {}
                Err(e) if e.raw_os_error() == Some(26) && attempt + 1 < MAX_ATTEMPTS => {
                    std::thread::sleep(std::time::Duration::from_millis(150));
                    continue;
                }
                Err(e) => return Err(e.into()),
            }
        }
        match std::fs::rename(tmp, dest) {
            Ok(()) => return Ok(()),
            Err(e) if e.raw_os_error() == Some(26) && attempt + 1 < MAX_ATTEMPTS => {
                std::thread::sleep(std::time::Duration::from_millis(150));
                continue;
            }
            Err(e) => return Err(e.into()),
        }
    }
    anyhow::bail!(
        "Could not install to {} (file busy). Run: {APP_SLUG} --stop, wait a second, then --install again",
        dest.display()
    )
}

pub fn resolve_install_source(args: &[String]) -> anyhow::Result<PathBuf> {
    resolve_binary_source(args, "--install-from=")
}

pub fn resolve_update_source(args: &[String]) -> anyhow::Result<PathBuf> {
    resolve_binary_source(args, "--update-from=")
}

fn resolve_binary_source(args: &[String], flag_prefix: &str) -> anyhow::Result<PathBuf> {
    if let Some(path) = args.iter().find_map(|a| a.strip_prefix(flag_prefix)) {
        return Ok(PathBuf::from(path));
    }
    let current = std::env::current_exe()?;

    if let Some(release) = sibling_release_binary(&current) {
        eprintln!("Using release build: {}", release.display());
        return Ok(release);
    }

    if let Some(cargo_bin) = cargo_binary() {
        if looks_like_debug_build(&current) || file_is_newer(&cargo_bin, &current) {
            eprintln!("Using cargo install binary: {}", cargo_bin.display());
            return Ok(cargo_bin);
        }
    }

    if let Some(brew_bin) = brew_binary() {
        if file_is_newer(&brew_bin, &current) {
            eprintln!("Using Homebrew binary: {}", brew_bin.display());
            return Ok(brew_bin);
        }
    }

    if looks_like_debug_build(&current) {
        eprintln!(
            "warning: installing a debug build — for production run:\n  \
             cargo build --release && ./target/release/{APP_SLUG} --install"
        );
    }

    Ok(current)
}

fn sibling_release_path(current: &Path) -> Option<PathBuf> {
    let parent = current.parent()?;
    if parent.file_name()?.to_str()? != "debug" {
        return None;
    }
    Some(parent.parent()?.join("release").join(APP_SLUG))
}

fn sibling_release_binary(current: &Path) -> Option<PathBuf> {
    let release = sibling_release_path(current)?;
    if release.is_file() {
        return Some(release);
    }
    let debug_parent = current.parent()?;
    let legacy = debug_parent.parent()?.join("release").join(LEGACY_SLUG);
    legacy.is_file().then_some(legacy)
}

fn cargo_binary() -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    for name in [APP_SLUG, LEGACY_SLUG] {
        let p = home.join(".cargo/bin").join(name);
        if p.is_file() {
            return Some(p);
        }
    }
    None
}

fn looks_like_debug_build(path: &Path) -> bool {
    path.to_string_lossy().contains("/target/debug/") || path.to_string_lossy().contains("/debug/")
}

fn brew_binary() -> Option<PathBuf> {
    let out = Command::new("brew").arg("--prefix").output().ok()?;
    if !out.status.success() {
        return None;
    }
    let prefix = String::from_utf8_lossy(&out.stdout).trim().to_string();
    for name in [APP_SLUG, LEGACY_SLUG] {
        let p = PathBuf::from(&prefix).join("bin").join(name);
        if p.is_file() {
            return Some(p);
        }
    }
    None
}

fn file_is_newer(a: &Path, b: &Path) -> bool {
    let (Ok(ma), Ok(mb)) = (a.metadata(), b.metadata()) else {
        return false;
    };
    match (ma.modified(), mb.modified()) {
        (Ok(ta), Ok(tb)) => ta > tb,
        _ => false,
    }
}

pub fn daemon_pids() -> Vec<i32> {
    let mut pids = Vec::new();
    for pattern in [
        format!("{APP_SLUG}.*--daemon"),
        format!("{LEGACY_SLUG}.*--daemon"),
    ] {
        let Ok(output) = Command::new("pgrep").args(["-f", &pattern]).output() else {
            continue;
        };
        let self_pid = std::process::id() as i32;
        pids.extend(
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter_map(|s| s.trim().parse::<i32>().ok())
                .filter(|&p| p != self_pid),
        );
    }
    pids.sort_unstable();
    pids.dedup();
    pids
}

pub fn is_daemon_running() -> bool {
    !daemon_pids().is_empty()
}

pub fn stop_daemon() -> u32 {
    let pids = daemon_pids();
    if pids.is_empty() {
        return 0;
    }
    for pid in &pids {
        let _ = Command::new("kill").arg(pid.to_string()).status();
    }
    for _ in 0..20 {
        std::thread::sleep(std::time::Duration::from_millis(100));
        if daemon_pids().is_empty() {
            break;
        }
    }
    for pid in daemon_pids() {
        let _ = Command::new("kill").args(["-9", &pid.to_string()]).status();
    }
    for _ in 0..10 {
        std::thread::sleep(std::time::Duration::from_millis(100));
        if daemon_pids().is_empty() {
            break;
        }
    }
    let sock = crate::ipc::socket_path();
    let _ = std::fs::remove_file(&sock);
    let _ = std::fs::remove_file(legacy_socket_path());
    let _ = std::fs::remove_file(daemon_lock_path());
    let n = pids.len() as u32;
    eprintln!("Stopped {n} daemon process(es)");
    n
}

pub fn start_daemon() -> anyhow::Result<()> {
    let bin = daemon_binary();
    if !bin.exists() {
        anyhow::bail!(
            "No installed binary at {}. Run: {APP_SLUG} --install",
            bin.display()
        );
    }
    if is_daemon_running() {
        eprintln!("Daemon already running");
        return Ok(());
    }
    Command::new(&bin)
        .arg("--daemon")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()?;
    eprintln!("Daemon started ({})", bin.display());
    Ok(())
}

pub fn print_status() {
    eprintln!("Cosmic Scribe status");
    eprintln!(
        "  daemon: {}",
        if is_daemon_running() {
            "running"
        } else {
            "stopped"
        }
    );
    let share = share_binary();
    eprintln!(
        "  installed binary: {} ({})",
        share.display(),
        if share.exists() { "present" } else { "absent" }
    );
    let wrapper = wrapper_binary();
    if wrapper.exists() {
        eprintln!(
            "  wrapper: {} → {:?}",
            wrapper.display(),
            std::fs::read_link(&wrapper).ok()
        );
    }
    let sock = crate::ipc::socket_path();
    eprintln!(
        "  ipc socket: {} ({})",
        sock.display(),
        if sock.exists() { "present" } else { "absent" }
    );
}

pub fn update_from(args: &[String]) -> anyhow::Result<()> {
    let source = resolve_update_source(args)?;
    stop_daemon();
    install_binary_from(&source)?;
    start_daemon()?;
    Ok(())
}

pub fn restart_daemon() -> anyhow::Result<()> {
    stop_daemon();
    start_daemon()
}

pub fn autostart_desktop_file() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("autostart")
        .join(format!("{APP_SLUG}.desktop"))
}

/// Remove daemon install (wrapper, share copy, autostart, IPC socket).
/// Does not remove the binary you are running from (e.g. Homebrew cellar).
/// Use `--purge` to delete `~/.local/share/cosmic-scribe/` (keys, recordings, settings).
pub fn uninstall(purge: bool) -> anyhow::Result<()> {
    stop_daemon();

    let wrapper = wrapper_binary();
    let share = share_binary();
    let autostart = autostart_desktop_file();
    let sock = crate::ipc::socket_path();
    let current = std::env::current_exe().ok();

    if wrapper.exists() {
        if wrapper.is_symlink() {
            std::fs::remove_file(&wrapper)?;
            eprintln!("Removed: {}", wrapper.display());
        } else if current.as_ref() == Some(&wrapper) {
            eprintln!(
                "Skipped wrapper {} (this executable — exit first, then delete manually)",
                wrapper.display()
            );
        } else {
            std::fs::remove_file(&wrapper)?;
            eprintln!("Removed: {}", wrapper.display());
        }
    }

    if share.exists() {
        if current.as_ref() == Some(&share) {
            eprintln!(
                "Skipped share binary {} (this executable — exit first, then delete manually)",
                share.display()
            );
        } else {
            std::fs::remove_file(&share)?;
            eprintln!("Removed: {}", share.display());
        }
    }

    if autostart.exists() {
        std::fs::remove_file(&autostart)?;
        eprintln!("Removed: {}", autostart.display());
    }

    if sock.exists() {
        std::fs::remove_file(&sock)?;
        eprintln!("Removed: {}", sock.display());
    }

    remove_legacy_install_artifacts();

    if purge {
        for dir in [data_dir(), legacy_data_dir()] {
            if dir.exists() {
                std::fs::remove_dir_all(&dir)?;
                eprintln!("Removed data directory: {}", dir.display());
            }
        }
    } else {
        eprintln!(
            "Kept settings and history in {} (use --purge to delete)",
            data_dir().display()
        );
    }

    eprintln!();
    eprintln!("If your shell still says '{APP_SLUG}: No such file or directory', run: hash -r");
    eprintln!("Homebrew package (if any) is separate: brew uninstall {APP_SLUG}");
    eprintln!(
        "Then install again: {APP_SLUG} --install  (or $(brew --prefix)/bin/{APP_SLUG} --install)"
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn share_and_wrapper_paths_under_home() {
        let path = share_binary();
        assert!(path.to_string_lossy().contains(APP_SLUG));
    }

    #[test]
    fn sibling_release_from_debug_path() {
        let debug = PathBuf::from(format!("/proj/target/debug/{APP_SLUG}"));
        let release = sibling_release_path(&debug).unwrap();
        assert_eq!(
            release,
            PathBuf::from(format!("/proj/target/release/{APP_SLUG}"))
        );
    }

    #[test]
    fn looks_like_debug_detects_target_debug() {
        assert!(looks_like_debug_build(&PathBuf::from(format!(
            "/home/u/proj/target/debug/{APP_SLUG}"
        ))));
        assert!(!looks_like_debug_build(&PathBuf::from(format!(
            "/home/u/proj/target/release/{APP_SLUG}"
        ))));
    }
}
