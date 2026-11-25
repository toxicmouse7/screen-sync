#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod add_device_dialog;
mod capture;
mod dashboard;
mod i18n;
mod mdns;
#[cfg(windows)]
mod reachability;
mod settings;

use std::{collections::BTreeMap, net::Ipv4Addr, time::Duration};

use eframe::egui::{self, RichText};

use add_device_dialog::{AddDeviceDialog, AddDeviceResult};
use capture::{Capture, CaptureMonitor};
use dashboard::{Arrow, MINT, MUTED, WARNING};
use i18n::{Key, Language};
use settings::DeviceSettings;

const DEVICES_KEY: &str = "devices";
const LANGUAGE_KEY: &str = "language";

struct ScreenSync {
    language: Language,
    preferences_open: bool,
    logo: egui::TextureHandle,
    devices: BTreeMap<Ipv4Addr, DeviceSettings>,
    #[cfg(windows)]
    reachability: reachability::Reachability,
    add_device: Option<AddDeviceDialog>,
    remove_device: Option<Ipv4Addr>,
    selected: Option<Ipv4Addr>,
    monitors: Vec<CaptureMonitor>,
    monitor_error: Option<anyhow::Error>,
    captures: BTreeMap<Ipv4Addr, Capture>,
    capture_errors: BTreeMap<Ipv4Addr, anyhow::Error>,
}

impl ScreenSync {
    fn new(ctx: &eframe::CreationContext<'_>) -> Self {
        dashboard::set_style(&ctx.egui_ctx);
        let logo = eframe::icon_data::from_png_bytes(include_bytes!("../assets/monitor-light.png"))
            .expect("The bundled logo must be a valid PNG");
        let logo = ctx.egui_ctx.load_texture(
            "screen-sync-logo",
            egui::ColorImage::from_rgba_unmultiplied(
                [logo.width as usize, logo.height as usize],
                &logo.rgba,
            ),
            egui::TextureOptions::LINEAR,
        );
        Self {
            language: ctx
                .storage
                .and_then(|storage| eframe::get_value(storage, LANGUAGE_KEY))
                .unwrap_or_default(),
            preferences_open: false,
            logo,
            devices: ctx
                .storage
                .and_then(|storage| eframe::get_value(storage, DEVICES_KEY))
                .unwrap_or_default(),
            #[cfg(windows)]
            reachability: reachability::Reachability::default(),
            add_device: None,
            remove_device: None,
            selected: None,
            monitors: Vec::new(),
            monitor_error: None,
            captures: BTreeMap::new(),
            capture_errors: BTreeMap::new(),
        }
    }

    fn show_workspace(&mut self, ui: &mut egui::Ui) {
        let language = self.language;
        let t = |key| language.text(key);
        ui.horizontal(|ui| {
            ui.add(egui::Image::new(&self.logo).max_width(34.0));
            ui.label(RichText::new("screen-sync").size(20.0).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(t(Key::Settings)).clicked() {
                    self.preferences_open = true;
                }
            });
        });
        ui.add_space(12.0);
        ui.separator();
        ui.add_space(12.0);

