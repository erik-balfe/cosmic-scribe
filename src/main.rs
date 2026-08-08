use cosmic_scribe::app::App;
use cosmic_scribe::audio::{FileAudioCapture, SubprocessCapture};
use cosmic_scribe::injector::WaylandInjector;
use cosmic_scribe::ipc;
use cosmic_scribe::keyring::{self, ConfigFileKeyring};
use cosmic_scribe::lifecycle;
use cosmic_scribe::logging;
use cosmic_scribe::state::Event;
use cosmic_scribe::stt::XaiSttClient;
use cosmic_scribe::traits::SttClient;
use cosmic_scribe::traits::{KeyringStore, TrayController};
use cosmic_scribe::tray;
use cosmic_scribe::APP_SLUG;

use std::sync::Arc;

struct CliTray;
impl TrayController for CliTray {
    fn set_state(&self, state: &str) {
        eprintln!("  [state: {state}]");
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--daemon") {
        logging::init_daemon();
    } else {
        logging::init();
    }
    check_deps();

    if args.iter().any(|a| a == "--login") {
        let no_browser = args.iter().any(|a| a == "--no-browser");
        cosmic_scribe::xai_oauth::login_device_code(!no_browser)?;
        return Ok(());
    }

    if args.iter().any(|a| a == "--logout") {
        cosmic_scribe::xai_oauth::clear()?;
        eprintln!("Signed out (API key left untouched)");
        return Ok(());
    }

    if args.iter().any(|a| a == "--set-key") {
        let key = args
            .iter()
            .skip_while(|a| *a != "--set-key")
            .nth(1)
            .cloned()
            .unwrap_or_default();
        ConfigFileKeyring.set_api_key(&key)?;
        eprintln!("API key saved. Optional: --login if you use SuperGrok or X Premium+.");
        return Ok(());
    }

    if args.iter().any(|a| a == "--clear-key") {
        ConfigFileKeyring.clear()?;
        eprintln!("API key cleared");
        return Ok(());
    }

    if args.iter().any(|a| a == "--set-lang") {
        let lang = args
            .iter()
            .skip_while(|a| *a != "--set-lang")
            .nth(1)
            .cloned()
            .unwrap_or_default();
        keyring::set_language(&lang)?;
        eprintln!("Speech language set to: {lang}");
        return Ok(());
    }

    if args.iter().any(|a| a == "--trigger") {
        return trigger_mode().await;
    }

    if args.iter().any(|a| a == "--cancel") {
        return cancel_mode().await;
    }

    if args.iter().any(|a| a == "--uninstall") {
        let purge = args.iter().any(|a| a == "--purge");
        return lifecycle::uninstall(purge);
    }

    if args.iter().any(|a| a == "--install")
        || args.iter().any(|a| a.starts_with("--install-from="))
    {
        lifecycle::stop_daemon();
        let source = lifecycle::resolve_install_source(&args)?;
        lifecycle::install_binary_from(&source)?;
        create_autostart()?;
        lifecycle::start_daemon()?;
        return Ok(());
    }

    if args.iter().any(|a| a == "--update") || args.iter().any(|a| a.starts_with("--update-from="))
    {
        lifecycle::update_from(&args)?;
        return Ok(());
    }

    if args.iter().any(|a| a == "--stop" || a == "--quit") {
        lifecycle::stop_daemon();
        return Ok(());
    }

    if args.iter().any(|a| a == "--start") {
        lifecycle::start_daemon()?;
        return Ok(());
    }

    if args.iter().any(|a| a == "--restart") {
        lifecycle::restart_daemon()?;
        return Ok(());
    }

    if args.iter().any(|a| a == "--status") {
        lifecycle::print_status();
        return Ok(());
    }

    if args.iter().any(|a| a == "--prune-junk-recordings") {
        let dir = lifecycle::recordings_dir();
        let n = cosmic_scribe::recording::prune_junk_recordings(&dir);
        println!("Removed {n} junk recording(s) from {}", dir.display());
        return Ok(());
    }

