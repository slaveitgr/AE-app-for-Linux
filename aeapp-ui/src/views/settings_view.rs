use crate::{shell, AeAppEguiApp, FrontendLanguage};
use egui::{pos2, vec2, Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, Ui};
use std::path::PathBuf;

const COLUMN_WIDTH: f32 = 888.0;
const COPY: Color32 = Color32::from_rgb(190, 192, 202);
const DIVIDER: Color32 = Color32::from_rgb(52, 54, 71);
const UNAVAILABLE: &str = "This EA app feature is not available in AeApp.";

pub fn settings_view(app: &mut AeAppEguiApp, ui: &mut Ui) {
    ui.spacing_mut().item_spacing.y = 0.0;
    let width = ui.available_width();
    let clip = ui.clip_rect();
    ui.set_clip_rect(Rect::from_min_max(clip.min - vec2(12.0, 0.0), clip.max));
    // The original page has a fixed header and a separately scrolling, centered column.
    let (header, _) = ui.allocate_exact_size(vec2(width, 135.0), Sense::hover());
    text(
        ui,
        header.min,
        "Settings",
        shell::display(32.0),
        shell::TEXT,
        width,
    );
    let id = egui::Id::new("ea_settings_tab");
    let mut selected = ui.ctx().data_mut(|data| data.get_temp::<usize>(id)).unwrap_or(0).min(5);
    let tabs = Rect::from_min_size(header.min + vec2(-12.0, 45.0), vec2(width, 42.0));
    ui.allocate_ui_at_rect(tabs, |ui| {
        egui::ScrollArea::horizontal()
            .id_source("settings_tabs")
            .auto_shrink([false, true])
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    ui.spacing_mut().button_padding = vec2(12.0, 8.0);
                    for (index, title) in [
                        "My account",
                        "Application",
                        "Download",
                        "My playtime",
                        "Family playtime",
                        "Linux & Proton",
                    ]
                    .iter()
                    .enumerate()
                    {
                        let active = selected == index;
                        let response = ui.add(
                            egui::Button::new(
                                egui::RichText::new(*title).font(FontId::proportional(15.0)).color(
                                    if active {
                                        Color32::from_rgb(20, 23, 36)
                                    } else {
                                        shell::TEXT
                                    },
                                ),
                            )
                            .fill(if active {
                                Color32::WHITE
                            } else {
                                Color32::TRANSPARENT
                            })
                            .stroke(Stroke::NONE)
                            .rounding(8.0)
                            .min_size(vec2(0.0, 34.0)),
                        );
                        if response.clicked() {
                            selected = index;
                        }
                    }
                });
            });
    });
    ui.advance_cursor_after_rect(header);
    ui.ctx().data_mut(|data| data.insert_temp(id, selected));
    egui::ScrollArea::vertical()
        .id_source(("settings_content", selected))
        .auto_shrink(false)
        .show(ui, |ui| {
            // The tab row extends twelve pixels left of the page heading.
            let padding = ((width - COLUMN_WIDTH) * 0.5 + 12.0).max(0.0);
            egui::Frame::none().inner_margin(egui::Margin::symmetric(padding, 0.0)).show(
                ui,
                |ui| {
                    ui.set_max_width(COLUMN_WIDTH);
                    ui.spacing_mut().item_spacing.y = 0.0;
                    match selected {
                        0 => account(app, ui),
                        1 => application(app, ui),
                        2 => download(app, ui),
                        3 => playtime(app, ui),
                        4 => family(app, ui),
                        _ => compatibility_panel(app.is_preview(), ui),
                    }
                },
            );
        });
    install_location_editor(app, ui.ctx());
}