        ui.horizontal(|ui| {
            ui.label(RichText::new(t(Key::ScreenLighting)).size(34.0).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add(dashboard::primary(&format!("+ {}", t(Key::AddDevice))))
                    .clicked()
                {
                    self.add_device = Some(AddDeviceDialog::default());
                }
            });
        });
        ui.add_space(14.0);

        let width = ui.available_width();
        let (device, session) = if width >= 790.0 {
            let session_width = (width * 0.35).clamp(290.0, 360.0);
            let device_width = width - session_width - ui.spacing().item_spacing.x;
            ui.horizontal_top(|ui| {
                let devices = ui
                    .vertical(|ui| {
                        ui.set_width(device_width);
                        self.show_device_list(ui)
                    })
                    .inner;
                let session = ui
                    .vertical(|ui| {
                        ui.set_width(session_width);
                        self.show_session(ui)
                    })
                    .inner;
                (devices, session)
            })
            .inner
        } else {
            let devices = self.show_device_list(ui);
            ui.add_space(8.0);
            (devices, self.show_session(ui))
        };

        if let Some(ip) = device.or(session) {
            self.selected = Some(ip);
            self.refresh_monitors(ip);
            ui.ctx().request_repaint();
        }
    }

    fn show_device_list(&self, ui: &mut egui::Ui) -> Option<Ipv4Addr> {
        let language = self.language;
        let t = |key| language.text(key);
        ui.horizontal(|ui| {
            ui.heading(t(Key::Devices));
            dashboard::tag(ui, &self.devices.len().to_string(), MUTED);
        });

        if self.devices.is_empty() {
            dashboard::surface().show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.label(RichText::new(t(Key::NoDevices)).strong());
            });
            return None;
        }

        let mut selected = None;
        for (&ip, settings) in &self.devices {
            let status = self.captures.get(&ip).map(|capture| {
                if capture.is_stopping() {
                    (t(Key::Stopping), WARNING)
                } else {
                    (t(Key::Sync), MINT)
                }
            });
            let strip = if settings.layout.validate().is_ok() {
                let count: u32 = settings
                    .layout
                    .segments
                    .iter()
                    .map(|segment| segment.led_count)
                    .sum();
                format!("{count} LED")
            } else {
                t(Key::StripNotConfigured).to_owned()
            };
            let details = format!("{} FPS · {strip}", settings.capture.fps);
            #[cfg(windows)]
            let availability = Some(match self.reachability.status(ip) {
                reachability::Status::Checking => (t(Key::Checking), MUTED),
                reachability::Status::Online => (t(Key::Online), MINT),
                reachability::Status::NoReply => (t(Key::NoReply), WARNING),
            });
            #[cfg(not(windows))]
            let availability = None;
            if dashboard::device_card(ui, ip, &details, availability, status, language).clicked() {
                selected = Some(ip);
            }
        }
        selected
    }

    fn show_session(&self, ui: &mut egui::Ui) -> Option<Ipv4Addr> {
        let t = |key| self.language.text(key);
        ui.heading(t(Key::Sync));
        dashboard::surface()
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                dashboard::signal_preview(
                    ui,
                    self.captures.values().any(|capture| !capture.is_stopping()),
                );
                if self.captures.is_empty() && self.capture_errors.is_empty() {
                    dashboard::tag(ui, t(Key::SyncOff), MUTED);
                    ui.colored_label(
                        MUTED,
                        if self.devices.is_empty() {
                            t(Key::AddDeviceHelp)
                        } else {
                            t(Key::SetupHelp)
                        },
                    );
                }
                for (&ip, capture) in &self.captures {
                    ui.separator();
                    if ui
                        .button(RichText::new(ip.to_string()).monospace())
                        .clicked()
                    {
                        return Some(ip);
                    }
                    ui.colored_label(MUTED, &capture.monitor);
                    if capture.is_stopping() {
                        dashboard::tag(ui, t(Key::Stopping), WARNING);
                    } else {
                        dashboard::tag(ui, t(Key::SyncRunning), MINT);
                    }
                }
                for (&ip, error) in &self.capture_errors {
                    ui.separator();
                    if ui
                        .button(RichText::new(ip.to_string()).monospace())
                        .clicked()
                    {
                        return Some(ip);
                    }
                    ui.colored_label(WARNING, self.language.error(error));
                }
                None
            })
            .inner
    }

    fn refresh_monitors(&mut self, ip: Ipv4Addr) {
        match capture::monitors() {
            Ok(monitors) => {
                self.monitor_error = None;
                let settings = &mut self.devices.get_mut(&ip).unwrap().capture;
                if settings.monitor.is_empty() {
                    if let Some(monitor) = monitors
                        .iter()
                        .filter(|monitor| {
                            !self
                                .captures
                                .values()
                                .any(|capture| capture.monitor == monitor.name)
                        })
                        .max_by_key(|monitor| monitor.primary)
                    {
                        settings.monitor.clone_from(&monitor.name);
                    }
                }
                self.monitors = monitors;
            }
            Err(error) => {
                self.monitors.clear();
                self.monitor_error = Some(error);
            }
        }
    }

    fn show_settings(&mut self, ui: &mut egui::Ui, ip: Ipv4Addr) {
        let language = self.language;
        let t = |key| language.text(key);
        let running = self.captures.contains_key(&ip);
        if dashboard::arrow_button(ui, Arrow::Left, true, language).clicked() {
            self.selected = None;
            ui.ctx().request_repaint();
        }
        ui.horizontal(|ui| {
            dashboard::monitor_icon(ui, 48.0);
            ui.vertical(|ui| {
                ui.label(RichText::new(t(Key::DeviceSettings)).size(28.0).strong());
                ui.label(RichText::new(ip.to_string()).monospace().color(MUTED));
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add_enabled(
                        !running,
                        egui::Button::new(RichText::new(t(Key::RemoveDevice)).color(WARNING)),
                    )
                    .on_disabled_hover_text(t(Key::StopFirst))
                    .clicked()
                {
                    self.remove_device = Some(ip);
                }
            });
        });
        ui.add_space(10.0);
        dashboard::surface().show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            self.show_capture_controls(ui, ip);
        });
        ui.add_space(8.0);

        let settings = self.devices.get_mut(&ip).unwrap();
        ui.add_enabled_ui(!running, |ui| settings.show(ui, &self.monitors, language));
    }

    fn show_remove_device_dialog(&mut self, ctx: &egui::Context, ip: Ipv4Addr) {
        let language = self.language;
        let t = |key| language.text(key);
        let modal = egui::Modal::new(egui::Id::new("remove_device"))
            .frame(dashboard::surface())
            .show(ctx, |ui| {
                ui.heading(t(Key::ConfirmRemoval));
                ui.label(language.format(Key::RemovalDetails, ip));
                ui.horizontal(|ui| {
                    if ui.button(t(Key::Cancel)).clicked() {
                        ui.close();
                    }
                    ui.button(RichText::new(t(Key::Remove)).color(WARNING))
                        .clicked()
                })
                .inner
            });

        if modal.inner {
            self.devices.remove(&ip);
            self.capture_errors.remove(&ip);
            #[cfg(windows)]
            self.reachability.forget(ip);
            self.selected = None;
            self.remove_device = None;
            ctx.request_repaint();
        } else if modal.should_close() {
            self.remove_device = None;
        }
    }

    fn show_capture_controls(&mut self, ui: &mut egui::Ui, ip: Ipv4Addr) {
        let language = self.language;
        let t = |key| language.text(key);
        ui.heading(t(Key::Sync));

        if let Some(error) = &self.monitor_error {
            ui.colored_label(WARNING, language.error(error));
        }
        if let Some(error) = self.capture_errors.get(&ip) {
            ui.colored_label(WARNING, language.error(error));
        }

        if let Some(capture) = self.captures.get_mut(&ip) {
            let stopping = capture.is_stopping();
            let label = if stopping {
                t(Key::Stopping)
            } else {
                t(Key::Stop)
            };
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(!stopping, egui::Button::new(label))
                    .clicked()
                {
                    capture.request_stop();
                }
                if !stopping {
                    dashboard::tag(ui, t(Key::SyncRunning), MINT);
                }
            });
            return;
        }

        let settings = &self.devices[&ip];
        let reason = if !self
            .monitors
            .iter()
            .any(|monitor| monitor.name == settings.capture.monitor)
        {
            Some(t(Key::AvailableMonitor).to_owned())
        } else if let Some((&owner, _)) = self
            .captures
            .iter()
            .find(|(_, capture)| capture.monitor == settings.capture.monitor)
        {
            Some(language.format(Key::MonitorBusy, owner))
        } else {
            settings
                .validate()
                .err()
                .map(|error| language.error(&error))
        };
        let start = ui
            .horizontal_wrapped(|ui| {
                let start = ui.add_enabled(reason.is_none(), dashboard::primary(t(Key::Start)));
                if let Some(reason) = reason {
                    ui.colored_label(MUTED, reason);
                }
                start.clicked()
            })
            .inner;
        if start {
            self.capture_errors.remove(&ip);
            match Capture::start(ip, settings.clone()) {
                Ok(capture) => {
                    self.captures.insert(ip, capture);
                }
                Err(error) => {
                    self.capture_errors.insert(ip, error);
                }
            }
            ui.ctx().request_repaint();
        }
    }

    fn show_preferences(&mut self, ctx: &egui::Context) {
        let before = self.language;
        let modal = egui::Modal::new(egui::Id::new("preferences"))
            .frame(dashboard::surface())
            .show(ctx, |ui| {
                ui.set_width(300.0);
                ui.heading(self.language.text(Key::Settings));
                ui.colored_label(MUTED, self.language.text(Key::Language));
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.language, Language::Russian, "Русский");
                    ui.selectable_value(&mut self.language, Language::English, "English");
                });
                if ui
                    .add(dashboard::primary(self.language.text(Key::Done)))
                    .clicked()
                {
                    ui.close();
                }
            });
        if modal.should_close() {
            self.preferences_open = false;
        }
        if before != self.language {
            ctx.request_repaint();
        }
    }
}