    if args.iter().any(|a| a == "--autostart") {
        create_autostart()?;
        return Ok(());
    }

    if args.iter().any(|a| a == "--history") {
        lifecycle::spawn_gui(false)?;
        return Ok(());
    }

    if args.iter().any(|a| a == "--settings") {
        lifecycle::spawn_gui(true)?;
        return Ok(());
    }

    if args.iter().any(|a| a == "--ui-server") {
        let start_path = args
            .iter()
            .find_map(|a| a.strip_prefix("--path="))
            .unwrap_or("/");
        std::env::set_var("COSMIC_SCRIBE_NO_BROWSER", "1");
        return cosmic_scribe::web::run_at(start_path);
    }

    if args.iter().any(|a| a == "--configure") {
        return configure_mode();
    }

    if args.iter().any(|a| a == "--record-once") {
        return record_once().await;
    }

    if let Some(path) = args
        .iter()
        .find(|a| a.starts_with("--file-input="))
        .map(|a| a.strip_prefix("--file-input=").unwrap())
    {
        return file_input(path).await;
    }

    if args.iter().any(|a| a == "--daemon") {
        return daemon_mode().await;
    }

    print_usage();
    Ok(())
}

fn print_usage() {
    eprintln!("Cosmic Scribe — record → speech-to-text → insert text");
    eprintln!();
    eprintln!("Service (background daemon + tray):");
    eprintln!(
        "  --install            Stop daemon, install binary, enable systemd autostart, start"
    );
    eprintln!("  --install-from=PATH  Install from PATH (default: release/cargo if newer)");
    eprintln!("  --update             Stop → install from this binary → start");
    eprintln!("  --update-from=PATH   Same, but copy from PATH (e.g. new release build)");
    eprintln!("  --start              Start daemon via systemd (or direct if not installed)");
    eprintln!("  --stop | --quit      Stop daemon (tray goes away)");
    eprintln!("  --restart            Stop then start daemon");
    eprintln!("  --status             Running? installed path? IPC socket?");
    eprintln!("  --uninstall          Stop daemon; remove ~/.local install + autostart");
    eprintln!("  --purge              With --uninstall: also delete ~/.local/share/{APP_SLUG}/");
    eprintln!("  --daemon             Run in foreground (used by systemd unit; not for daily use)");
    eprintln!();
    eprintln!("Dictation:");
    eprintln!("  --trigger            Toggle recording on running daemon");
    eprintln!("  --cancel             Abort recording or STT (bind e.g. Ctrl+Shift+Space)");
    eprintln!("  --record-once        Record, transcribe, insert text, exit");
    eprintln!("  --file-input=<path>  Transcribe pre-recorded raw PCM");
    eprintln!();
    eprintln!("Setup:");
    eprintln!("  --login              Sign in (SuperGrok / X Premium+ plan access)");
    eprintln!("  --logout             Sign out (API key left untouched)");
    eprintln!("  --no-browser         With --login: print URL only (SSH/headless)");
    eprintln!("  --configure          Interactive auth + language");
    eprintln!("  --history            History window");
    eprintln!("  --settings           Settings window");
    eprintln!("  --autostart          Enable com.cosmic-scribe.service (graphical-session.target)");
    eprintln!("  --set-key KEY        Store speech API key (or COSMIC_SCRIBE_API_KEY)");
    eprintln!("  --clear-key          Remove stored API key");
    eprintln!("  --set-lang LANG      Set speech language (default: en)");
    eprintln!();
    eprintln!("Speech endpoint (xAI dialect; see docs/STT_PROVIDERS.md):");
    eprintln!("  COSMIC_SCRIBE_STT_URL  Full STT URL (default https://api.x.ai/v1/stt)");
}

