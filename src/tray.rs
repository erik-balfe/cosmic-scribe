// ── System tray icon ─────────────────────────────────────────
// StatusNotifierItem via D-Bus (ksni crate).
// Left click: toggle recording (start/stop).
// Right click: menu with Cancel, Settings, Quit.
// Mic-only icon: recording = red capsule; transcribing = blue capsule; idle = theme colors.

use crate::keyring;
use crate::state::Event;
use ksni::menu::StandardItem;
use ksni::{Icon, MenuItem, Status, ToolTip, Tray};
use std::io::Cursor;
use std::sync::OnceLock;
use tokio::sync::mpsc;

pub use crate::tray_theme::ui_prefers_dark;

/// ksni / StatusNotifierItem expects ARGB32 (see ksni::Icon docs), not RGBA.
fn rgba_to_argb(rgba: &mut [u8]) {
    for px in rgba.chunks_exact_mut(4) {
        px.rotate_right(1);
    }
}

const RED: (u8, u8, u8) = (220, 40, 40);
const BLUE: (u8, u8, u8) = (55, 140, 255);
const GREEN: (u8, u8, u8) = (60, 170, 60);
const MIC_LIGHT: (u8, u8, u8) = (238, 241, 248); // #eef1f8 on dark panels
const MIC_DARK: (u8, u8, u8) = (26, 34, 56); // #1a2238 on light panels

const TRAY_CAPSULE_22: &[u8] = include_bytes!("../gui/icons/tray-capsule-22.png");
const TRAY_CAPSULE_44: &[u8] = include_bytes!("../gui/icons/tray-capsule-44.png");
const TRAY_BODY_22: &[u8] = include_bytes!("../gui/icons/tray-body-22.png");
const TRAY_BODY_44: &[u8] = include_bytes!("../gui/icons/tray-body-44.png");
#[derive(Clone, Copy)]
enum TrayMask {
    Body,
    Capsule,
}

pub fn mic_color(dark_ui: bool) -> (u8, u8, u8) {
    if dark_ui {
        MIC_LIGHT
    } else {
        MIC_DARK
    }
}

fn decode_png_rgba(bytes: &[u8]) -> (i32, Vec<u8>) {
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().expect("tray icon PNG");
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).expect("tray icon frame");
    let w = info.width as i32;
    let h = info.height as i32;
    let raw = &buf[..info.buffer_size()];

    let rgba = match info.color_type {
        png::ColorType::Rgba => raw.to_vec(),
        png::ColorType::Rgb => raw
            .chunks_exact(3)
            .flat_map(|px| [px[0], px[1], px[2], 255])
            .collect(),
        other => panic!("unsupported tray icon color type: {other:?}"),
    };
    assert_eq!(rgba.len(), (w * h * 4) as usize, "w={w} h={h}");
    (w, rgba)
}

fn mask_layer(size: i32, part: TrayMask) -> &'static (i32, Vec<u8>) {
    static CAPSULE_22: OnceLock<(i32, Vec<u8>)> = OnceLock::new();
    static CAPSULE_44: OnceLock<(i32, Vec<u8>)> = OnceLock::new();
    static BODY_22: OnceLock<(i32, Vec<u8>)> = OnceLock::new();
    static BODY_44: OnceLock<(i32, Vec<u8>)> = OnceLock::new();
    match part {
        TrayMask::Capsule => {
            if size >= 44 {
                CAPSULE_44.get_or_init(|| decode_png_rgba(TRAY_CAPSULE_44))
            } else {
                CAPSULE_22.get_or_init(|| decode_png_rgba(TRAY_CAPSULE_22))
            }
        }
        TrayMask::Body => {
            if size >= 44 {
                BODY_44.get_or_init(|| decode_png_rgba(TRAY_BODY_44))
            } else {
                BODY_22.get_or_init(|| decode_png_rgba(TRAY_BODY_22))
            }
        }
    }
}

