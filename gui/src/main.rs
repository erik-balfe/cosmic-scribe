//! Tauri spike — native window hosting the existing Svelte UI.
//! Phase 1: load the embedded HTTP API in a WebKit webview (same backend as browser mode).

use tauri::{Manager, WebviewUrl};

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

    let start_path = start_path_from_args();
    let server_url = match cosmic_scribe::web::spawn_server(start_path) {
        Ok(url) => url,
        Err(e) => {
            eprintln!("failed to start UI server: {e}");
            std::process::exit(1);
        }
    };

    tauri::Builder::default()
        .setup(move |app| {
            let parsed = server_url.parse().expect("UI server URL must be valid");
            let title = if start_path == "/settings" {
                "Cosmic Scribe — Settings"
            } else {
                "Cosmic Scribe — History"
            };
            tauri::WebviewWindowBuilder::new(app, "main", WebviewUrl::External(parsed))
                .title(title)
                .inner_size(920.0, 720.0)
                .min_inner_size(480.0, 400.0)
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error running Cosmic Scribe GUI");
}