impl eframe::App for ScreenSync {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        #[cfg(windows)]
        self.reachability
            .poll(ui.ctx(), self.devices.keys().copied(), |ip| {
                self.devices.contains_key(&ip)
            });

        self.captures.retain(|&ip, capture| match capture.poll() {
            None => true,
            Some(result) => {
                if let Err(error) = result {
                    self.capture_errors.insert(ip, error);
                }
                false
            }
        });
        if !self.captures.is_empty() {
            ui.ctx().request_repaint_after(Duration::from_millis(150));
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::central_panel(ui.style()).inner_margin(22))
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let width = ui.available_width();
                    let content_width = width.min(1080.0);
                    ui.horizontal_top(|ui| {
                        ui.add_space((width - content_width) / 2.0);
                        ui.vertical(|ui| {
                            ui.set_width(content_width);
                            if let Some(ip) = self.selected {
                                self.show_settings(ui, ip);
                            } else {
                                self.show_workspace(ui);
                            }
                        });
                    });
                });
            });

        if let Some(dialog) = &mut self.add_device {
            match dialog.show(ui.ctx(), self.language, |ip| self.devices.contains_key(&ip)) {
                AddDeviceResult::Pending => {}
                AddDeviceResult::Cancelled => self.add_device = None,
                AddDeviceResult::Submitted(ip) => {
                    self.devices.insert(ip, DeviceSettings::default());
                    self.add_device = None;
                    ui.ctx().request_repaint();
                }
            }
        }
        if let Some(ip) = self.remove_device {
            self.show_remove_device_dialog(ui.ctx(), ip);
        }
        if self.preferences_open {
            self.show_preferences(ui.ctx());
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, DEVICES_KEY, &self.devices);
        eframe::set_value(storage, LANGUAGE_KEY, &self.language);
    }
}

fn main() -> eframe::Result {
    let icon = eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon.png"))
        .expect("The bundled icon must be a valid PNG");
    eframe::run_native(
        "Screen Sync",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([1280.0, 720.0])
                .with_resizable(false)
                .with_maximize_button(false)
                .with_icon(icon),
            ..Default::default()
        },
        Box::new(|ctx| Ok(Box::new(ScreenSync::new(ctx)))),
    )
}
