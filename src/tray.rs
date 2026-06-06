// ── System tray icon ─────────────────────────────────────────
// StatusNotifierItem via D-Bus (ksni crate).
// Left click: toggle recording (start/stop).
// Right click: menu with Cancel, Settings, Quit.
// Mic-only icon (no background), theme-aware, tinted by state.

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
const GRAY: (u8, u8, u8) = (150, 150, 150);
const GREEN: (u8, u8, u8) = (60, 170, 60);
const MIC_LIGHT: (u8, u8, u8) = (238, 241, 248); // #eef1f8 on dark panels
const MIC_DARK: (u8, u8, u8) = (26, 34, 56); // #1a2238 on light panels

const TRAY_ICON_22: &[u8] = include_bytes!("../gui/icons/tray-22.png");
const TRAY_ICON_44: &[u8] = include_bytes!("../gui/icons/tray-44.png");

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

fn base_icon(size: i32) -> &'static (i32, Vec<u8>) {
    static ICON_22: OnceLock<(i32, Vec<u8>)> = OnceLock::new();
    static ICON_44: OnceLock<(i32, Vec<u8>)> = OnceLock::new();
    if size >= 44 {
        ICON_44.get_or_init(|| decode_png_rgba(TRAY_ICON_44))
    } else {
        ICON_22.get_or_init(|| decode_png_rgba(TRAY_ICON_22))
    }
}

fn draw_circle(data: &mut [u8], size: i32, r: u8, g: u8, b: u8, alpha: u8) {
    let cx = (size / 2) as f32;
    let cy = (size / 2) as f32;
    let radius = size as f32 * 0.42;
    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            let idx = (y * size + x) as usize * 4;
            if dist <= radius + 0.5 {
                data[idx] = r;
                data[idx + 1] = g;
                data[idx + 2] = b;
                data[idx + 3] = alpha;
            }
        }
    }
}

fn blend_channel(base: u8, tint: u8, amount: f32) -> u8 {
    ((f32::from(base) * (1.0 - amount)) + (f32::from(tint) * amount)).round() as u8
}

fn apply_theme_color(rgba: &mut [u8], dark_ui: bool) {
    let (r, g, b) = mic_color(dark_ui);
    for px in rgba.chunks_exact_mut(4) {
        if px[3] == 0 {
            continue;
        }
        px[0] = r;
        px[1] = g;
        px[2] = b;
    }
}

fn apply_state_tint(rgba: &mut [u8], state: &str) {
    let (tr, tg, tb, amount) = match state {
        "transcribing" => (GRAY.0, GRAY.1, GRAY.2, 0.55),
        "inserting" => (GREEN.0, GREEN.1, GREEN.2, 0.45),
        "error" => (RED.0, RED.1, RED.2, 0.5),
        _ => return,
    };

    for px in rgba.chunks_exact_mut(4) {
        if px[3] == 0 {
            continue;
        }
        if state == "transcribing" {
            let lum =
                (0.299 * f32::from(px[0]) + 0.587 * f32::from(px[1]) + 0.114 * f32::from(px[2]))
                    .round() as u8;
            px[0] = lum;
            px[1] = lum;
            px[2] = lum;
        } else {
            px[0] = blend_channel(px[0], tr, amount);
            px[1] = blend_channel(px[1], tg, amount);
            px[2] = blend_channel(px[2], tb, amount);
        }
    }
}

pub fn build_icon(state: &str) -> Icon {
    build_icon_sized(state, ui_prefers_dark(), 22)
}

fn build_icon_sized(state: &str, dark_ui: bool, size: i32) -> Icon {
    let mut pixels = vec![0u8; (size * size * 4) as usize];

    if state == "recording" {
        draw_circle(&mut pixels, size, RED.0, RED.1, RED.2, 255);
    } else {
        let (src_w, src_rgba) = base_icon(size);
        assert_eq!(*src_w, size, "tray icon size mismatch");
        pixels.copy_from_slice(src_rgba);
        apply_theme_color(&mut pixels, dark_ui);
        apply_state_tint(&mut pixels, state);
    }

    rgba_to_argb(&mut pixels);

    Icon {
        width: size,
        height: size,
        data: pixels,
    }
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

    #[test]
    fn recording_icon_uses_red_channel_in_argb() {
        let icon = build_icon_sized("recording", true, 22);
        let idx = (11 * icon.width as usize + 11) * 4;
        // ARGB byte order: A, R, G, B
        let r = icon.data[idx + 1];
        let b = icon.data[idx + 3];
        assert!(r > 200, "red channel expected, got r={r} b={b}");
        assert!(b < 100, "blue should be low, got b={b}");
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
        let idx = (11 * icon.width as usize + 11) * 4;
        let r = icon.data[idx + 1];
        let g = icon.data[idx + 2];
        let b = icon.data[idx + 3];
        assert!(
            r > 200 && g > 200 && b > 200,
            "light mic expected, got r={r} g={g} b={b}"
        );
    }

    #[test]
    fn idle_mic_is_dark_on_light_ui() {
        let icon = build_icon_sized("idle", false, 22);
        let idx = (11 * icon.width as usize + 11) * 4;
        let r = icon.data[idx + 1];
        let g = icon.data[idx + 2];
        let b = icon.data[idx + 3];
        assert!(
            r < 40 && g < 50 && b < 70,
            "dark mic expected, got r={r} g={g} b={b}"
        );
    }

    #[test]
    fn transcribing_icon_is_grayscale() {
        let idle = build_icon_sized("idle", true, 22);
        let gray = build_icon_sized("transcribing", true, 22);
        let idx = (11 * idle.width as usize + 11) * 4;
        let ir = idle.data[idx + 1];
        let ig = idle.data[idx + 2];
        let gr = gray.data[idx + 1];
        let gg = gray.data[idx + 2];
        assert!(
            (gr as i16 - gg as i16).abs() < 5,
            "transcribing should be gray, got r={gr} g={gg}"
        );
        assert!(
            ir != gr || ig != gg,
            "transcribing tint should differ from idle at center"
        );
    }
}