async fn trigger_mode() -> anyhow::Result<()> {
    ipc::send_toggle().await?;
    tracing::info!("toggle sent");
    Ok(())
}

async fn cancel_mode() -> anyhow::Result<()> {
    match ipc::send_cancel().await {
        Ok(()) => {
            tracing::info!("cancel sent");
            Ok(())
        }
        Err(e) => {
            eprintln!(
                "Could not cancel — is the daemon running? ({e})\n\
                 Try: cosmic-scribe --status"
            );
            Err(e)
        }
    }
}

async fn file_input(path: &str) -> anyhow::Result<()> {
    let audio = FileAudioCapture::new(std::path::PathBuf::from(path));
    let keyring = Arc::new(ConfigFileKeyring);
    let stt: Arc<dyn SttClient> = Arc::new(XaiSttClient::new(keyring.clone()));

    let mut app = App::new(
        Box::new(audio),
        stt,
        Box::new(WaylandInjector),
        keyring,
        Box::new(CliTray),
    );

    let done = app.done_rx();
    let tx = app.event_sender();
    let handle = tokio::spawn(async move { app.run().await });

    tx.send(Event::Toggle).ok(); // Idle → Recording
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    tx.send(Event::Toggle).ok(); // Recording → Transcribing (triggers file read)

    // Wait for processing to complete or timeout
    let timeout =
        cosmic_scribe::env_compat("COSMIC_SCRIBE_STT_TIMEOUT_MS", "VOICE_INPUT_STT_TIMEOUT_MS")
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(60_000);
    let _ = tokio::time::timeout(std::time::Duration::from_millis(timeout + 5000), done).await;
    handle.abort();
    tracing::info!("done");
    Ok(())
}

async fn record_once() -> anyhow::Result<()> {
    let keyring = Arc::new(ConfigFileKeyring);
    let stt: Arc<dyn SttClient> = Arc::new(XaiSttClient::new(keyring.clone()));
    let mut app = App::new(
        Box::new(SubprocessCapture::new()),
        stt,
        Box::new(WaylandInjector),
        keyring,
        Box::new(CliTray),
    );

    let done = app.done_rx();
    let tx = app.event_sender();

    let handle = tokio::spawn(async move { app.run().await });

    tx.send(Event::Toggle).ok();
    tracing::info!("Recording... Press Ctrl+C to stop.");

    tokio::signal::ctrl_c().await.ok();
    tx.send(Event::Toggle).ok();

    // Wait for processing to complete (with generous timeout)
    let _ = tokio::time::timeout(std::time::Duration::from_secs(90), done).await;

    handle.abort();
    tracing::info!("Done.");
    Ok(())
}

async fn daemon_mode() -> anyhow::Result<()> {
    tracing::info!(
        pid = std::process::id(),
        exe = ?std::env::current_exe().ok(),
        xdg_runtime = ?std::env::var("XDG_RUNTIME_DIR").ok(),
        desktop_session = ?std::env::var("DESKTOP_SESSION").ok(),
        wayland_display = ?std::env::var("WAYLAND_DISPLAY").ok(),
        "daemon_mode entered"
    );

    let lock_path = lifecycle::daemon_lock_path();
    let _daemon_lock = match lifecycle::try_acquire_daemon_lock() {
        Ok(guard) => {
            tracing::info!(lock = %lock_path.display(), "daemon lock acquired");
            guard
        }
        Err(pid) => {
            tracing::info!(
                existing_pid = pid,
                lock = %lock_path.display(),
                "daemon already running — exiting (singleton)"
            );
            return Ok(());
        }
    };

    let keyring = Arc::new(ConfigFileKeyring);
    let stt: Arc<dyn SttClient> = Arc::new(XaiSttClient::new(keyring.clone()));
    let mut app = App::new(
        Box::new(SubprocessCapture::new()),
        stt,
        Box::new(WaylandInjector),
        keyring,
        Box::new(CliTray),
    );

    let tx = app.event_sender();

    let deferred = tray::DeferredTray::new();
    let tray_slot = deferred.slot();
    app.set_tray_controller(Box::new(deferred));

    let ipc_tx = tx.clone();
    tokio::spawn(ipc::spawn_listener(ipc_tx));
    tracing::info!("IPC listener task spawned");

    let tray_tx = tx.clone();
    tokio::spawn(tray::connect_tray_background(tray_tx, tray_slot));
    tracing::info!("tray connect task spawned (retries until panel ready)");

    // Keep SuperGrok access token warm so STT never blocks on refresh after long idle.
    tokio::spawn(cosmic_scribe::xai_oauth::keep_warm_loop());
    tracing::info!("xAI OAuth keep-warm task spawned");

    tracing::info!(
        "Cosmic Scribe daemon — use '{APP_SLUG} --trigger', tray click, or Ctrl+C to stop"
    );

    let shutdown = tokio::select! {
        () = app.run() => {
            tracing::warn!("app.run() returned unexpectedly");
            "app_exit"
        }
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("shutting down (Ctrl+C)");
            "signal"
        }
    };
    tracing::info!(reason = shutdown, "daemon exiting");

    Ok(())
}

