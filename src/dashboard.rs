use std::net::Ipv4Addr;

use eframe::egui::{self, Color32, FontId, RichText, Stroke, TextStyle};

use crate::i18n::{Key, Language};

const TEXT: Color32 = Color32::from_rgb(231, 240, 250);
pub const MUTED: Color32 = Color32::from_rgb(156, 175, 194);
pub const MINT: Color32 = Color32::from_rgb(115, 238, 190);
pub const WARNING: Color32 = Color32::from_rgb(248, 191, 109);
const BACKGROUND: Color32 = Color32::from_rgb(14, 16, 26);
const SURFACE: Color32 = Color32::from_rgb(23, 28, 43);
const BORDER: Color32 = Color32::from_rgb(47, 55, 78);
const PRIMARY: Color32 = Color32::from_rgb(184, 169, 255);
const INK: Color32 = Color32::from_rgb(23, 15, 51);

pub fn set_style(ctx: &egui::Context) {
    ctx.set_theme(egui::Theme::Dark);
    ctx.style_mut_of(egui::Theme::Dark, |style| {
        style.spacing.item_spacing = egui::vec2(12.0, 10.0);
        style.spacing.button_padding = egui::vec2(12.0, 7.0);
        style.spacing.interact_size.y = 32.0;
        style
            .text_styles
            .insert(TextStyle::Body, FontId::proportional(15.0));
        style
            .text_styles
            .insert(TextStyle::Button, FontId::proportional(14.0));
        style
            .text_styles
            .insert(TextStyle::Heading, FontId::proportional(22.0));

        let visuals = &mut style.visuals;
        visuals.panel_fill = BACKGROUND;
        visuals.window_fill = SURFACE;
        visuals.window_corner_radius = 14.into();
        visuals.window_stroke = Stroke::new(1.0, BORDER);
        visuals.extreme_bg_color = BACKGROUND;
        visuals.selection.bg_fill = Color32::from_rgb(66, 54, 117);
        visuals.selection.stroke = Stroke::new(1.0, TEXT);
        visuals.widgets.noninteractive.fg_stroke.color = TEXT;
        visuals.widgets.noninteractive.bg_stroke.color = BORDER;

        for (widget, fill, line) in [
            (
                &mut visuals.widgets.inactive,
                Color32::from_rgb(34, 41, 60),
                BORDER,
            ),
            (
                &mut visuals.widgets.hovered,
                Color32::from_rgb(47, 54, 79),
                PRIMARY,
            ),
            (
                &mut visuals.widgets.active,
                Color32::from_rgb(59, 63, 96),
                PRIMARY,
            ),
        ] {
            widget.bg_fill = fill;
            widget.weak_bg_fill = fill;
            widget.bg_stroke = Stroke::new(1.0, line);
            widget.fg_stroke.color = TEXT;
            widget.corner_radius = 8.into();
        }
    });
}

pub fn surface() -> egui::Frame {
    egui::Frame::new()
        .fill(SURFACE)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(14)
        .inner_margin(20)
}

pub fn primary(text: &str) -> egui::Button<'_> {
    egui::Button::new(RichText::new(text).strong().color(INK))
        .fill(PRIMARY)
        .stroke(Stroke::new(1.0, PRIMARY))
        .corner_radius(8)
}

pub fn tag(ui: &mut egui::Ui, label: &str, color: Color32) {
    egui::Frame::new()
        .fill(Color32::from_rgb(38, 45, 64))
        .corner_radius(7)
        .inner_margin(egui::Margin::symmetric(8, 4))
        .show(ui, |ui| {
            ui.label(RichText::new(label).color(color).size(13.0))
        });
}

pub fn signal_preview(ui: &mut egui::Ui, active: bool) {
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 96.0), egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, 9, BACKGROUND);
    let center = rect.center();
    let monitor = egui::Rect::from_center_size(
        egui::pos2(rect.left() + rect.width() * 0.23, center.y - 3.0),
        egui::vec2(52.0, 35.0),
    );
    let line = Stroke::new(1.5, if active { MINT } else { MUTED });
    painter.rect_stroke(monitor, 6, line, egui::StrokeKind::Inside);
    let base = monitor.center_bottom() + egui::vec2(0.0, 8.0);
    painter.line_segment([monitor.center_bottom(), base], line);
    painter.line_segment(
        [base - egui::vec2(9.0, 0.0), base + egui::vec2(9.0, 0.0)],
        line,
    );
    paint_arrow(ui, center, egui::Vec2::X, line.color);

    let strip = egui::Rect::from_center_size(
        egui::pos2(rect.left() + rect.width() * 0.77, center.y),
        egui::vec2(76.0, 25.0),
    );
    painter.rect_filled(strip, 7, SURFACE);
    painter.rect_stroke(strip, 7, Stroke::new(1.0, BORDER), egui::StrokeKind::Inside);
    for (index, color) in [
        Color32::from_rgb(81, 221, 237),
        Color32::from_rgb(104, 153, 255),
        PRIMARY,
        Color32::from_rgb(235, 123, 224),
    ]
    .into_iter()
    .enumerate()
    {
        let led = egui::pos2(strip.left() + 13.0 + index as f32 * 17.0, center.y);
        painter.circle_filled(
            led,
            3.0,
            if active {
                color
            } else {
                color.gamma_multiply(0.55)
            },
        );
    }
}