fn text(ui: &Ui, at: Pos2, copy: &str, font: FontId, color: Color32, width: f32) {
    let galley = ui.painter().layout(copy.to_owned(), font, color, width.max(1.0));
    ui.painter().galley(at, galley, color);
}
fn heading(ui: &Ui, origin: Pos2, y: f32, title: &str) {
    text(
        ui,
        origin + vec2(0.0, y),
        title,
        shell::bold(20.0),
        shell::TEXT,
        ui.available_width(),
    );
}
fn body(ui: &Ui, origin: Pos2, y: f32, copy: &str, width: f32) {
    text(
        ui,
        origin + vec2(0.0, y),
        copy,
        FontId::proportional(14.0),
        COPY,
        width,
    );
}
fn divider(ui: &Ui, origin: Pos2, y: f32, width: f32) {
    let y = y + 4.0;
    ui.painter().line_segment(
        [origin + vec2(0.0, y), origin + vec2(width, y)],
        Stroke::new(1.0, DIVIDER),
    );
}
fn reserve(ui: &mut Ui, height: f32) -> Rect {
    ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover()).0
}
fn action(
    ui: &mut Ui,
    rect: Rect,
    title: &str,
    enabled: bool,
    primary: bool,
    reason: &str,
) -> bool {
    let rect = rect.translate(vec2(0.0, 4.0));
    let response = ui
        .allocate_ui_at_rect(rect, |ui| {
            ui.add_enabled(
                enabled,
                egui::Button::new(egui::RichText::new(title).font(shell::bold(14.0)))
                    .fill(if primary {
                        crate::ACCENT_COLOR
                    } else {
                        Color32::TRANSPARENT
                    })
                    .stroke(if primary {
                        Stroke::NONE
                    } else {
                        Stroke::new(1.0, Color32::from_rgb(115, 119, 139))
                    })
                    .rounding(4.0)
                    .min_size(rect.size()),
            )
        })
        .inner;
    let clicked = response.clicked();
    if !enabled {
        response.on_disabled_hover_text(reason);
    }
    clicked
}
fn manage(ui: &mut Ui, origin: Pos2, y: f32, width: f32, preview: bool) {
    if action(
        ui,
        Rect::from_min_size(
            origin + vec2((width - 108.0).max(0.0), y),
            vec2(108.0, 32.0),
        ),
        "MANAGE",
        !preview,
        false,
        "Connect an EA account to manage it in your browser.",
    ) {
        ui.ctx().open_url(egui::OpenUrl::new_tab("https://myaccount.ea.com/"));
    }
}
fn avatar(ui: &Ui, rect: Rect) {
    let rect = rect.translate(vec2(0.0, 4.0));
    let painter = ui.painter();
    painter.rect_filled(rect, 5.0, Color32::from_rgb(80, 83, 96));
    let center = rect.center();
    painter.circle_filled(
        center + vec2(0.0, -10.0),
        10.0,
        Color32::from_rgb(239, 240, 246),
    );
    let shoulders = Rect::from_center_size(center + vec2(0.0, 15.0), vec2(35.0, 23.0));
    painter.rect_filled(
        shoulders,
        egui::Rounding {
            nw: 16.0,
            ne: 16.0,
            sw: 3.0,
            se: 3.0,
        },
        Color32::from_rgb(239, 240, 246),
    );
}
fn toggle(ui: &mut Ui, at: Pos2, id: &str, value: &mut bool, enabled: bool, reason: &str) {
    let rect = Rect::from_min_size(at + vec2(0.0, 4.0), vec2(48.0, 26.0));
    let response = ui.interact(
        rect,
        ui.make_persistent_id(id),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    if enabled && response.clicked() {
        *value = !*value;
    }
    let fill = if enabled && *value {
        crate::ACCENT_COLOR
    } else {
        Color32::from_rgb(65, 68, 84)
    };
    ui.painter().rect_filled(rect, 13.0, fill);
    ui.painter().circle_filled(
        pos2(
            if *value {
                rect.right() - 13.0
            } else {
                rect.left() + 13.0
            },
            rect.center().y,
        ),
        9.0,
        if enabled {
            Color32::WHITE
        } else {
            Color32::from_rgb(143, 146, 163)
        },
    );
    if enabled {
        response.on_hover_cursor(egui::CursorIcon::PointingHand);
    } else {
        response.on_hover_text(reason);
    }
}
fn disabled_toggle(ui: &mut Ui, origin: Pos2, y: f32, width: f32, id: &str) {
    toggle(
        ui,
        origin + vec2((width - 48.0).max(0.0), y),
        id,
        &mut false,
        false,
        UNAVAILABLE,
    );
}
fn setting_label(ui: &Ui, origin: Pos2, y: f32, title: &str, width: f32) {
    text(
        ui,
        origin + vec2(0.0, y),
        title,
        FontId::proportional(16.0),
        shell::TEXT,
        (width - 240.0).max(140.0),
    );
}

fn account(app: &AeAppEguiApp, ui: &mut Ui) {
    let rect = reserve(ui, 1080.0);
    let origin = rect.min;
    let width = rect.width();
    heading(ui, origin, 0.0, "My profile");
    body(
        ui,
        origin,
        28.0,
        "Manage your profile, payment methods, orders and refunds.",
        width,
    );
    shell::account_avatar(
        app,
        ui,
        Rect::from_min_size(origin + vec2(0.0, 66.0), vec2(64.0, 64.0)),
    );
    let name = if app.is_preview() {
        "Demo player"
    } else if app.user_id.is_empty() {
        "Not connected"
    } else {
        &app.user_name
    };
    text(
        ui,
        origin + vec2(88.0, 84.0),
        name,
        shell::bold(20.0),
        shell::TEXT,
        (width - 220.0).max(80.0),
    );
    manage(ui, origin, 82.0, width, app.is_preview());
    divider(ui, origin, 194.0, width);

    heading(ui, origin, 264.0, "My membership");
    body(ui, origin, 292.0, "See your billing information, manage your payment method, and edit your membership details.", width);
    let logo = Rect::from_min_size(origin + vec2(0.0, 330.0), vec2(160.0, 46.0));
    let medallion = Rect::from_min_size(logo.min + vec2(0.0, 4.0), vec2(32.0, 32.0));
    shell::original_icon(
        ui,
        medallion,
        "EA_PLAY_MEDALLION",
        Color32::from_rgb(255, 71, 76),
    );
    text(
        ui,
        logo.min + vec2(40.0, 4.0),
        "Play",
        shell::display(24.0),
        Color32::from_rgb(255, 71, 76),
        120.0,
    );
    ui.interact(logo, ui.id().with("membership_status"), Sense::hover()).on_hover_text(
        "Membership details are not loaded. Manage opens your EA account in the browser.",
    );
    manage(ui, origin, 333.0, width, app.is_preview());
    divider(ui, origin, 433.0, width);

    heading(ui, origin, 503.0, "Privacy");
    body(
        ui,
        origin,
        531.0,
        "Manage your data settings, and control who can see your profile and library.",
        (width - 140.0).max(120.0),
    );
    manage(ui, origin, 506.0, width, app.is_preview());
    setting_label(ui, origin, 572.0, "Usage sharing", width);
    body(
        ui,
        origin,
        596.0,
        "Share usage information to help improve the EA app.",
        width - 80.0,
    );
    disabled_toggle(ui, origin, 575.0, width, "usage_sharing");
    setting_label(ui, origin, 638.0, "Blocked players", width);
    body(
        ui,
        origin,
        662.0,
        "Players you've blocked can't view your profile or contact you.",
        width,
    );
    let blocked = Rect::from_min_size(origin + vec2(0.0, 695.0), vec2(width, 72.0));
    ui.painter().rect_stroke(blocked, 4.0, Stroke::new(1.0, DIVIDER));
    ui.painter().text(
        blocked.center(),
        Align2::CENTER_CENTER,
        "Blocked players are not loaded.",
        FontId::proportional(14.0),
        COPY,
    );
    divider(ui, origin, 811.0, width);
    heading(ui, origin, 859.0, "Connected accounts");
    body(
        ui,
        origin,
        887.0,
        "Manage the accounts connected to your EA Account.",
        width - 140.0,
    );
    setting_label(ui, origin, 943.0, "Discord", width);
    body(
        ui,
        origin,
        967.0,
        "Connected account details are not loaded.",
        width - 140.0,
    );
    manage(ui, origin, 946.0, width, app.is_preview());
}

fn application(app: &mut AeAppEguiApp, ui: &mut Ui) {
    let rect = reserve(ui, 1220.0);
    let origin = rect.min;
    let width = rect.width();
    heading(ui, origin, 0.0, "Application settings");
    setting_label(ui, origin, 62.0, "Language", width);
    let control = Rect::from_min_size(
        origin + vec2((width - 200.0).max(0.0), 58.0),
        vec2(200.0, 40.0),
    );
    ui.allocate_ui_at_rect(control, |ui| {
        combo_style(ui);
        egui::ComboBox::from_id_source("settings_language")
            .width(200.0)
            .selected_text(match app.settings.language {
                FrontendLanguage::SystemDefault => "System default",
                FrontendLanguage::EnUS => "English",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut app.settings.language,
                    FrontendLanguage::SystemDefault,
                    "System default",
                );
                ui.selectable_value(
                    &mut app.settings.language,
                    FrontendLanguage::EnUS,
                    "English",
                );
            })
            .response
            .on_hover_text("Language changes take effect after restarting AeApp.");
    });
    setting_label(ui, origin, 144.0, "Select region", width);
    inactive_select(
        ui,
        origin + vec2((width - 200.0).max(0.0), 136.0),
        "Not available",
        UNAVAILABLE,
    );
    for (y, label, id) in [
        (209.0, "Install updates automatically", "app_updates"),
        (264.0, "Run on startup", "app_startup"),
        (
            314.0,
            "Stay signed in with your Steam-linked EA Account",
            "steam_account",
        ),
    ] {
        setting_label(ui, origin, y, label, width);
        disabled_toggle(ui, origin, y - 1.0, width, id);
    }
    divider(ui, origin, 394.0, width);
    heading(ui, origin, 464.0, "Notifications");
    body(
        ui,
        origin,
        492.0,
        "Choose when you want to receive notifications.",
        width,
    );
    let game_x = (width - 250.0).max(220.0);
    let app_x = (width - 90.0).max(game_x + 72.0);
    text(
        ui,
        origin + vec2(game_x, 540.0),
        "In games",
        FontId::proportional(14.0),
        COPY,
        90.0,
    );
    text(
        ui,
        origin + vec2(app_x, 540.0),
        "In app",
        FontId::proportional(14.0),
        COPY,
        90.0,
    );
    for (index, label) in [
        "Incoming chat messages",
        "Game invites",
        "Friends logging on",
        "Friend requests",
        "Sales and promotions",
    ]
    .iter()
    .enumerate()
    {
        let y = 586.0 + index as f32 * 48.0;
        setting_label(ui, origin, y, label, width);
        inactive_check(
            ui,
            origin + vec2(game_x + 18.0, y - 1.0),
            ("notify_game", index),
        );
        inactive_check(
            ui,
            origin + vec2(app_x + 10.0, y - 1.0),
            ("notify_app", index),
        );
    }
    divider(ui, origin, 856.0, width);
    heading(ui, origin, 904.0, "In-game overlay");
    setting_label(ui, origin, 952.0, "Enable in-game overlay", width);
    disabled_toggle(ui, origin, 951.0, width, "overlay");
    divider(ui, origin, 1024.0, width);
    heading(ui, origin, 1072.0, "Linux interface");
    setting_label(ui, origin, 1120.0, "Enable background blur", width);
    let mut blur = !app.settings.performance_settings.disable_blur;
    toggle(
        ui,
        origin + vec2((width - 48.0).max(0.0), 1119.0),
        "background_blur",
        &mut blur,
        true,
        "",
    );
    app.settings.performance_settings.disable_blur = !blur;
    body(
        ui,
        origin,
        1150.0,
        "Some interface text is currently English.",
        width,
    );
}
fn combo_style(ui: &mut Ui) {
    ui.spacing_mut().interact_size.y = 40.0;
    ui.visuals_mut().widgets.inactive.bg_fill = Color32::from_rgb(35, 38, 55);
    ui.visuals_mut().widgets.inactive.weak_bg_fill = Color32::from_rgb(35, 38, 55);
    ui.visuals_mut().widgets.inactive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(71, 75, 96));
    ui.visuals_mut().widgets.inactive.rounding = egui::Rounding::same(4.0);
}
fn inactive_select(ui: &mut Ui, at: Pos2, title: &str, reason: &str) {
    let at = at + vec2(0.0, 4.0);
    let rect = Rect::from_min_size(at, vec2(200.0, 40.0));
    ui.painter().rect_filled(rect, 4.0, Color32::from_rgb(35, 38, 55));
    ui.painter().rect_stroke(rect, 4.0, Stroke::new(1.0, DIVIDER));
    text(
        ui,
        at + vec2(12.0, 10.0),
        title,
        FontId::proportional(14.0),
        COPY,
        164.0,
    );
    let c = rect.right_center() - vec2(17.0, 0.0);
    ui.painter().line_segment(
        [c - vec2(4.0, 2.0), c + vec2(0.0, 2.0)],
        Stroke::new(1.5, COPY),
    );
    ui.painter().line_segment(
        [c + vec2(0.0, 2.0), c + vec2(4.0, -2.0)],
        Stroke::new(1.5, COPY),
    );
    ui.interact(rect, ui.id().with(title), Sense::hover()).on_hover_text(reason);
}
fn inactive_check(ui: &mut Ui, at: Pos2, id: (&str, usize)) {
    let at = at + vec2(0.0, 4.0);
    let rect = Rect::from_min_size(at, vec2(22.0, 22.0));
    ui.painter().rect_stroke(
        rect,
        3.0,
        Stroke::new(1.0, Color32::from_rgb(101, 105, 124)),
    );
    ui.interact(rect, ui.id().with(id), Sense::hover()).on_hover_text(UNAVAILABLE);
}

