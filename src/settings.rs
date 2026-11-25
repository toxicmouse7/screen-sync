use anyhow::ensure;
use eframe::egui::{self, RichText};
use serde::{Deserialize, Serialize};

use crate::capture::CaptureMonitor;
use crate::dashboard::{self, Arrow, MINT, MUTED};
use crate::i18n::{Key, Language};

#[derive(Clone, Copy, Deserialize, Serialize)]
pub enum ScreenSyncSide {
    Top,
    Right,
    Bottom,
    Left,
}

impl ScreenSyncSide {
    fn label(self, language: Language) -> &'static str {
        match self {
            Self::Top => language.text(Key::Top),
            Self::Right => language.text(Key::Right),
            Self::Bottom => language.text(Key::Bottom),
            Self::Left => language.text(Key::Left),
        }
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct ScreenSyncSegment {
    pub side: ScreenSyncSide,
    pub reversed: bool,
    pub led_count: u32,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct ScreenSyncLayout {
    pub segments: [ScreenSyncSegment; 4],
}

impl ScreenSyncLayout {
    pub fn validate(&self) -> anyhow::Result<()> {
        ensure!(
            self.segments
                .iter()
                .all(|segment| (1..=450).contains(&segment.led_count)),
            Key::LedCountRequired
        );
        ensure!(
            self.segments
                .iter()
                .map(|segment| segment.led_count)
                .sum::<u32>()
                <= 450,
            Key::LedLimit
        );
        Ok(())
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct ScreenSyncCaptureSettings {
    pub monitor: String,
    pub fps: u32,
    pub band_percent: u32,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct DeviceSettings {
    pub capture: ScreenSyncCaptureSettings,
    pub layout: ScreenSyncLayout,
}

impl Default for DeviceSettings {
    fn default() -> Self {
        use ScreenSyncSide::*;

        Self {
            capture: ScreenSyncCaptureSettings {
                monitor: String::new(),
                fps: 30,
                band_percent: 5,
            },
            layout: ScreenSyncLayout {
                segments: [Top, Right, Bottom, Left].map(|side| ScreenSyncSegment {
                    side,
                    reversed: false,
                    led_count: 0,
                }),
            },
        }
    }
}

impl DeviceSettings {
    pub fn validate(&self) -> anyhow::Result<()> {
        ensure!(!self.capture.monitor.is_empty(), Key::SelectMonitor);
        ensure!((1..=60).contains(&self.capture.fps), Key::FrameRateRange);
        ensure!(
            (1..=50).contains(&self.capture.band_percent),
            Key::BandDepthRange
        );
        self.layout.validate()
    }

    pub fn show(&mut self, ui: &mut egui::Ui, monitors: &[CaptureMonitor], language: Language) {
        if ui.available_width() >= 960.0 {
            ui.columns(2, |columns| {
                self.show_capture(&mut columns[0], monitors, language);
                self.show_layout(&mut columns[1], language);
            });
        } else {
            self.show_capture(ui, monitors, language);
            self.show_layout(ui, language);
        }
    }

    fn show_capture(&mut self, ui: &mut egui::Ui, monitors: &[CaptureMonitor], language: Language) {
        dashboard::surface().show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.heading(language.text(Key::Capture));
            self.show_capture_fields(ui, monitors, language);
            ui.add_space(6.0);
            ui.colored_label(MUTED, language.text(Key::CaptureArea));
            self.show_band_preview(ui);
        });
    }

    fn show_capture_fields(
        &mut self,
        ui: &mut egui::Ui,
        monitors: &[CaptureMonitor],
        language: Language,
    ) {
        let selected = monitors
            .iter()
            .find(|monitor| monitor.name == self.capture.monitor);
        ui.label(RichText::new(language.text(Key::Monitor)).color(MUTED));
        egui::ComboBox::from_id_salt("capture_monitor")
            .width(ui.available_width().min(360.0))
            .selected_text(selected.map_or_else(
                || language.text(Key::SelectMonitor).to_owned(),
                |monitor| monitor_label(monitor, language),
            ))
            .show_ui(ui, |ui| {
                for monitor in monitors {
                    ui.selectable_value(
                        &mut self.capture.monitor,
                        monitor.name.clone(),
                        monitor_label(monitor, language),
                    );
                }
            });
        ui.add(
            egui::Slider::new(&mut self.capture.band_percent, 1..=50)
                .text(language.text(Key::BandDepth)),
        );
        ui.add(
            egui::Slider::new(&mut self.capture.fps, 1..=60).text(language.text(Key::FrameRate)),
        );
    }

    fn show_band_preview(&self, ui: &mut egui::Ui) {
        let width = ui.available_width().min(320.0);
        let (rect, _) =
            ui.allocate_exact_size(egui::vec2(width, width * 9.0 / 16.0), egui::Sense::hover());
        let painter = ui.painter();
        let depth = self.capture.band_percent as f32 / 100.0;
        let inner = rect.shrink2(rect.size() * depth);
        painter.rect_filled(rect, 8, egui::Color32::from_rgb(43, 98, 86));
        painter.rect_filled(inner, 4, egui::Color32::from_rgb(12, 19, 29));
        painter.rect_stroke(
            rect,
            8,
            egui::Stroke::new(1.0, MINT),
            egui::StrokeKind::Inside,
        );
    }

    fn show_layout(&mut self, ui: &mut egui::Ui, language: Language) {
        dashboard::surface().show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.heading(language.text(Key::StripLayout));
            ui.label(RichText::new(language.text(Key::WiringOrder)).color(MUTED));
            self.show_segments(ui, language);
            let total: u32 = self
                .layout
                .segments
                .iter()
                .map(|segment| segment.led_count)
                .sum();
            ui.label(
                RichText::new(language.format(Key::LedTotal, total))
                    .strong()
                    .color(MINT),
            );
        });
    }

    fn show_segments(&mut self, ui: &mut egui::Ui, language: Language) {
        let mut move_segment = None;
        for (index, segment) in self.layout.segments.iter_mut().enumerate() {
            ui.push_id(index, |ui| {
                ui.separator();
                ui.horizontal(|ui| {
                    ui.add_sized(
                        [82.0, 32.0],
                        egui::Label::new(RichText::new(segment.side.label(language)).strong()),
                    );
                    ui.add(
                        egui::DragValue::new(&mut segment.led_count)
                            .range(0..=450)
                            .suffix(" LED"),
                    );
                    ui.checkbox(&mut segment.reversed, language.text(Key::Reverse))
                        .on_hover_text(language.text(Key::ReverseDirection));
                    if dashboard::arrow_button(ui, Arrow::Up, index > 0, language).clicked() {
                        move_segment = Some((index, index - 1));
                    }
                    if dashboard::arrow_button(ui, Arrow::Down, index < 3, language).clicked() {
                        move_segment = Some((index, index + 1));
                    }
                });
            });
        }
        if let Some((from, to)) = move_segment {
            self.layout.segments.swap(from, to);
        }
    }
}

fn monitor_label(monitor: &CaptureMonitor, language: Language) -> String {
    let main = if monitor.primary {
        format!(" · {}", language.text(Key::PrimaryMonitor))
    } else {
        String::new()
    };
    format!(
        "{} · {} × {}{main}",
        monitor.name, monitor.width, monitor.height
    )
}
