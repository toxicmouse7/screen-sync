use std::{net::Ipv4Addr, time::Duration};

use eframe::egui;

use crate::dashboard::{self, MUTED};
use crate::i18n::{Key, Language};
use crate::mdns;

#[derive(Default)]
pub struct AddDeviceDialog {
    input: String,
    search: Option<Result<mdns::Search, mdns_sd::Error>>,
}

pub enum AddDeviceResult {
    Pending,
    Cancelled,
    Submitted(Ipv4Addr),
}

impl AddDeviceDialog {
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        language: Language,
        contains: impl Fn(Ipv4Addr) -> bool,
    ) -> AddDeviceResult {
        let modal = egui::Modal::new(egui::Id::new("add_device"))
            .frame(dashboard::surface())
            .show(ctx, |ui| {
                ui.set_width(320.0);
                ui.heading(language.text(Key::AddDevice));
                ui.colored_label(MUTED, language.text(Key::Address));
                ui.add(
                    egui::TextEdit::singleline(&mut self.input)
                        .desired_width(f32::INFINITY)
                        .hint_text("192.168.1.50"),
                );

                let ip = self.input.trim().parse::<Ipv4Addr>().ok();
                let new_ip = ip.filter(|&ip| !contains(ip));
                let manual_ip = ui
                    .horizontal(|ui| {
                        let add = ui
                            .add_enabled(
                                new_ip.is_some(),
                                dashboard::primary(language.text(Key::Add)),
                            )
                            .on_disabled_hover_text(if ip.is_some() {
                                language.text(Key::AlreadyAdded)
                            } else {
                                language.text(Key::InvalidAddress)
                            });
                        if ui.button(language.text(Key::Cancel)).clicked() {
                            ui.close();
                        }
                        if add.clicked() { new_ip } else { None }
                    })
                    .inner;

                if manual_ip.is_some() {
                    return manual_ip;
                }

                ui.separator();
                self.show_search(ui, &contains, language)
            });

        if let Some(ip) = modal.inner {
            AddDeviceResult::Submitted(ip)
        } else if modal.should_close() {
            AddDeviceResult::Cancelled
        } else {
            AddDeviceResult::Pending
        }
    }

    fn show_search(
        &mut self,
        ui: &mut egui::Ui,
        contains: &impl Fn(Ipv4Addr) -> bool,
        language: Language,
    ) -> Option<Ipv4Addr> {
        let searching = self
            .search
            .as_mut()
            .is_some_and(|result| result.as_mut().is_ok_and(|search| search.poll()));
        if searching {
            ui.ctx().request_repaint_after(Duration::from_millis(100));
        }

        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    !searching,
                    egui::Button::new(language.text(Key::AutomaticSearch)),
                )
                .clicked()
            {
                self.search = Some(mdns::Search::start());
                ui.ctx().request_repaint();
            }
            if searching {
                ui.spinner();
            }
        });

        match &self.search {
            Some(Ok(search)) if !search.addresses.is_empty() => {
                egui::ScrollArea::vertical()
                    .max_height(132.0)
                    .show(ui, |ui| {
                        for &ip in &search.addresses {
                            let added = contains(ip);
                            let label = if added {
                                language.text(Key::Added)
                            } else {
                                language.text(Key::Add)
                            };
                            let clicked = ui
                                .horizontal(|ui| {
                                    ui.monospace(ip.to_string());
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            ui.add_enabled(!added, egui::Button::new(label))
                                                .clicked()
                                        },
                                    )
                                    .inner
                                })
                                .inner;
                            if clicked {
                                return Some(ip);
                            }
                        }
                        None
                    })
                    .inner
            }
            Some(Ok(_)) if !searching => {
                ui.colored_label(MUTED, language.text(Key::NoAddresses));
                None
            }
            Some(Err(_)) => {
                ui.colored_label(dashboard::WARNING, language.text(Key::SearchFailed));
                None
            }
            _ => None,
        }
    }
}