fn paint_mask(pixels: &mut [u8], mask: &[u8], color: (u8, u8, u8)) {
    for (px, m) in pixels.chunks_exact_mut(4).zip(mask.chunks_exact(4)) {
        if m[3] == 0 {
            continue;
        }
        px[0] = color.0;
        px[1] = color.1;
        px[2] = color.2;
        px[3] = 255;
    }
}

fn capsule_color(state: &str, dark_ui: bool) -> (u8, u8, u8) {
    match state {
        "recording" | "error" => RED,
        "transcribing" => BLUE,
        "inserting" => GREEN,
        _ => mic_color(dark_ui),
    }
}

pub fn build_icon(state: &str) -> Icon {
    build_icon_sized(state, ui_prefers_dark(), 22)
}

fn compose_icon_rgba(state: &str, dark_ui: bool, size: i32) -> Vec<u8> {
    let (body_w, body_mask) = mask_layer(size, TrayMask::Body);
    let (cap_w, cap_mask) = mask_layer(size, TrayMask::Capsule);
    assert_eq!(*body_w, size);
    assert_eq!(*cap_w, size);

    let mut pixels = vec![0u8; (size * size * 4) as usize];
    let body_color = mic_color(dark_ui);
    paint_mask(&mut pixels, body_mask, body_color);
    paint_mask(&mut pixels, cap_mask, capsule_color(state, dark_ui));
    pixels
}

fn build_icon_sized(state: &str, dark_ui: bool, size: i32) -> Icon {
    let mut pixels = compose_icon_rgba(state, dark_ui, size);
    rgba_to_argb(&mut pixels);
    Icon {
        width: size,
        height: size,
        data: pixels,
    }
}

