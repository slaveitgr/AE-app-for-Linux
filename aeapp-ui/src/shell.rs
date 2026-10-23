use crate::{
    views::{downloads_view::downloads_view, game_view::games_view, settings_view::settings_view},
    AeAppEguiApp, BackendStallState, PageType, ACCENT_COLOR,
};
use egui::{
    pos2, vec2, Align, Align2, Color32, FontId, Layout, Margin, Rect, RichText, Rounding, Stroke,
    Ui,
};

pub const BACKGROUND: Color32 = Color32::from_rgb(23, 26, 41);
pub const SIDEBAR: Color32 = Color32::from_rgb(20, 23, 36);
pub const SURFACE: Color32 = Color32::from_rgb(29, 32, 51);
pub const SURFACE_HOVER: Color32 = Color32::from_rgb(52, 54, 71);
pub const BORDER: Color32 = Color32::from_rgb(52, 54, 71);
pub const TEXT: Color32 = Color32::from_rgb(244, 245, 249);
pub const MUTED: Color32 = Color32::from_rgb(144, 142, 151);
pub const GREEN: Color32 = Color32::from_rgb(104, 218, 173);

pub fn pill(ui: &mut Ui, text: &str, color: Color32) {
    egui::Frame::none()
        .fill(color.linear_multiply(0.13))
        .rounding(20.0)
        .inner_margin(Margin::symmetric(11.0, 5.0))
        .show(ui, |ui| {
            ui.label(RichText::new(text).color(color).size(12.0));
        });
}

pub fn primary_button(ui: &mut Ui, label: &str) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(label).color(Color32::WHITE).strong())
            .fill(ACCENT_COLOR)
            .stroke(Stroke::NONE)
            .rounding(8.0)
            .min_size(vec2(140.0, 48.0)),
    )
}

#[derive(Clone)]
struct History {
    pages: Vec<PageType>,
    index: usize,
}
fn history_key() -> egui::Id {
    egui::Id::new("ea-shell-navigation")
}
fn record_page(app: &AeAppEguiApp, ctx: &egui::Context) {
    let mut history = ctx.data(|d| d.get_temp::<History>(history_key())).unwrap_or(History {
        pages: vec![app.page_view.clone()],
        index: 0,
    });
    if history.pages[history.index] != app.page_view {
        history.pages.truncate(history.index + 1);
        history.pages.push(app.page_view.clone());
        history.index = history.pages.len() - 1;
    }
    ctx.data_mut(|d| d.insert_temp(history_key(), history));
}
fn navigate(app: &mut AeAppEguiApp, page: PageType, ctx: &egui::Context) {
    app.page_view = page;
    app.library_details = false;
    record_page(app, ctx);
}
fn change_history(app: &mut AeAppEguiApp, ctx: &egui::Context, forward: bool) {
    if !forward && app.library_details {
        app.library_details = false;
        return;
    }
    if let Some(mut history) = ctx.data(|d| d.get_temp::<History>(history_key())) {
        if forward && history.index + 1 < history.pages.len() {
            history.index += 1;
        } else if !forward && history.index > 0 {
            history.index -= 1;
        }
        app.page_view = history.pages[history.index].clone();
        app.library_details = false;
        ctx.data_mut(|d| d.insert_temp(history_key(), history));
    }
}
fn profile_key() -> egui::Id {
    egui::Id::new("ea-own-profile-summary")
}