fn download(app: &mut AeAppEguiApp, ui: &mut Ui) {
    let rect = reserve(ui, 570.0);
    let origin = rect.min;
    let width = rect.width();
    heading(ui, origin, 0.0, "Downloads");
    setting_label(ui, origin, 49.0, "Install location", width);
    let location = if app.settings.default_install_folder.is_empty() {
        "Choose a folder when installing a game"
    } else {
        &app.settings.default_install_folder
    };
    body(ui, origin, 74.0, location, (width - 140.0).max(120.0));
    if action(
        ui,
        Rect::from_min_size(
            origin + vec2((width - 108.0).max(0.0), 50.0),
            vec2(108.0, 32.0),
        ),
        "EDIT",
        true,
        false,
        "",
    ) {
        ui.ctx().data_mut(|data| {
            data.insert_temp(egui::Id::new("install_location_editor"), true);
            data.insert_temp(
                egui::Id::new("install_location_draft"),
                app.settings.default_install_folder.clone(),
            );
        });
    }
    setting_label(ui, origin, 122.0, "Limit download speed", width);
    inactive_select(
        ui,
        origin + vec2((width - 200.0).max(0.0), 114.0),
        "Unlimited",
        "AeApp does not currently support a download speed limit.",
    );
    for (y, label, id) in [
        (170.0, "Pause downloads during gameplay", "pause_in_game"),
        (218.0, "Update games automatically", "auto_game_updates"),
        (
            268.0,
            "Continue updates when the app is closed",
            "background_updates",
        ),
    ] {
        setting_label(ui, origin, y, label, width);
        disabled_toggle(ui, origin, y - 1.0, width, id);
    }
    divider(ui, origin, 348.0, width);
    heading(ui, origin, 396.0, "Game version checks");
    setting_label(
        ui,
        origin,
        448.0,
        "Skip the out-of-date game warning",
        width,
    );
    toggle(
        ui,
        origin + vec2((width - 48.0).max(0.0), 447.0),
        "ignore_outdated",
        &mut app.settings.ignore_ood_games,
        true,
        "",
    );
    body(
        ui,
        origin,
        480.0,
        "Launch installed games without prompting when a newer version is available.",
        width - 80.0,
    );
}
fn install_location_editor(app: &mut AeAppEguiApp, ctx: &egui::Context) {
    let id = egui::Id::new("install_location_editor");
    let mut open = ctx.data_mut(|data| data.get_temp::<bool>(id)).unwrap_or(false);
    if !open {
        return;
    }
    let draft_id = egui::Id::new("install_location_draft");
    let mut draft = ctx.data_mut(|data| data.get_temp::<String>(draft_id)).unwrap_or_default();
    let mut apply = false;
    let mut cancel = false;
    egui::Window::new("Install location")
        .id(id)
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .default_width(520.0)
        .show(ctx, |ui| {
            ui.label("Default folder for new game installations");
            ui.add_space(12.0);
            ui.add_sized(
                [ui.available_width(), 40.0],
                egui::TextEdit::singleline(&mut draft)
                    .hint_text("Absolute Linux directory, or leave empty"),
            );
            let path = PathBuf::from(draft.trim());
            let valid = draft.trim().is_empty() || (path.is_absolute() && path.is_dir());
            ui.add_space(8.0);
            if !valid {
                ui.colored_label(
                    Color32::from_rgb(255, 179, 141),
                    "Choose an existing absolute directory.",
                );
            } else {
                ui.label("Leave empty to choose a folder for each installation.");
            }
            ui.add_space(16.0);
            ui.horizontal(|ui| {
                if ui.add_enabled(valid, egui::Button::new("Apply")).clicked() {
                    apply = true;
                }
                if ui.button("Cancel").clicked() {
                    cancel = true;
                }
            });
        });
    if apply {
        app.settings.default_install_folder = draft.trim().to_owned();
    }
    if apply || cancel {
        open = false;
    }
    ctx.data_mut(|data| {
        data.insert_temp(id, open);
        data.insert_temp(draft_id, draft);
    });
}

