//! Native Tauri window for History and Settings (embedded Svelte + local API server).
//! Shares prod data at ~/.local/share/cosmic-scribe/.

use tauri::image::Image;
use tauri::WebviewUrl;

const APP_ICON: Image<'static> = tauri::include_image!("icons/128x128.png");

#[cfg(feature = "debug-gui")]
const DEBUG_GUI: bool = true;
#[cfg(not(feature = "debug-gui"))]
const DEBUG_GUI: bool = false;

fn start_path_from_args() -> &'static str {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--settings") {
        "/settings"
    } else {
        "/"
    }
}

fn main() {
    std::env::set_var("COSMIC_SCRIBE_NO_BROWSER", "1");
    std::env::set_var("GTK_APPLICATION_ID", "com.cosmic-scribe.gui");
    std::env::set_var("GDK_APPLICATION_NAME", "Cosmic Scribe");

    if let Err(pid) = cosmic_scribe::lifecycle::try_acquire_gui_lock(DEBUG_GUI) {
        let name = if DEBUG_GUI {
            "cosmic-scribe-gui-debug"
        } else {
            "cosmic-scribe-gui"
        };
        eprintln!("{name} already running (pid {pid}); not opening another window.");
        std::process::exit(0);
    }

    let start_path = start_path_from_args();
    let server_url = match cosmic_scribe::web::spawn_server(start_path) {
        Ok(url) => url,
        Err(e) => {
            eprintln!("failed to start UI server: {e}");
            std::process::exit(1);
        }
    };

    if DEBUG_GUI {
        eprintln!("cosmic-scribe-gui-debug UI server: {server_url}");
    }
    eprintln!(
        "data dir: {}",
        cosmic_scribe::lifecycle::data_dir().display()
    );

    let window_title = if DEBUG_GUI {
        if start_path == "/settings" {
            "Cosmic Scribe (debug) — Settings"
        } else {
            "Cosmic Scribe (debug) — History"
        }
    } else if start_path == "/settings" {
        "Cosmic Scribe — Settings"
    } else {
        "Cosmic Scribe — History"
    };

    tauri::Builder::default()
        .setup(move |app| {
            let parsed = server_url.parse().expect("UI server URL must be valid");
            tauri::WebviewWindowBuilder::new(app, "main", WebviewUrl::External(parsed))
                .title(window_title)
                .icon(APP_ICON)?
                .inner_size(920.0, 720.0)
                .min_inner_size(480.0, 400.0)
                .build()?;
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error running Cosmic Scribe GUI")
        .run(|_app, event| {
            if let tauri::RunEvent::Exit = event {
                cosmic_scribe::lifecycle::release_gui_lock(DEBUG_GUI);
            }
        });
}
