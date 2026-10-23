use crate::{widgets::enum_dropdown::enum_dropdown, MaximaEguiApp};
use egui::{vec2, Color32, RichText, Ui};
use std::path::PathBuf;

pub fn settings_view(app: &mut MaximaEguiApp, ui: &mut Ui) {
    ui.label(RichText::new("Settings").size(32.0).strong());
    ui.add_space(12.0);
    let id = egui::Id::new("ea_settings_tab");
    let mut tab = ui.ctx().data_mut(|data| data.get_temp::<usize>(id)).unwrap_or(0);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0;
        for (index, title) in [
            "My account",
            "Application",
            "Download",
            "Linux & Proton",
            "About",
        ]
        .iter()
        .enumerate()
        {
            let selected = index == tab;
            if ui
                .add(
                    egui::Button::new(RichText::new(*title).size(15.0).color(if selected {
                        Color32::from_rgb(20, 23, 36)
                    } else {
                        Color32::WHITE
                    }))
                    .fill(if selected {
                        Color32::WHITE
                    } else {
                        Color32::TRANSPARENT
                    })
                    .stroke(egui::Stroke::NONE)
                    .rounding(8.0)
                    .min_size(vec2(80.0, 34.0)),
                )
                .clicked()
            {
                tab = index;
            }
        }
    });
    ui.ctx().data_mut(|data| data.insert_temp(id, tab));
    ui.add_space(26.0);
    egui::ScrollArea::vertical().id_source("settings_content").auto_shrink(false).show(ui, |ui| {
        let padding = ((ui.available_width() - 860.0) / 2.0).max(0.0);
        egui::Frame::none().inner_margin(egui::Margin::symmetric(padding, 0.0)).show(ui, |ui| {
            ui.add_space(15.0);
            match tab {
                0 => account(app, ui),
                1 => {
                    ui.heading("Application settings");
                    ui.add_space(10.0);
                    enum_dropdown(ui, "Settings_LanguageComboBox".to_owned(), &mut app.settings.language,
                        180.0, &app.locale.localization.settings_view.interface.language, &app.locale);
                    ui.small("Language changes take effect after restarting the client. Some interface text is currently English.");
                    ui.add_space(24.0);
                    ui.label("Native Linux interface powered by Maxima and egui.");
                },
                2 => {
                    ui.heading("Install location");
                    ui.add_space(10.0);
                    ui.label("Default game installation directory");
                    ui.add_sized(vec2(ui.available_width().min(620.0), 38.0),
                        egui::TextEdit::singleline(&mut app.settings.default_install_folder)
                        .hint_text("Paste an existing Linux directory").vertical_align(egui::Align::Center));
                    if !app.settings.default_install_folder.is_empty()
                        && !PathBuf::from(&app.settings.default_install_folder).is_dir() {
                        ui.colored_label(Color32::from_rgb(255, 166, 115), "This directory does not exist yet.");
                    }
                    ui.add_space(18.0);
                    ui.checkbox(&mut app.settings.ignore_ood_games,
                        &app.locale.localization.settings_view.game_installation.ignore_ood_warning);
                },
                3 => { #[cfg(target_os = "linux")] compatibility_panel(app.is_preview(), ui); },
                _ => {
                    ui.heading("EA for Linux · Maxima edition");
                    ui.label("Version 0.1.0 · GPL-3.0");
                    ui.add_space(12.0);
                    ui.label("An unofficial community client. Not affiliated with Electronic Arts.");
                    ui.hyperlink_to("Project source", "https://github.com/slaveitgr/AE-app-for-Linux");
                    ui.hyperlink_to("Upstream Maxima", "https://github.com/ArmchairDevelopers/Maxima");
                    ui.hyperlink_to("umu launcher", "https://github.com/Open-Wine-Components/umu-launcher");
                },
            }
        });
    });
}