fn configure_mode() -> anyhow::Result<()> {
    use std::io::{self, Write};

    let lang = keyring::get_language();
    let auth = cosmic_scribe::xai_oauth::auth_status_label();
    let has_creds = ConfigFileKeyring.get_api_key().is_ok();

    println!("=== Cosmic Scribe configuration ===\n");
    println!(
        "Auth: {} {}",
        auth,
        if has_creds {
            "(usable credential present)"
        } else {
            "(not configured)"
        }
    );
    println!("  API key:  --set-key  (or paste in Settings)");
    println!("  optional: --login   (SuperGrok / X Premium+ plan access)\n");
    println!("Language: {}\n", lang);

    print!("Sign in with SuperGrok / X Premium+ now? [Y/n]: ");
    io::stdout().flush().ok();
    let mut ans = String::new();
    io::stdin().read_line(&mut ans)?;
    let ans = ans.trim().to_lowercase();
    if ans.is_empty() || ans == "y" || ans == "yes" {
        cosmic_scribe::xai_oauth::login_device_code(true)?;
    } else {
        print!("Enter API key (or press Enter to skip): ");
        io::stdout().flush().ok();
        let mut key_input = String::new();
        io::stdin().read_line(&mut key_input)?;
        let key_input = key_input.trim().to_string();
        if !key_input.is_empty() {
            ConfigFileKeyring.set_api_key(&key_input)?;
            println!("API key updated.");
        }
    }

    print!("Speech language code (default en; press Enter to keep '{lang}'): ");
    io::stdout().flush().ok();
    let mut lang_input = String::new();
    io::stdin().read_line(&mut lang_input)?;
    let lang_input = lang_input.trim().to_string();

    if !lang_input.is_empty() {
        keyring::set_language(&lang_input)?;
        println!("Language set to: {lang_input}");
    }

    Ok(())
}

fn create_autostart() -> anyhow::Result<()> {
    lifecycle::enable_login_autostart()
}

fn check_deps() {
    use std::process::Command;

    let missing: Vec<&str> = [
        ("arecord", "alsa-utils (dnf install alsa-utils)"),
        ("wl-copy", "wl-clipboard (dnf install wl-clipboard)"),
        (
            "wtype",
            "wtype (dnf install wtype) — only if output mode is wtype",
        ),
        ("notify-send", "libnotify (dnf install libnotify)"),
    ]
    .iter()
    .filter(|(bin, _)| {
        !Command::new("which")
            .arg(bin)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    })
    .map(|(bin, pkg)| {
        tracing::warn!("{bin} not found — install: {pkg}");
        *bin
    })
    .collect();
    if !missing.is_empty() {
        tracing::warn!("{} missing dependencies", missing.len());
    }
}