pub fn device_card(
    ui: &mut egui::Ui,
    ip: Ipv4Addr,
    details: &str,
    availability: Option<(&str, Color32)>,
    status: Option<(&str, Color32)>,
    language: Language,
) -> egui::Response {
    let card = surface()
        .inner_margin(14)
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                monitor_icon(ui, 40.0);
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 4.0;
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(ip.to_string())
                                .monospace()
                                .size(20.0)
                                .strong(),
                        );
                        if let Some((label, color)) = availability {
                            ui.colored_label(color, label);
                        }
                    });
                    ui.colored_label(MUTED, details);
                });
                if let Some((label, color)) = status {
                    tag(ui, label, color);
                }
                ui.add_space(26.0);
            });
        })
        .response
        .interact(egui::Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);

    card.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Button,
            card.enabled(),
            language.format(Key::DeviceSettingsFor, ip),
        )
    });
    let color = if card.hovered() || card.has_focus() {
        ui.painter().rect_stroke(
            card.rect.shrink(0.5),
            14,
            Stroke::new(1.0, PRIMARY),
            egui::StrokeKind::Inside,
        );
        PRIMARY
    } else {
        MUTED
    };
    paint_arrow(
        ui,
        card.rect.right_center() - egui::vec2(21.0, 0.0),
        egui::Vec2::X,
        color,
    );
    card
}

pub enum Arrow {
    Left,
    Up,
    Down,
}

pub fn arrow_button(
    ui: &mut egui::Ui,
    arrow: Arrow,
    enabled: bool,
    language: Language,
) -> egui::Response {
    let icon = egui::Atom::custom(egui::Id::new("arrow"), egui::vec2(14.0, 14.0));
    let (button, direction, icon_label) = match arrow {
        Arrow::Left => (
            egui::Button::new((icon, language.text(Key::Devices))),
            egui::vec2(-1.0, 0.0),
            None,
        ),
        Arrow::Up => (
            egui::Button::new(icon),
            egui::vec2(0.0, -1.0),
            Some(language.text(Key::MoveUp)),
        ),
        Arrow::Down => (
            egui::Button::new(icon),
            egui::vec2(0.0, 1.0),
            Some(language.text(Key::MoveDown)),
        ),
    };

    let layout = ui
        .add_enabled_ui(enabled, |ui| {
            let layout = button.gap(7.0).atom_ui(ui);
            if let Some(rect) = layout.rect(egui::Id::new("arrow")) {
                paint_arrow(
                    ui,
                    rect.center(),
                    direction,
                    ui.style().interact(&layout.response).text_color(),
                );
            }
            layout
        })
        .inner;

    if let Some(label) = icon_label {
        layout.response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, layout.response.enabled(), label)
        });
        layout.response.on_hover_text(label)
    } else {
        layout.response
    }
}

fn paint_arrow(ui: &egui::Ui, center: egui::Pos2, direction: egui::Vec2, color: Color32) {
    let tip = center + direction * 5.0;
    let normal = egui::vec2(-direction.y, direction.x);
    let stroke = Stroke::new(1.6, color);
    ui.painter()
        .line_segment([center - direction * 5.0, tip], stroke);
    ui.painter()
        .line_segment([tip - direction * 4.0 + normal * 4.0, tip], stroke);
    ui.painter()
        .line_segment([tip, tip - direction * 4.0 - normal * 4.0], stroke);
}

pub fn monitor_icon(ui: &mut egui::Ui, size: f32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, 10, Color32::from_rgb(42, 37, 72));
    let center = rect.center();
    let screen = egui::Rect::from_center_size(
        center - egui::vec2(0.0, size * 0.06),
        egui::vec2(size * 0.49, size * 0.32),
    );
    let stroke = Stroke::new(1.6, PRIMARY);
    painter.rect_stroke(screen, 3, stroke, egui::StrokeKind::Inside);
    let from = egui::pos2(center.x, screen.bottom());
    let to = egui::pos2(center.x, screen.bottom() + size * 0.10);
    painter.line_segment([from, to], stroke);
    painter.line_segment(
        [
            to - egui::vec2(size * 0.12, 0.0),
            to + egui::vec2(size * 0.12, 0.0),
        ],
        stroke,
    );
}