fn account(app: &MaximaEguiApp, ui: &mut Ui) {
    ui.heading("My profile");
    ui.label("Manage your profile, payment methods, orders and refunds on EA.com.");
    ui.add_space(22.0);
    ui.horizontal(|ui| {
        ui.image((app.img_cache.placeholder_avatar.id(), vec2(56.0, 56.0)));
        let name = if app.is_preview() {
            "Demo player"
        } else if app.user_id.is_empty() {
            "Not connected"
        } else {
            &app.user_name
        };
        ui.label(RichText::new(name).size(20.0).strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_enabled_ui(!app.is_preview(), |ui| {
                ui.hyperlink_to("MANAGE ↗", "https://myaccount.ea.com/");
            });
        });
    });
    ui.add_space(34.0);
    ui.separator();
    ui.add_space(34.0);
    ui.heading("My membership");
    ui.label("See billing information and manage your membership details on EA.com.");
    ui.add_space(20.0);
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("EA Play").size(27.0).strong().color(Color32::from_rgb(255, 71, 71)),
        );
        ui.label(RichText::new("Membership managed by EA").color(crate::shell::MUTED));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_enabled_ui(!app.is_preview(), |ui| {
                ui.hyperlink_to("MANAGE ↗", "https://myaccount.ea.com/");
            });
        });
    });
    ui.add_space(34.0);
    ui.separator();
    ui.add_space(34.0);
    ui.heading("Privacy");
    ui.label("Manage account privacy and connected accounts on EA.com.");
    ui.add_space(16.0);
    ui.add_enabled_ui(!app.is_preview(), |ui| {
        ui.hyperlink_to("MANAGE PRIVACY ↗", "https://myaccount.ea.com/");
    });
    ui.add_space(18.0);
    ui.small("EA account settings open in your system browser. This client does not change billing or privacy settings.");
    if app.is_preview() {
        ui.add_space(14.0);
        ui.colored_label(
            crate::ACCENT_COLOR,
            "Local demo account. No EA membership or identity is implied.",
        );
    }
}

#[cfg(target_os = "linux")]
fn compatibility_panel(preview: bool, ui: &mut Ui) {
    ui.heading("Linux & Proton");
    ui.add_space(8.0);
    ui.label("Games launch through umu and GE-Proton. The client itself runs natively on Linux.");
    ui.label("Choose a custom Proton build, logging and sync options from each game's settings.");
    ui.add_space(12.0);
    // Resolve paths without maxima_dir(), which creates directories merely by being called.
    let data_root = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
        .map(|root| root.join("maxima"));
    if let Some(root) = data_root {
        egui::Frame::none()
            .fill(Color32::from_rgb(30, 33, 46))
            .inner_margin(egui::Margin::same(16.0))
            .rounding(8.0)
            .show(ui, |ui| {
                runtime_status(ui, "Managed Proton", root.join("wine/proton/proton"));
                runtime_status(ui, "umu launcher", root.join("wine/umu/umu-run"));
                runtime_status(ui, "Shared Wine prefix", root.join("wine/prefix"));
                ui.add_space(8.0);
                ui.label(RichText::new(root.display().to_string()).small().color(Color32::GRAY));
                if ui
                    .add_enabled(
                        !preview && root.is_dir(),
                        egui::Button::new("Open data folder"),
                    )
                    .clicked()
                {
                    if let Err(error) = std::process::Command::new("xdg-open").arg(&root).spawn() {
                        log::warn!("Could not open data folder: {error}");
                    }
                }
            });
    }
    ui.add_space(10.0);
    ui.label("Maxima prepares missing managed runtimes when launching or importing a game. This may download several GB.");
    ui.label(
        "A custom build uses Maxima's shared prefix. Switching builds can update that prefix.",
    );
    ui.hyperlink_to("Check game compatibility", "https://www.protondb.com/");
    ui.label(RichText::new("Proton support depends on the game. EA Javelin and other anti-cheat requirements can prevent play on Linux.")
        .color(Color32::from_rgb(255, 179, 141)));
}

#[cfg(target_os = "linux")]
fn runtime_status(ui: &mut Ui, label: &str, path: PathBuf) {
    ui.horizontal(|ui| {
        ui.add_sized([160.0, 24.0], egui::Label::new(label));
        let present = path.exists();
        ui.colored_label(
            if present {
                Color32::from_rgb(92, 210, 164)
            } else {
                Color32::from_rgb(164, 170, 190)
            },
            if present { "Present" } else { "Not prepared" },
        );
    });
}
