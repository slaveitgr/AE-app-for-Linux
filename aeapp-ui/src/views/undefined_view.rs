use egui::Ui;

use crate::AeAppEguiApp;

pub fn undefined_view(app: &mut AeAppEguiApp, ui: &mut Ui) {
    ui.with_layout(
        egui::Layout::centered_and_justified(egui::Direction::RightToLeft),
        |ui| {
            ui.heading(&app.locale.localization.errors.view_not_impl);
        },
    );
}

pub fn coming_soon_view(app: &mut AeAppEguiApp, ui: &mut Ui) {
    ui.with_layout(
        egui::Layout::centered_and_justified(egui::Direction::RightToLeft),
        |ui| {
            ui.heading(&app.locale.localization.errors.view_coming_soon);
        },
    );
}