fn clipped_text(ui: &Ui, rect: Rect, text: &str, size: f32, color: Color32) {
    let mut job = egui::text::LayoutJob::simple_singleline(
        text.to_owned(),
        FontId::proportional(size),
        color,
    );
    job.wrap.max_width = rect.width();
    job.wrap.max_rows = 1;
    job.wrap.overflow_character = Some('…');
    let galley = ui.fonts(|fonts| fonts.layout_job(job));
    ui.painter().with_clip_rect(rect).galley(rect.min, galley, color);
}
fn navigation_icon(ui: &Ui, center: egui::Pos2, kind: usize, color: Color32) {
    let p = ui.painter();
    let stroke = Stroke::new(1.7, color);
    let at = |x, y| center + vec2(x, y);
    match kind {
        0 => {
            p.line_segment([at(-9.0, -1.0), at(0.0, -9.0)], stroke);
            p.line_segment([at(0.0, -9.0), at(9.0, -1.0)], stroke);
            p.rect_stroke(
                Rect::from_min_max(at(-6.0, -1.0), at(6.0, 8.0)),
                1.0,
                stroke,
            );
        }
        1 => {
            p.circle_stroke(center, 8.0, stroke);
            p.line_segment([at(3.0, -4.0), at(-3.0, 4.0)], stroke);
            p.line_segment([at(-3.0, 4.0), at(-2.0, -1.0)], stroke);
            p.line_segment([at(3.0, -4.0), at(2.0, 1.0)], stroke);
        }
        2 => {
            p.rect_stroke(
                Rect::from_min_max(at(-7.0, -4.0), at(7.0, 9.0)),
                1.0,
                stroke,
            );
            p.line_segment([at(-5.0, -7.0), at(5.0, -7.0)], stroke);
            p.line_segment([at(-3.0, -10.0), at(3.0, -10.0)], stroke);
        }
        3 => {
            p.circle_stroke(center, 9.0, stroke);
            p.add(egui::Shape::convex_polygon(
                vec![at(-2.0, -4.0), at(-2.0, 4.0), at(4.0, 0.0)],
                color,
                Stroke::NONE,
            ));
        }
        4 => {
            p.line_segment([at(0.0, -8.0), at(0.0, 4.0)], stroke);
            p.line_segment([at(-5.0, -1.0), at(0.0, 4.0)], stroke);
            p.line_segment([at(0.0, 4.0), at(5.0, -1.0)], stroke);
            p.line_segment([at(-8.0, 8.0), at(8.0, 8.0)], stroke);
        }
        _ => {
            p.circle_stroke(center, 5.0, stroke);
            p.circle_stroke(center, 1.5, stroke);
            for n in 0..8 {
                let angle = n as f32 * std::f32::consts::TAU / 8.0;
                let d = vec2(angle.cos(), angle.sin());
                p.line_segment(
                    [center + d * 6.0, center + d * 9.0],
                    Stroke::new(2.3, color),
                );
            }
        }
    }
}
fn avatar(ui: &Ui, rect: Rect, name: &str, status: Option<Color32>) {
    ui.painter().circle_filled(
        rect.center(),
        rect.width() * 0.5,
        Color32::from_rgb(83, 77, 106),
    );
    let letter: String = name.chars().take(1).collect();
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        letter.to_uppercase(),
        FontId::proportional(rect.width() * 0.40),
        TEXT,
    );
    if let Some(color) = status {
        let pos = rect.right_bottom() - vec2(3.0, 3.0);
        ui.painter().circle_filled(pos, 6.0, SIDEBAR);
        ui.painter().circle_filled(pos, 4.0, color);
    }
}
fn titlebar(app: &mut AeAppEguiApp, ui: &mut Ui, rect: Rect) {
    ui.painter().rect_filled(rect, 0.0, Color32::BLACK);
    let drag_rect = Rect::from_min_max(rect.min + vec2(75.0, 0.0), rect.max - vec2(140.0, 0.0));
    let drag = ui.interact(
        drag_rect,
        ui.id().with("app-window-drag"),
        egui::Sense::click_and_drag(),
    );
    if drag.double_clicked() {
        let maximized = ui.ctx().input(|i| i.viewport().maximized.unwrap_or(false));
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
    } else if drag.drag_started() {
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
    }
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        "EA",
        FontId::proportional(21.0),
        Color32::WHITE,
    );
    let menu_rect = Rect::from_min_size(rect.min + vec2(4.0, 2.0), vec2(66.0, 36.0));
    ui.allocate_ui_at_rect(menu_rect, |ui| {
        ui.spacing_mut().button_padding = vec2(12.0, 6.0);
        let response = ui.menu_button("     ", |ui| {
            ui.set_min_width(180.0);
            if ui.button("Settings").clicked() {
                navigate(app, PageType::Settings, ui.ctx());
                ui.close_menu();
            }
            ui.separator();
            ui.label(egui::RichText::new("Unofficial Linux client").size(11.0).color(MUTED));
            ui.separator();
            if ui.button("Exit").clicked() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });
        let c = response.response.rect.center();
        for offset in [-5.0, 0.0, 5.0] {
            ui.painter().line_segment(
                [c + vec2(-7.0, offset), c + vec2(7.0, offset)],
                Stroke::new(1.5, TEXT),
            );
        }
    });
    for index in 0..3 {
        let button_rect = Rect::from_min_size(
            pos2(rect.right() - 126.0 + index as f32 * 42.0, rect.top()),
            vec2(42.0, 40.0),
        );
        let response = ui.interact(
            button_rect,
            ui.id().with(("window-control", index)),
            egui::Sense::click(),
        );
        if response.hovered() {
            ui.painter().rect_filled(
                button_rect,
                0.0,
                if index == 2 {
                    Color32::from_rgb(172, 42, 51)
                } else {
                    SURFACE_HOVER
                },
            );
        }
        let c = button_rect.center();
        let stroke = Stroke::new(1.0, TEXT);
        match index {
            0 => {
                ui.painter().line_segment([c + vec2(-5.0, 3.0), c + vec2(5.0, 3.0)], stroke);
            }
            1 => {
                ui.painter().rect_stroke(Rect::from_center_size(c, vec2(10.0, 9.0)), 0.0, stroke);
            }
            _ => {
                ui.painter().line_segment([c - vec2(5.0, 5.0), c + vec2(5.0, 5.0)], stroke);
                ui.painter().line_segment([c + vec2(-5.0, 5.0), c + vec2(5.0, -5.0)], stroke);
            }
        }
        if response.clicked() {
            match index {
                0 => ui.ctx().send_viewport_cmd(egui::ViewportCommand::Minimized(true)),
                1 => {
                    let maximized = ui.ctx().input(|i| i.viewport().maximized.unwrap_or(false));
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
                }
                _ => ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close),
            }
        }
    }
}
fn navigation_row(ui: &mut Ui, text: &str, kind: usize, selected: bool) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), 56.0), egui::Sense::click());
    if selected || response.hovered() {
        ui.painter().rect_filled(rect, 0.0, if selected { SURFACE_HOVER } else { SURFACE });
    }
    if selected {
        ui.painter().rect_filled(
            Rect::from_min_size(rect.min, vec2(4.0, rect.height())),
            0.0,
            TEXT,
        );
    }
    navigation_icon(
        ui,
        pos2(rect.left() + 30.0, rect.center().y),
        kind,
        if selected { TEXT } else { MUTED },
    );
    ui.painter().text(
        pos2(rect.left() + 56.0, rect.center().y),
        Align2::LEFT_CENTER,
        text,
        FontId::proportional(18.0),
        if selected { TEXT } else { MUTED },
    );
    response
}
fn navigation(app: &mut AeAppEguiApp, ui: &mut Ui) {
    ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
    ui.add_space(12.0);
    if navigation_row(ui, "Home", 0, app.page_view == PageType::Home).clicked() {
        navigate(app, PageType::Home, ui.ctx());
    }
    if navigation_row(ui, "Browse  ↗", 1, false)
        .on_hover_text("Open EA's official games website")
        .clicked()
    {
        ui.ctx().open_url(egui::OpenUrl::new_tab("https://www.ea.com/games"));
    }
    if navigation_row(ui, "Library", 2, app.page_view == PageType::Games).clicked() {
        navigate(app, PageType::Games, ui.ctx());
    }
    if navigation_row(ui, "EA Play  ↗", 3, false)
        .on_hover_text("Open EA Play on the official EA website")
        .clicked()
    {
        ui.ctx().open_url(egui::OpenUrl::new_tab("https://www.ea.com/ea-play"));
    }
    ui.add_space(25.0);
    ui.horizontal(|ui| {
        ui.add_space(24.0);
        ui.label(RichText::new("Installed games").size(13.0).color(MUTED));
    });
    ui.add_space(12.0);
    let mut games: Vec<_> = app
        .games
        .values()
        .filter(|g| g.installed)
        .map(|g| (g.slug.clone(), g.name.clone()))
        .collect();
    games.sort_by(|a, b| a.1.cmp(&b.1));
    let remaining = (ui.max_rect().bottom() - ui.cursor().min.y - 110.0).max(40.0);
    egui::ScrollArea::vertical()
        .id_source("sidebar-installed-games")
        .max_height(remaining)
        .auto_shrink(false)
        .show(ui, |ui| {
            if games.is_empty() {
                ui.horizontal(|ui| {
                    ui.add_space(24.0);
                    ui.label(RichText::new("No games installed").size(12.0).color(MUTED));
                });
            }
            for (slug, name) in games {
                let (rect, response) =
                    ui.allocate_exact_size(vec2(ui.available_width(), 52.0), egui::Sense::click());
                if response.hovered() {
                    ui.painter().rect_filled(rect, 0.0, SURFACE);
                }
                let thumb = Rect::from_min_size(rect.min + vec2(24.0, 8.0), vec2(30.0, 36.0));
                ui.painter().rect_filled(thumb, 3.0, Color32::from_rgb(65, 76, 98));
                ui.painter().text(
                    thumb.center(),
                    Align2::CENTER_CENTER,
                    name.chars().next().unwrap_or('G'),
                    FontId::proportional(17.0),
                    TEXT,
                );
                clipped_text(
                    ui,
                    Rect::from_min_size(
                        rect.min + vec2(65.0, 12.0),
                        vec2(rect.width() - 77.0, 21.0),
                    ),
                    &name,
                    13.0,
                    TEXT,
                );
                if app.is_preview() {
                    clipped_text(
                        ui,
                        Rect::from_min_size(
                            rect.min + vec2(65.0, 32.0),
                            vec2(rect.width() - 77.0, 15.0),
                        ),
                        "Demo title",
                        10.0,
                        MUTED,
                    );
                }
                if response.clicked() {
                    navigate(app, PageType::Games, ui.ctx());
                    app.game_sel = slug;
                    app.library_details = true;
                }
            }
        });
    let footer = Rect::from_min_max(
        pos2(ui.max_rect().left(), ui.max_rect().bottom() - 100.0),
        ui.max_rect().right_bottom(),
    );
    ui.allocate_ui_at_rect(footer, |ui| {
        ui.with_layout(Layout::top_down(Align::Min), |ui| {
            ui.horizontal(|ui| {
                ui.add_space(22.0);
                if ui.small_button("Downloads").clicked() {
                    navigate(app, PageType::Downloads, ui.ctx());
                }
                if ui.small_button("Settings").clicked() {
                    navigate(app, PageType::Settings, ui.ctx());
                }
            });
            ui.add_space(14.0);
            ui.horizontal(|ui| {
                ui.add_space(24.0);
                ui.label(RichText::new("aeapp · unofficial Linux client").size(10.0).color(MUTED));
            });
        });
    });
}
fn toolbar(app: &mut AeAppEguiApp, ui: &mut Ui) {
    ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
        let history = ui.ctx().data(|d| d.get_temp::<History>(history_key()));
        let back = app.library_details || history.as_ref().is_some_and(|h| h.index > 0);
        let forward = history.as_ref().is_some_and(|h| h.index + 1 < h.pages.len());
        for (is_forward, enabled) in [(false, back), (true, forward)] {
            let (rect, response) = ui.allocate_exact_size(vec2(32.0, 32.0), egui::Sense::click());
            if response.hovered() && enabled {
                ui.painter().rect_filled(rect, 4.0, SURFACE_HOVER);
            }
            let c = rect.center();
            let dir = if is_forward { 1.0 } else { -1.0 };
            let color = if enabled { TEXT } else { BORDER };
            ui.painter().line_segment(
                [c + vec2(-dir * 3.0, -5.0), c + vec2(dir * 3.0, 0.0)],
                Stroke::new(1.8, color),
            );
            ui.painter().line_segment(
                [c + vec2(dir * 3.0, 0.0), c + vec2(-dir * 3.0, 5.0)],
                Stroke::new(1.8, color),
            );
            if response.clicked() && enabled {
                change_history(app, ui.ctx(), is_forward);
            }
        }
        ui.add_space(12.0);
        ui.scope(|ui| {
            ui.visuals_mut().override_text_color = Some(Color32::from_rgb(40, 43, 53));
            ui.visuals_mut().extreme_bg_color = Color32::WHITE;
            ui.visuals_mut().widgets.inactive.bg_stroke = Stroke::NONE;
            let width = (ui.available_width() - 16.0).min(340.0).max(150.0);
            let response = ui.add_sized(
                vec2(width, 34.0),
                egui::TextEdit::singleline(&mut app.game_view_bar.search_buffer)
                    .hint_text("Search games and add-ons")
                    .margin(vec2(13.0, 8.0)),
            );
            if response.changed() {
                navigate(app, PageType::Games, ui.ctx());
            }
        });
    });
}
fn social(app: &mut AeAppEguiApp, ui: &mut Ui) {
    let all = ui.available_rect_before_wrap();
    let profile = Rect::from_min_size(all.min + vec2(16.0, 17.0), vec2(40.0, 40.0));
    avatar(
        ui,
        profile,
        &app.user_name,
        if app.is_preview() {
            None
        } else if app.backend_state == BackendStallState::BingChilling {
            Some(GREEN)
        } else {
            None
        },
    );
    let name: String = app.user_name.chars().take(12).collect();
    let menu = Rect::from_min_size(all.min + vec2(61.0, 17.0), vec2(all.width() - 69.0, 35.0));
    ui.allocate_ui_at_rect(menu, |ui| {
        ui.spacing_mut().button_padding = vec2(7.0, 6.0);
        ui.menu_button(format!("{}  v", name), |ui| {
            if ui.button("My profile").clicked() {
                ui.ctx().data_mut(|d| d.insert_temp(profile_key(), true));
                ui.close_menu();
            }
            if ui.button("Settings").clicked() {
                navigate(app, PageType::Settings, ui.ctx());
                ui.close_menu();
            }
        });
    });
    clipped_text(
        ui,
        Rect::from_min_size(all.min + vec2(65.0, 49.0), vec2(all.width() - 76.0, 17.0)),
        if app.is_preview() {
            "Local demo"
        } else if app.backend_state == BackendStallState::BingChilling {
            "Online"
        } else {
            "Not connected"
        },
        11.0,
        MUTED,
    );
    let top = all.min.y + 113.0;
    let friend_count = if app.is_preview() {
        1
    } else {
        app.friends.len()
    };
    ui.painter().text(
        pos2(all.min.x + 17.0, top),
        Align2::LEFT_TOP,
        format!("Friends ({friend_count})"),
        FontId::proportional(14.0),
        TEXT,
    );
    let friends_rect = Rect::from_min_max(
        pos2(all.min.x + 14.0, top + 36.0),
        all.max - vec2(12.0, 38.0),
    );
    ui.allocate_ui_at_rect(friends_rect, |ui| {
        ui.with_layout(Layout::top_down(Align::Min), |ui| {
            egui::ScrollArea::vertical().id_source("ea-friend-list").auto_shrink(false).show(
                ui,
                |ui| {
                    if app.is_preview() {
                        friend_row(ui, "Demo friend", "Preview data", false);
                    } else if app.friends.is_empty() {
                        ui.label(RichText::new("No friends to show").size(12.0).color(MUTED));
                    } else {
                        for friend in &app.friends {
                            use aeapp::rtm::client::BasicPresence;
                            let online = matches!(
                                friend.online,
                                BasicPresence::Online | BasicPresence::Away | BasicPresence::Dnd
                            );
                            let status = friend.game.as_deref().unwrap_or(match friend.online {
                                BasicPresence::Online => "Online",
                                BasicPresence::Away => "Away",
                                BasicPresence::Dnd => "Busy",
                                _ => "Offline",
                            });
                            friend_row(ui, &friend.name, status, online);
                        }
                    }
                },
            );
        });
    });
    clipped_text(
        ui,
        Rect::from_min_size(
            pos2(all.min.x + 17.0, all.max.y - 25.0),
            vec2(all.width() - 29.0, 18.0),
        ),
        "EA friends · read only",
        10.0,
        MUTED,
    );
}
fn friend_row(ui: &mut Ui, name: &str, status: &str, online: bool) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 60.0), egui::Sense::hover());
    avatar(
        ui,
        Rect::from_min_size(rect.min + vec2(2.0, 5.0), vec2(34.0, 34.0)),
        name,
        if online { Some(GREEN) } else { None },
    );
    clipped_text(
        ui,
        Rect::from_min_size(rect.min + vec2(47.0, 7.0), vec2(rect.width() - 48.0, 20.0)),
        name,
        13.0,
        TEXT,
    );
    clipped_text(
        ui,
        Rect::from_min_size(rect.min + vec2(47.0, 29.0), vec2(rect.width() - 48.0, 17.0)),
        status,
        11.0,
        if online { GREEN } else { MUTED },
    );
}
pub fn show(app: &mut AeAppEguiApp, ui: &mut Ui) {
    record_page(app, ui.ctx());
    let all = ui.available_rect_before_wrap();
    let title = Rect::from_min_size(all.min, vec2(all.width(), 40.0));
    titlebar(app, ui, title);
    let nav_width = if all.width() >= 1700.0 { 256.0 } else { 224.0 };
    let social_width = if all.width() >= 1700.0 { 256.0 } else { 210.0 };
    let sidebar = Rect::from_min_max(
        all.min + vec2(0.0, 40.0),
        pos2(all.min.x + nav_width, all.max.y),
    );
    let friends = Rect::from_min_max(pos2(all.max.x - social_width, all.min.y + 40.0), all.max);
    ui.painter().rect_filled(sidebar, 0.0, SIDEBAR);
    ui.painter().rect_filled(friends, 0.0, SIDEBAR);
    ui.allocate_ui_at_rect(sidebar, |ui| {
        ui.set_clip_rect(sidebar);
        ui.with_layout(Layout::top_down(Align::Min), |ui| navigation(app, ui));
    });
    ui.allocate_ui_at_rect(friends, |ui| {
        ui.set_clip_rect(friends);
        social(app, ui);
    });
    let center = Rect::from_min_max(
        pos2(sidebar.max.x + 32.0, all.min.y + 58.0),
        pos2(friends.min.x - 32.0, all.max.y - 16.0),
    );
    let top = Rect::from_min_size(center.min, vec2(center.width(), 40.0));
    ui.allocate_ui_at_rect(top, |ui| toolbar(app, ui));
    let mut content = Rect::from_min_max(center.min + vec2(0.0, 66.0), center.max);
    if app.is_preview() {
        let banner = Rect::from_min_size(content.min, vec2(content.width(), 28.0));
        ui.painter().rect_filled(banner, 4.0, Color32::from_rgb(49, 42, 58));
        clipped_text(
            ui,
            banner.shrink2(vec2(10.0, 6.0)),
            "LOCAL DEMO · Sample library and friend. EA game actions are disabled.",
            11.0,
            Color32::from_rgb(222, 186, 194),
        );
        content.min.y += 44.0;
    }
    ui.allocate_ui_at_rect(content, |ui| {
        ui.set_clip_rect(content);
        ui.with_layout(Layout::top_down(Align::Min), |ui| {
            if app.page_view == PageType::Settings {
                settings_view(app, ui);
                return;
            }
            match app.backend_state {
                BackendStallState::BingChilling => match app.page_view {
                    PageType::Games => games_view(app, ui),
                    PageType::Home => home(app, ui),
                    PageType::Downloads => {
                        ui.heading("Downloads");
                        ui.label(RichText::new("Keep track of your games.").color(MUTED));
                        ui.add_space(26.0);
                        if app.installing_now.is_none() && app.install_queue.is_empty() {
                            empty(
                                ui,
                                "You're all caught up",
                                "Game downloads and updates will appear here.",
                            );
                        } else {
                            downloads_view(app, ui);
                        }
                    }
                    PageType::Debug => crate::views::debug_view::debug_view(app, ui),
                    _ => {}
                },
                BackendStallState::UserNeedsToLogIn => login(app, ui),
                BackendStallState::UserNeedsToInstallService => {
                    ui.heading("Finish setting up AeApp");
                    ui.label("The platform service is required before connecting your account.");
                    if primary_button(ui, "Set up service").clicked() {
                        let _ = app
                            .backend
                            .backend_commander
                            .send(crate::bridge_thread::AeAppLibRequest::StartService);
                        app.backend_state = BackendStallState::Starting;
                    }
                }
                _ => {
                    ui.vertical_centered(|ui| {
                        ui.add_space(100.0);
                        ui.spinner();
                        ui.add_space(14.0);
                        ui.heading(if app.backend_state == BackendStallState::LoggingIn {
                            "Connecting to your EA account…"
                        } else {
                            "Getting things ready…"
                        });
                    });
                }
            }
        });
    });
    let mut profile_open = ui.ctx().data(|d| d.get_temp::<bool>(profile_key())).unwrap_or(false);
    egui::Window::new("My profile")
        .open(&mut profile_open)
        .collapsible(false)
        .resizable(false)
        .default_width(320.0)
        .show(ui.ctx(), |ui| {
            ui.heading(&app.user_name);
            ui.label(if app.is_preview() {
                "Local demo profile"
            } else if app.backend_state == BackendStallState::BingChilling {
                "Connected EA profile"
            } else {
                "No EA account connected"
            });
            ui.separator();
            ui.label(format!(
                "{} {}",
                app.games.len(),
                if app.is_preview() {
                    "sample games"
                } else {
                    "games in your library"
                }
            ));
            ui.label(format!(
                "{} installed",
                app.games.values().filter(|g| g.installed).count()
            ));
            if app.is_preview() {
                ui.small("No EA account is connected.");
            }
        });
    ui.ctx().data_mut(|d| d.insert_temp(profile_key(), profile_open));
}