fn playtime(app: &AeAppEguiApp, ui: &mut Ui) {
    let rect = reserve(ui, 470.0);
    let origin = rect.min;
    let width = rect.width();
    let copy_width = (width - 330.0).max(240.0);
    text(
        ui,
        origin + vec2(0.0, 35.0),
        "Track your playtime",
        shell::bold(24.0),
        shell::TEXT,
        copy_width,
    );
    body(ui, origin, 78.0, "See how much time you spend playing, and set a weekly playtime goal to find a balance that works for you.", copy_width);
    action(
        ui,
        Rect::from_min_size(origin + vec2(0.0, 120.0), vec2(246.0, 32.0)),
        "ENABLE PLAYTIME TRACKING",
        false,
        true,
        "Playtime tracking and goals are not yet available in AeApp.",
    );
    illustration(
        ui,
        Rect::from_min_size(
            origin + vec2((width - 280.0).max(0.0), 8.0),
            vec2(260.0, 190.0),
        ),
        false,
    );
    divider(ui, origin, 234.0, width);
    heading(ui, origin, 288.0, "Spending controls");
    body(
        ui,
        origin,
        320.0,
        "Manage your spending limits and preferences in your EA Account.",
        (width - 150.0).max(200.0),
    );
    manage(ui, origin, 290.0, width, app.is_preview());
}
fn family(app: &AeAppEguiApp, ui: &mut Ui) {
    let rect = reserve(ui, 500.0);
    let origin = rect.min;
    let width = rect.width();
    let copy_width = (width - 350.0).max(240.0);
    text(
        ui,
        origin + vec2(0.0, 17.0),
        "Help your teen find a healthy balance with games",
        shell::bold(28.0),
        shell::TEXT,
        copy_width,
    );
    body(
        ui,
        origin,
        96.0,
        "Connect your teen's EA Account to your family to manage their playtime and spending.",
        copy_width,
    );
    action(ui, Rect::from_min_size(origin + vec2(0.0, 139.0), vec2(192.0, 32.0)), "ADD TEEN ACCOUNT", false, true, "Family account management is not available in AeApp. Manage your family through your EA Account.");
    illustration(
        ui,
        Rect::from_min_size(
            origin + vec2((width - 300.0).max(0.0), 4.0),
            vec2(280.0, 210.0),
        ),
        true,
    );
    body(
        ui,
        origin,
        260.0,
        "Your family account information is not loaded.",
        copy_width,
    );
    manage(ui, origin, 252.0, width, app.is_preview());
}
// An independently drawn fallback. Original user-owned illustrations may be supplied locally.
fn illustration(ui: &Ui, rect: Rect, family: bool) {
    let rect = rect.translate(vec2(0.0, 4.0));
    let painter = ui.painter().with_clip_rect(ui.clip_rect().intersect(rect));
    let at = |x: f32, y: f32| rect.min + vec2(x / 280.0 * rect.width(), y / 210.0 * rect.height());
    painter.circle_filled(
        at(153.0, 99.0),
        rect.width() * 0.3,
        Color32::from_rgb(35, 43, 74),
    );
    painter.circle_filled(at(239.0, 32.0), 5.0, Color32::from_rgb(95, 193, 210));
    painter.circle_filled(at(38.0, 150.0), 4.0, crate::ACCENT_COLOR);
    let face = Color32::from_rgb(232, 190, 161);
    if family {
        painter.circle_filled(at(107.0, 70.0), 18.0, face);
        painter.rect_filled(
            Rect::from_min_max(at(79.0, 92.0), at(137.0, 166.0)),
            18.0,
            Color32::from_rgb(99, 148, 244),
        );
        painter.circle_filled(at(179.0, 98.0), 15.0, face);
        painter.rect_filled(
            Rect::from_min_max(at(155.0, 118.0), at(204.0, 174.0)),
            16.0,
            Color32::from_rgb(247, 182, 95),
        );
        painter.line_segment(
            [at(133.0, 115.0), at(162.0, 135.0)],
            Stroke::new(11.0, face),
        );
    } else {
        let clock = at(158.0, 90.0);
        painter.circle_filled(clock, 55.0, Color32::from_rgb(231, 236, 252));
        painter.circle_stroke(
            clock,
            44.0,
            Stroke::new(3.0, Color32::from_rgb(63, 91, 155)),
        );
        painter.line_segment(
            [clock, clock + vec2(0.0, -25.0)],
            Stroke::new(4.0, Color32::from_rgb(47, 65, 114)),
        );
        painter.line_segment(
            [clock, clock + vec2(19.0, 12.0)],
            Stroke::new(4.0, Color32::from_rgb(47, 65, 114)),
        );
        painter.rect_filled(
            Rect::from_min_max(at(41.0, 138.0), at(137.0, 182.0)),
            18.0,
            Color32::from_rgb(91, 137, 231),
        );
        for x in [63.0, 78.0] {
            painter.circle_filled(at(x, 158.0), 4.0, Color32::WHITE);
        }
        painter.circle_filled(at(113.0, 158.0), 4.0, Color32::from_rgb(228, 188, 255));
    }
}

