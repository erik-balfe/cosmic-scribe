// ── System tray icon ─────────────────────────────────────────
// StatusNotifierItem via D-Bus (ksni crate).
// Left click: toggle recording (start/stop).
// Right click: menu with Cancel, Settings, Quit.
// Mic-shaped icon, colored by state.

use crate::keyring;
use crate::state::Event;
use ksni::menu::StandardItem;
use ksni::{Icon, MenuItem, Status, ToolTip, Tray};
use tokio::sync::mpsc;

/// ksni / StatusNotifierItem expects ARGB32 (see ksni::Icon docs), not RGBA.
fn rgba_to_argb(rgba: &mut [u8]) {
    for px in rgba.chunks_exact_mut(4) {
        px.rotate_right(1);
    }
}

const RED: (u8, u8, u8) = (220, 40, 40);
const GRAY: (u8, u8, u8) = (150, 150, 150);

fn state_color(state: &str) -> (u8, u8, u8) {
    match state {
        "recording" | "error" => RED,
        "transcribing" => GRAY,
        "inserting" => (60, 170, 60),
        _ => (240, 240, 240), // idle — white mic
    }
}

fn draw_mic(data: &mut [u8], size: i32, r: u8, g: u8, b: u8) {
    let sz = size as f32;
    // Mic capsule: centered horizontally, upper 60% of icon
    let cap_cx = sz / 2.0;
    let cap_top = sz * 0.08;
    let cap_bot = sz * 0.58;
    let cap_rx = sz * 0.18;
    let cap_ry = (cap_bot - cap_top) / 2.0;
    let cap_cy = (cap_top + cap_bot) / 2.0;

    // Stand: vertical bar below capsule
    let stand_w = sz * 0.06;
    let stand_top = cap_bot;
    let stand_bot = sz * 0.82;

    // Base: horizontal bar
    let base_h = sz * 0.06;
    let base_w = sz * 0.30;
    let base_y = sz * 0.88;

    for y in 0..size {
        for x in 0..size {
            let xf = x as f32 + 0.5;
            let yf = y as f32 + 0.5;
            let idx = (y * size + x) as usize * 4;

            let cap = {
                let dx = (xf - cap_cx) / cap_rx;
                let dy = (yf - cap_cy) / cap_ry;
                dx * dx + dy * dy <= 1.0
            };

            let stand = xf >= cap_cx - stand_w
                && xf <= cap_cx + stand_w
                && yf >= stand_top
                && yf <= stand_bot;

            let base = xf >= cap_cx - base_w
                && xf <= cap_cx + base_w
                && yf >= base_y - base_h
                && yf <= base_y + base_h;

            if cap || stand || base {
                data[idx] = r;
                data[idx + 1] = g;
                data[idx + 2] = b;
                data[idx + 3] = 255;
            }
        }
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

pub fn build_icon(state: &str) -> Icon {
    let size: i32 = 22;
    let mut pixels = vec![0u8; (size * size * 4) as usize];

    match state {
        "recording" => draw_circle(&mut pixels, size, RED.0, RED.1, RED.2, 255),
        _ => {
            let (r, g, b) = state_color(state);
            draw_mic(&mut pixels, size, r, g, b);
        }
    }
    rgba_to_argb(&mut pixels);

    Icon {
        width: size,
        height: size,
        data: pixels,
    }
}

pub struct VoiceTray {
    state: String,
    toggle_tx: mpsc::UnboundedSender<Event>,
}

impl VoiceTray {
    pub fn new(toggle_tx: mpsc::UnboundedSender<Event>) -> Self {
        Self {
            state: "idle".into(),
            toggle_tx,
        }
    }

    pub fn set_state(&mut self, state: &str) {
        self.state = state.to_string();
    }
}

impl Tray for VoiceTray {
    fn icon_pixmap(&self) -> Vec<Icon> {
        vec![build_icon(&self.state)]
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
                let exe = std::env::current_exe().unwrap_or_else(|_| "voice-input".into());
                let _ = std::process::Command::new(exe).arg("--settings").spawn();
            }),
            ..Default::default()
        }));
        items.push(MenuItem::Standard(StandardItem {
            label: "Settings".into(),
            enabled: true,
            icon_name: "preferences-system".into(),
            activate: Box::new(|_: &mut Self| {
                let exe = std::env::current_exe().unwrap_or_else(|_| "voice-input".into());
                let _ = std::process::Command::new(exe).arg("--settings").spawn();
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
        let icon = build_icon("recording");
        let idx = (11 * icon.width as usize + 11) * 4;
        // ARGB byte order: A, R, G, B
        let r = icon.data[idx + 1];
        let b = icon.data[idx + 3];
        assert!(r > 200, "red channel expected, got r={r} b={b}");
        assert!(b < 100, "blue should be low, got b={b}");
    }

    #[test]
    fn transcribing_icon_uses_gray_not_red() {
        let icon = build_icon("transcribing");
        let idx = (11 * icon.width as usize + 11) * 4;
        let r = icon.data[idx + 1];
        let g = icon.data[idx + 2];
        let b = icon.data[idx + 3];
        assert!(
            r < 200 && g < 200 && b < 200,
            "gray mic expected, got r={r} g={g} b={b}"
        );
        assert!(
            (r as i16 - g as i16).abs() < 30,
            "channels should be balanced for gray"
        );
    }
}
