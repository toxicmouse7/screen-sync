#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;

struct ScreenSync;

impl eframe::App for ScreenSync {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |_| {});
    }
}

fn main() -> eframe::Result {
    eframe::run_native(
        "Screen Sync",
        eframe::NativeOptions::default(),
        Box::new(|_| Ok(Box::new(ScreenSync))),
    )
}