fn compatibility_panel(preview: bool, ui: &mut Ui) {
    ui.heading("Linux & Proton");
    ui.add_space(12.0);
    ui.label("Games launch through umu and GE-Proton. AeApp itself runs natively on Linux.");
    ui.label("Choose a custom Proton build, logging and sync options in each game's settings.");
    ui.add_space(28.0);
    let data_root = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
        .map(|root| root.join("aeapp"));
    if let Some(root) = data_root {
        egui::Frame::none()
            .fill(Color32::from_rgb(30, 33, 46))
            .inner_margin(egui::Margin::same(20.0))
            .rounding(8.0)
            .show(ui, |ui| {
                runtime_status(ui, "Managed Proton", root.join("wine/proton/proton"));
                runtime_status(ui, "umu launcher", root.join("wine/umu/umu-run"));
                runtime_status(ui, "Shared Wine prefix", root.join("wine/prefix"));
                ui.add_space(12.0);
                ui.label(egui::RichText::new(root.display().to_string()).small().color(COPY));
                #[cfg(target_os = "linux")]
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
    #[cfg(not(target_os = "linux"))]
    let _ = preview;
    ui.add_space(20.0);
    ui.label("AeApp prepares missing managed runtimes when launching or importing a game. This may download several GB.");
    ui.add_space(8.0);
    ui.label("A custom build uses AeApp's shared prefix. Switching builds can update that prefix.");
    ui.add_space(12.0);
    ui.hyperlink_to("Check game compatibility", "https://www.protondb.com/");
    ui.add_space(8.0);
    ui.label(egui::RichText::new("Proton support depends on the game. EA Javelin and other anti-cheat requirements can prevent play on Linux.").color(Color32::from_rgb(255, 179, 141)));
    ui.add_space(28.0);
    ui.separator();
    ui.add_space(20.0);
    ui.label(egui::RichText::new("AeApp · 0.1.0").font(shell::bold(18.0)));
    ui.label("An unofficial community client. Not affiliated with Electronic Arts. Source licensed under GPL-3.0.");
    ui.add_space(8.0);
    ui.hyperlink_to(
        "Project source",
        "https://github.com/slaveitgr/AE-app-for-Linux",
    );
    ui.hyperlink_to(
        "umu launcher",
        "https://github.com/Open-Wine-Components/umu-launcher",
    );
}
fn runtime_status(ui: &mut Ui, label: &str, path: PathBuf) {
    ui.horizontal(|ui| {
        ui.add_sized([180.0, 32.0], egui::Label::new(label));
        let present = path.exists();
        ui.colored_label(
            if present {
                Color32::from_rgb(92, 210, 164)
            } else {
                COPY
            },
            if present { "Present" } else { "Not prepared" },
        );
    });
}
