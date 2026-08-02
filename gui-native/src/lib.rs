mod app;
mod audio;
mod time;
mod ux;

use cosmic::app::Settings;
use cosmic::iced::{Limits, Size};

pub fn run(debug_gui: bool) -> cosmic::iced::Result {
    std::env::set_var("GTK_APPLICATION_ID", "com.cosmic-scribe.gui");
    std::env::set_var("GDK_APPLICATION_NAME", "Cosmic Scribe");

    if let Err(pid) = cosmic_scribe::lifecycle::try_acquire_gui_lock(debug_gui) {
        let name = if debug_gui {
            "cosmic-scribe-gui-native-debug"
        } else {
            "cosmic-scribe-gui-native"
        };
        eprintln!("{name} already running (pid {pid}); not opening another window.");
        std::process::exit(0);
    }

    cosmic_scribe::api::prune_junk_on_ui_start();

    let open_settings = std::env::args().any(|a| a == "--settings");
    if debug_gui {
        eprintln!(
            "data dir: {}",
            cosmic_scribe::lifecycle::data_dir().display()
        );
    }

    let icon_theme = cosmic::icon_theme::default();
    if icon_theme.is_empty() || icon_theme == "hicolor" {
        cosmic::icon_theme::set_default("Cosmic");
    }

    let settings = Settings::default()
        .size(Size::new(920.0, 720.0))
        .size_limits(Limits::NONE.min_width(480.0).min_height(400.0));

    let result = cosmic::app::run::<app::App>(settings, app::Flags { open_settings });

    cosmic_scribe::lifecycle::release_gui_lock(debug_gui);
    result
}