/// Write a tray-state PNG for docs/screenshots (RGBA, upscaled for readability).
pub fn write_icon_png(
    path: &std::path::Path,
    state: &str,
    dark_ui: bool,
    size: i32,
) -> anyhow::Result<()> {
    let pixels = compose_icon_rgba(state, dark_ui, size);
    let mut encoder = png::Encoder::new(std::fs::File::create(path)?, size as u32, size as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(&pixels)?;
    Ok(())
}

pub async fn watch_ui_theme(handle: ksni::Handle<VoiceTray>) {
    let handle = std::sync::Arc::new(handle);
    crate::tray_theme::run_theme_watchers(std::sync::Arc::new(move |dark_ui| {
        let handle = handle.clone();
        tokio::spawn(async move {
            let _ = handle
                .update(move |tray: &mut VoiceTray| tray.set_dark_ui(dark_ui))
                .await;
        });
    }))
    .await;
}

pub struct VoiceTray {
    state: String,
    dark_ui: bool,
    toggle_tx: mpsc::UnboundedSender<Event>,
}

impl VoiceTray {
    pub fn new(toggle_tx: mpsc::UnboundedSender<Event>) -> Self {
        Self {
            state: "idle".into(),
            dark_ui: ui_prefers_dark(),
            toggle_tx,
        }
    }

    pub fn set_state(&mut self, state: &str) {
        self.state = state.to_string();
    }

    pub fn set_dark_ui(&mut self, dark_ui: bool) {
        self.dark_ui = dark_ui;
    }
}

impl Tray for VoiceTray {
    fn icon_pixmap(&self) -> Vec<Icon> {
        vec![
            build_icon_sized(&self.state, self.dark_ui, 22),
            build_icon_sized(&self.state, self.dark_ui, 44),
        ]
    }

    fn status(&self) -> Status {
        match self.state.as_str() {
            "recording" | "transcribing" => Status::Active,
            _ => Status::Passive,
        }
    }

    fn icon_name(&self) -> String {
        "".into()
    }

    fn title(&self) -> String {
        "Cosmic Scribe".into()
    }

    fn id(&self) -> String {
        "cosmic-scribe".into()
    }

    fn tool_tip(&self) -> ToolTip {
        let desc = match self.state.as_str() {
            "recording" => "Recording — click to stop",
            "transcribing" => "Transcribing...",
            "inserting" => {
                if keyring::get_output_mode() == "clipboard" {
                    "Copied to clipboard — paste into your field"
                } else {
                    "Inserting text..."
                }
            }
            "error" => "Error — click to dismiss",
            _ => "Cosmic Scribe — click to record",
        };
        ToolTip {
            title: "Cosmic Scribe".into(),
            description: desc.into(),
            icon_name: "".into(),
            icon_pixmap: vec![],
        }
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let mut items = Vec::new();

        if self.state == "recording" {
            items.push(MenuItem::Standard(StandardItem {
                label: "Cancel recording".into(),
                enabled: true,
                icon_name: "edit-delete".into(),
                activate: Box::new(|tray: &mut Self| {
                    let _ = tray.toggle_tx.send(Event::Cancel);
                }),
                ..Default::default()
            }));
            items.push(MenuItem::Separator);
        } else if self.state == "transcribing" {
            items.push(MenuItem::Standard(StandardItem {
                label: "Cancel transcription".into(),
                enabled: true,
                icon_name: "process-stop".into(),
                activate: Box::new(|tray: &mut Self| {
                    let _ = tray.toggle_tx.send(Event::Cancel);
                }),
                ..Default::default()
            }));
            items.push(MenuItem::Separator);
        }

        items.push(MenuItem::Standard(StandardItem {
            label: "History".into(),
            enabled: true,
            icon_name: "document-open-recent".into(),
            activate: Box::new(|_: &mut Self| {
                if let Err(e) = crate::lifecycle::spawn_gui(false) {
                    tracing::warn!("failed to open history GUI: {e}");
                }
            }),
            ..Default::default()
        }));
        items.push(MenuItem::Standard(StandardItem {
            label: "Settings".into(),
            enabled: true,
            icon_name: "preferences-system".into(),
            activate: Box::new(|_: &mut Self| {
                if let Err(e) = crate::lifecycle::spawn_gui(true) {
                    tracing::warn!("failed to open settings GUI: {e}");
                }
            }),
            ..Default::default()
        }));
        items.push(MenuItem::Standard(StandardItem {
            label: "Quit".into(),
            enabled: true,
            icon_name: "application-exit".into(),
            activate: Box::new(|_: &mut Self| {
                std::process::exit(0);
            }),
            ..Default::default()
        }));

        items
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        let _ = self.toggle_tx.send(Event::ToggleTray);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn capsule_center_argb(icon: &Icon) -> (u8, u8, u8) {
        let idx = (8 * icon.width as usize + 11) * 4;
        (icon.data[idx + 1], icon.data[idx + 2], icon.data[idx + 3])
    }

    #[test]
    fn recording_capsule_is_red() {
        let icon = build_icon_sized("recording", true, 22);
        let (r, g, b) = capsule_center_argb(&icon);
        assert!(
            r > 200 && g < 80 && b < 80,
            "red capsule, got r={r} g={g} b={b}"
        );
    }

    #[test]
    fn transcribing_capsule_is_blue() {
        let icon = build_icon_sized("transcribing", true, 22);
        let (r, g, b) = capsule_center_argb(&icon);
        assert!(b > 200 && r < 120, "blue capsule, got r={r} g={g} b={b}");
    }

    #[test]
    fn idle_tray_background_is_transparent() {
        let icon = build_icon_sized("idle", true, 22);
        let idx = (2 * icon.width as usize + 2) * 4;
        assert_eq!(icon.data[idx], 0, "corner alpha should be 0");
    }

    #[test]
    fn idle_mic_is_light_on_dark_ui() {
        let icon = build_icon_sized("idle", true, 22);
        let (r, g, b) = capsule_center_argb(&icon);
        assert!(
            r > 200 && g > 200 && b > 200,
            "light mic expected, got r={r} g={g} b={b}"
        );
    }

    #[test]
    fn idle_mic_is_dark_on_light_ui() {
        let icon = build_icon_sized("idle", false, 22);
        let (r, g, b) = capsule_center_argb(&icon);
        assert!(
            r < 40 && g < 50 && b < 70,
            "dark mic expected, got r={r} g={g} b={b}"
        );
    }
}