fn login(app: &mut AeAppEguiApp, ui: &mut Ui) {
    ui.add_space(55.0);
    pill(ui, "NATIVE LINUX CLIENT", GREEN);
    ui.add_space(22.0);
    ui.label(RichText::new("Your games.\nA new home on Linux.").size(48.0).strong());
    ui.add_space(17.0);
    ui.label(RichText::new("Connect your EA account to browse your owned games,\ndownload them, and launch supported titles with Proton.").size(17.0).color(MUTED));
    ui.add_space(25.0);
    if primary_button(ui, "Sign in with EA").clicked() {
        let _ = app
            .backend
            .backend_commander
            .send(crate::bridge_thread::AeAppLibRequest::LoginRequestOauth);
        app.backend_state = BackendStallState::LoggingIn;
    }
    ui.add_space(17.0);
    ui.label(RichText::new("Secure sign-in opens in your browser.").size(12.0).color(MUTED));
    ui.add_space(40.0);
    ui.separator();
    ui.add_space(16.0);
    ui.label(
        RichText::new("An independent Linux interface. Compatibility varies by game.")
            .size(12.0)
            .color(MUTED),
    );
}

pub fn empty(ui: &mut Ui, title: &str, subtitle: &str) {
    egui::Frame::none().fill(SURFACE).rounding(12.0).inner_margin(35.0).show(ui, |ui| {
        ui.set_min_width((ui.available_width() - 1.0).max(0.0));
        ui.add_space(30.0);
        ui.label(RichText::new(title).size(24.0).strong());
        ui.add_space(9.0);
        ui.label(RichText::new(subtitle).color(MUTED));
        ui.add_space(30.0);
    });
}

