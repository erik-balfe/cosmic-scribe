//! Daemon install path, stop/start/restart/update.

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn share_binary() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("voice-input")
        .join("voice-input")
}

pub fn wrapper_binary() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".local/bin/voice-input")
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

pub fn install_binary_from(source: &Path) -> anyhow::Result<PathBuf> {
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
        "Could not install to {} (file busy). Run: voice-input --stop, wait a second, then --install again",
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
             cargo build --release && ./target/release/voice-input --install"
        );
    }

    Ok(current)
}

fn sibling_release_path(current: &Path) -> Option<PathBuf> {
    let parent = current.parent()?;
    if parent.file_name()?.to_str()? != "debug" {
        return None;
    }
    Some(parent.parent()?.join("release").join("voice-input"))
}

fn sibling_release_binary(current: &Path) -> Option<PathBuf> {
    let release = sibling_release_path(current)?;
    release.is_file().then_some(release)
}

fn cargo_binary() -> Option<PathBuf> {
    dirs::home_dir()
        .map(|h| h.join(".cargo/bin/voice-input"))
        .filter(|p| p.is_file())
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
    let p = PathBuf::from(prefix).join("bin/voice-input");
    p.is_file().then_some(p)
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
    let Ok(output) = Command::new("pgrep")
        .args(["-f", "voice-input.*--daemon"])
        .output()
    else {
        return vec![];
    };
    let self_pid = std::process::id() as i32;
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|s| s.trim().parse::<i32>().ok())
        .filter(|&p| p != self_pid)
        .collect()
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
        let _ = Command::new("kill").arg("-9").arg(pid.to_string()).status();
    }
    for _ in 0..10 {
        std::thread::sleep(std::time::Duration::from_millis(100));
        if daemon_pids().is_empty() {
            break;
        }
    }
    let sock = crate::ipc::socket_path();
    let _ = std::fs::remove_file(&sock);
    let n = pids.len() as u32;
    eprintln!("Stopped {n} daemon process(es)");
    n
}

pub fn start_daemon() -> anyhow::Result<()> {
    if is_daemon_running() {
        eprintln!("Daemon already running");
        return Ok(());
    }
    let bin = daemon_binary();
    if !bin.exists() {
        anyhow::bail!(
            "No installed binary at {}. Run: voice-input --install",
            bin.display()
        );
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
    let pids = daemon_pids();
    let share = share_binary();
    let wrapper = wrapper_binary();
    eprintln!("Cosmic Scribe status");
    eprintln!(
        "  daemon: {}",
        if pids.is_empty() {
            "stopped"
        } else {
            "running"
        }
    );
    if !pids.is_empty() {
        eprintln!(
            "  pids: {}",
            pids.iter()
                .map(|p| p.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    if share.exists() {
        if let Ok(m) = share.metadata() {
            if let Ok(t) = m.modified() {
                eprintln!("  installed: {} (modified {:?})", share.display(), t);
            }
        }
    } else {
        eprintln!("  installed: (none — run --install)");
    }
    if wrapper.exists() {
        eprintln!(
            "  command: {} → {:?}",
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
        .join("voice-input.desktop")
}

pub fn data_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("voice-input")
}

/// Remove daemon install (wrapper, share copy, autostart, IPC socket).
/// Does not remove the binary you are running from (e.g. Homebrew cellar).
/// Use `--purge` to delete `~/.local/share/voice-input/` (keys, recordings, settings).
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

    if purge {
        let dir = data_dir();
        if dir.exists() {
            std::fs::remove_dir_all(&dir)?;
            eprintln!("Removed data directory: {}", dir.display());
        }
    } else {
        eprintln!(
            "Kept settings and history in {} (use --purge to delete)",
            data_dir().display()
        );
    }

    eprintln!();
    eprintln!("If your shell still says 'voice-input: No such file or directory', run: hash -r");
    eprintln!("Homebrew package (if any) is separate: brew uninstall voice-input");
    eprintln!("Then install again: voice-input --install  (or $(brew --prefix)/bin/voice-input --install)");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn share_and_wrapper_paths_under_home() {
        let path = share_binary();
        assert!(path.to_string_lossy().contains("voice-input"));
    }

    #[test]
    fn sibling_release_from_debug_path() {
        let debug = PathBuf::from("/proj/target/debug/voice-input");
        let release = sibling_release_path(&debug).unwrap();
        assert_eq!(release, PathBuf::from("/proj/target/release/voice-input"));
    }

    #[test]
    fn looks_like_debug_detects_target_debug() {
        assert!(looks_like_debug_build(&PathBuf::from(
            "/home/u/proj/target/debug/voice-input"
        )));
        assert!(!looks_like_debug_build(&PathBuf::from(
            "/home/u/proj/target/release/voice-input"
        )));
    }
}
