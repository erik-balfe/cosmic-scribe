//! Export tray state PNGs for README screenshots.
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out = root.join("screenshots");
    std::fs::create_dir_all(&out)?;

    let size = 44;
    let dark_ui = true;
    for (state, name) in [
        ("idle", "tray-idle"),
        ("recording", "tray-recording"),
        ("transcribing", "tray-transcribing"),
    ] {
        let path = out.join(format!("{name}.png"));
        cosmic_scribe::tray::write_icon_png(&path, state, dark_ui, size)?;
        println!("wrote {}", path.display());
    }
    Ok(())
}