fn home(app: &mut AeAppEguiApp, ui: &mut Ui) {
    let mut games: Vec<_> = app.games.values().cloned().collect();
    games.sort_by(|a, b| a.name.cmp(&b.name));
    let featured = app
        .games
        .get(&app.game_sel)
        .cloned()
        .or_else(|| games.iter().find(|game| game.installed).cloned())
        .or_else(|| games.first().cloned());
    let Some(featured) = featured else {
        ui.label(RichText::new("Home").size(34.0).strong());
        ui.add_space(24.0);
        empty(
            ui,
            "Your library is waiting",
            "Your owned EA games will appear here when your library has loaded.",
        );
        return;
    };
    egui::ScrollArea::vertical()
        .id_source("ea-home-content")
        .auto_shrink(false)
        .show(ui, |ui| {
            let (hero, _) =
                ui.allocate_exact_size(vec2(ui.available_width(), 300.0), egui::Sense::hover());
            crate::views::game_view::artwork(app, ui, hero, &featured);
            let mut shade = egui::Mesh::default();
            shade.colored_vertex(hero.left_top(), Color32::from_black_alpha(205));
            shade.colored_vertex(hero.right_top(), Color32::from_black_alpha(10));
            shade.colored_vertex(hero.right_bottom(), Color32::from_black_alpha(65));
            shade.colored_vertex(hero.left_bottom(), Color32::from_black_alpha(235));
            shade.add_triangle(0, 1, 2);
            shade.add_triangle(0, 2, 3);
            ui.painter().with_clip_rect(hero).add(egui::Shape::mesh(shade));
            clipped_text(
                ui,
                Rect::from_min_size(hero.min + vec2(32.0, 35.0), vec2(hero.width() - 64.0, 22.0)),
                if app.is_preview() {
                    "FROM THE DEMO LIBRARY"
                } else {
                    "FROM YOUR LIBRARY"
                },
                11.0,
                Color32::from_rgb(214, 218, 231),
            );
            let mut title = egui::text::LayoutJob::simple_singleline(
                featured.name.clone(),
                FontId::proportional(34.0),
                Color32::WHITE,
            );
            title.wrap.max_width = (hero.width() * 0.67).max(220.0);
            title.wrap.max_rows = 2;
            title.wrap.break_anywhere = false;
            let galley = ui.fonts(|fonts| fonts.layout_job(title));
            ui.painter().with_clip_rect(hero).galley(
                hero.min + vec2(32.0, 83.0),
                galley,
                Color32::WHITE,
            );
            clipped_text(
                ui,
                Rect::from_min_size(
                    hero.min + vec2(32.0, 180.0),
                    vec2(hero.width() - 64.0, 20.0),
                ),
                if app.is_preview() {
                    "Explore the client with local sample data."
                } else {
                    "Your next game starts here."
                },
                14.0,
                Color32::from_rgb(214, 218, 231),
            );
            let button =
                Rect::from_min_size(hero.left_bottom() + vec2(32.0, -77.0), vec2(145.0, 45.0));
            if ui
                .put(
                    button,
                    egui::Button::new(RichText::new("View game").color(Color32::WHITE).strong())
                        .fill(ACCENT_COLOR)
                        .stroke(Stroke::NONE)
                        .rounding(8.0),
                )
                .clicked()
            {
                navigate(app, PageType::Games, ui.ctx());
                app.game_sel = featured.slug.clone();
                app.library_details = true;
            }
            ui.add_space(25.0);
            let (heading, _) =
                ui.allocate_exact_size(vec2(ui.available_width(), 41.0), egui::Sense::hover());
            clipped_text(
                ui,
                Rect::from_min_size(heading.min, vec2(heading.width() - 145.0, 32.0)),
                if app.is_preview() {
                    "From the demo library"
                } else {
                    "From your library"
                },
                23.0,
                TEXT,
            );
            let library_button =
                Rect::from_min_size(heading.right_top() - vec2(137.0, 0.0), vec2(137.0, 34.0));
            if ui
                .put(
                    library_button,
                    egui::Button::new(format!("View all ({})", games.len())),
                )
                .clicked()
            {
                navigate(app, PageType::Games, ui.ctx());
            }
            ui.add_space(10.0);
            let tiles: Vec<_> =
                games.iter().filter(|game| game.slug != featured.slug).take(3).cloned().collect();
            if !tiles.is_empty() {
                let gap = 20.0;
                let width = (ui.available_width() - 2.0 * gap) / 3.0;
                let (row, _) =
                    ui.allocate_exact_size(vec2(ui.available_width(), 205.0), egui::Sense::hover());
                for (index, game) in tiles.iter().enumerate() {
                    let card = Rect::from_min_size(
                        row.min + vec2(index as f32 * (width + gap), 0.0),
                        vec2(width, 198.0),
                    );
                    let image = Rect::from_min_size(card.min, vec2(width, 137.0));
                    crate::views::game_view::artwork(app, ui, image, game);
                    let response = ui.interact(
                        card,
                        ui.id().with(("home-library-card", &game.slug)),
                        egui::Sense::click(),
                    );
                    if response.hovered() {
                        ui.painter().rect_stroke(
                            image.expand(2.0),
                            2.0,
                            Stroke::new(2.0, ACCENT_COLOR),
                        );
                    }
                    clipped_text(
                        ui,
                        Rect::from_min_size(
                            image.left_bottom() + vec2(0.0, 12.0),
                            vec2(width, 23.0),
                        ),
                        &game.name,
                        15.0,
                        TEXT,
                    );
                    clipped_text(
                        ui,
                        Rect::from_min_size(
                            image.left_bottom() + vec2(0.0, 37.0),
                            vec2(width, 18.0),
                        ),
                        if app.is_preview() {
                            "Demo title"
                        } else if game.installed {
                            "Installed"
                        } else {
                            "In your library"
                        },
                        11.0,
                        MUTED,
                    );
                    if response.clicked() {
                        navigate(app, PageType::Games, ui.ctx());
                        app.game_sel = game.slug.clone();
                        app.library_details = true;
                    }
                }
            }
            ui.add_space(12.0);
            ui.label(
                RichText::new(if app.is_preview() {
                    "Sample titles and geometric artwork. No EA account is connected."
                } else {
                    "Proton and umu support varies by game, including anti-cheat compatibility."
                })
                .size(11.0)
                .color(MUTED),
            );
        });
}
