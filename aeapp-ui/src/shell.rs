use crate::{
    views::{downloads_view::downloads_view, game_view::games_view, settings_view::settings_view},
    AeAppEguiApp, BackendStallState, PageType, ACCENT_COLOR,
};
use egui::{
    pos2, vec2, Align, Align2, Color32, FontId, Layout, Margin, Rect, RichText, Rounding, Stroke,
    Ui,
};

pub const BACKGROUND: Color32 = Color32::from_rgb(30, 30, 48);
pub const SIDEBAR: Color32 = Color32::from_rgb(24, 24, 38);
pub const SURFACE: Color32 = Color32::from_rgb(39, 39, 56);
pub const SURFACE_HOVER: Color32 = Color32::from_rgb(52, 54, 71);
pub const BORDER: Color32 = Color32::from_rgb(52, 54, 71);
pub const TEXT: Color32 = Color32::from_rgb(244, 245, 249);
pub const MUTED: Color32 = Color32::from_rgb(144, 142, 151);
pub const GREEN: Color32 = Color32::from_rgb(0, 255, 132);

pub fn bold(size: f32) -> FontId {
    FontId::new(size, egui::FontFamily::Name("ea_bold".into()))
}
pub fn display(size: f32) -> FontId {
    FontId::new(size, egui::FontFamily::Name("ea_display_bold".into()))
}

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
#[derive(Clone)]
struct LocalIcon(Option<std::sync::Arc<[u8]>>);
pub fn original_icon(ui: &Ui, rect: Rect, name: &str, color: Color32) -> bool {
    let key = egui::Id::new(("aeapp-original-icon", name));
    let loaded = ui.ctx().data(|data| data.get_temp::<LocalIcon>(key)).unwrap_or_else(|| {
        let bytes = crate::local_asset_directory()
            .and_then(|directory| {
                std::fs::read_to_string(directory.join("icons").join(format!("{name}.svg"))).ok()
            })
            .map(|svg| {
                std::sync::Arc::<[u8]>::from(
                    svg.replacen("<svg ", "<svg fill=\"#ffffff\" ", 1).into_bytes(),
                )
            });
        let result = LocalIcon(bytes);
        ui.ctx().data_mut(|data| data.insert_temp(key, result.clone()));
        result
    });
    if let Some(bytes) = loaded.0 {
        egui::Image::from_bytes(format!("bytes://aeapp-original/{name}.svg"), bytes)
            .tint(color)
            .fit_to_exact_size(rect.size())
            .paint_at(ui, rect);
        true
    } else {
        false
    }
}
fn navigation_icon(ui: &Ui, center: egui::Pos2, kind: usize, color: Color32) {
    let name = match kind {
        0 => "HOME",
        1 => "BROWSE",
        2 => "COLLECTION",
        3 => "EA_PLAY_MEDALLION",
        4 => "DOWNLOAD_QUEUE",
        _ => "Settings",
    };
    if original_icon(
        ui,
        Rect::from_center_size(center, vec2(24.0, 24.0)),
        name,
        color,
    ) {
        return;
    }
    let painter = ui.painter();
    let line = Stroke::new(1.7, color);
    match kind {
        0 => {
            painter.rect_stroke(Rect::from_center_size(center, vec2(14.0, 14.0)), 2.0, line);
            for x in [-11.0, 11.0] {
                painter.line_segment([center + vec2(x, -6.0), center + vec2(x, 6.0)], line);
            }
            painter.line_segment([center + vec2(-3.0, 0.0), center + vec2(3.0, 0.0)], line);
            painter.line_segment([center + vec2(0.0, -3.0), center + vec2(0.0, 3.0)], line);
        }
        1 => {
            for x in [-6.0, 3.0] {
                for y in [-6.0, 3.0] {
                    painter.rect_stroke(
                        Rect::from_min_size(center + vec2(x, y), vec2(5.0, 5.0)),
                        1.0,
                        line,
                    );
                }
            }
        }
        2 => {
            painter.rect_stroke(
                Rect::from_min_size(center + vec2(-7.0, -3.0), vec2(14.0, 15.0)),
                2.0,
                line,
            );
            painter.line_segment([center + vec2(-5.0, -7.0), center + vec2(5.0, -7.0)], line);
            painter.line_segment(
                [center + vec2(-3.0, -11.0), center + vec2(3.0, -11.0)],
                line,
            );
        }
        3 => {
            painter.circle_filled(center, 10.0, color);
            painter.text(center, Align2::CENTER_CENTER, "EA", bold(8.0), SIDEBAR);
        }
        _ => {
            painter.circle_stroke(center, 8.0, line);
        }
    }
}
pub fn account_avatar(app: &AeAppEguiApp, ui: &Ui, rect: Rect) {
    if !app.is_preview() {
        if let Some(texture) =
            app.img_cache.get(crate::ui_image::UIImageType::Avatar(app.user_id.clone()))
        {
            egui::Image::new((texture.id(), texture.size_vec2()))
                .rounding(rect.width() / 4.0)
                .paint_at(ui, rect);
            return;
        }
    }
    avatar(ui, rect, "", None);
}
fn avatar(ui: &Ui, rect: Rect, _name: &str, status: Option<Color32>) {
    let p = ui.painter();
    let size = rect.width();
    let point = |x, y| rect.min + vec2(size * x, size * y);
    p.rect_filled(rect, size / 4.0, Color32::from_rgb(99, 97, 111));
    for x in [0.28, 0.72] {
        p.circle_filled(point(x, 0.46), size * 0.065, Color32::WHITE);
    }
    p.add(egui::epaint::CubicBezierShape::from_points_stroke(
        [
            point(0.34, 0.66),
            point(0.40, 0.76),
            point(0.60, 0.76),
            point(0.66, 0.66),
        ],
        false,
        Color32::TRANSPARENT,
        Stroke::new(size * 0.06, Color32::WHITE),
    ));
    if let Some(color) = status {
        status_dot(ui, rect, color);
    }
}
fn status_dot(ui: &Ui, rect: Rect, color: Color32) {
    let radius = rect.width() / 8.0;
    let center = rect.right_bottom() - vec2(3.0, 3.0);
    ui.painter().circle_filled(center, radius + 2.0, SIDEBAR);
    ui.painter().circle_filled(center, radius, color);
    if color == GREEN {
        ui.painter().line_segment(
            [
                center + vec2(-radius * 0.5, 0.0),
                center + vec2(-radius * 0.1, radius * 0.4),
            ],
            Stroke::new(1.2, SIDEBAR),
        );
        ui.painter().line_segment(
            [
                center + vec2(-radius * 0.1, radius * 0.4),
                center + vec2(radius * 0.5, -radius * 0.4),
            ],
            Stroke::new(1.2, SIDEBAR),
        );
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
        bold(18.0),
        Color32::WHITE,
    );
    let menu_rect = Rect::from_min_size(rect.min + vec2(4.0, 2.0), vec2(66.0, 36.0));
    ui.allocate_ui_at_rect(menu_rect, |ui| {
        ui.spacing_mut().button_padding = vec2(12.0, 6.0);
        ui.visuals_mut().widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
        ui.visuals_mut().widgets.inactive.bg_fill = Color32::TRANSPARENT;
        ui.visuals_mut().widgets.inactive.bg_stroke = Stroke::NONE;
        let response = ui.menu_button("     ", |ui| {
            ui.set_min_width(180.0);
            if app.is_preview() {
                ui.label("LOCAL DEMO · EA actions disabled");
                ui.separator();
            }
            if ui.button("Downloads").clicked() {
                navigate(app, PageType::Downloads, ui.ctx());
                ui.close_menu();
            }
            if ui.button("Settings").clicked() {
                navigate(app, PageType::Settings, ui.ctx());
                ui.close_menu();
            }
            ui.separator();
            ui.label(egui::RichText::new("Unofficial Linux client").size(11.0).color(MUTED));
            ui.separator();
            if ui.button("Exit").clicked() {
                app.force_quit = true;
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });
        let c = response.response.rect.center();
        for offset in [-4.0, 0.0, 4.0] {
            ui.painter().line_segment(
                [c + vec2(-6.0, offset), c + vec2(6.0, offset)],
                Stroke::new(1.0, MUTED),
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
        ui.allocate_exact_size(vec2(ui.available_width(), 60.0), egui::Sense::click());
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
    navigation_icon(ui, pos2(rect.left() + 36.0, rect.center().y), kind, TEXT);
    ui.painter().text(
        pos2(rect.left() + 72.0, rect.center().y),
        Align2::LEFT_CENTER,
        text,
        FontId::proportional(14.0),
        TEXT,
    );
    response
}
fn navigation(app: &mut AeAppEguiApp, ui: &mut Ui) {
    ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
    ui.add_space(12.0);
    if navigation_row(ui, "Home", 0, app.page_view == PageType::Home).clicked() {
        navigate(app, PageType::Home, ui.ctx());
    }
    if navigation_row(ui, "Browse", 1, app.page_view == PageType::Store).clicked() {
        navigate(app, PageType::Store, ui.ctx());
    }
    if navigation_row(ui, "Library", 2, app.page_view == PageType::Games).clicked() {
        navigate(app, PageType::Games, ui.ctx());
    }
    if navigation_row(ui, "EA Play", 3, app.page_view == PageType::EaPlay).clicked() {
        navigate(app, PageType::EaPlay, ui.ctx());
    }
    ui.add_space(30.0);
    ui.horizontal(|ui| {
        ui.add_space(12.0);
        ui.label(RichText::new("Installed games").size(13.0).color(MUTED));
    });
    ui.add_space(12.0);
    let mut games: Vec<_> = app.games.values().filter(|g| g.installed).cloned().collect();
    games.sort_by(|a, b| a.name.cmp(&b.name));
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
            for game in games {
                let (rect, response) =
                    ui.allocate_exact_size(vec2(ui.available_width(), 64.0), egui::Sense::click());
                if response.hovered() {
                    ui.painter().rect_filled(rect, 0.0, SURFACE);
                }
                let thumb = Rect::from_min_size(rect.min + vec2(12.0, 8.0), vec2(48.0, 48.0));
                crate::views::game_view::cover_artwork(app, ui, thumb, &game);
                let mut job = egui::text::LayoutJob::simple_singleline(
                    game.name.clone(),
                    FontId::proportional(14.0),
                    TEXT,
                );
                job.wrap.max_width = rect.width() - 84.0;
                job.wrap.max_rows = 2;
                job.wrap.break_anywhere = false;
                job.wrap.overflow_character = Some('…');
                let galley = ui.fonts(|fonts| fonts.layout_job(job));
                ui.painter().galley(rect.min + vec2(72.0, 14.0), galley, TEXT);
                if response.clicked() {
                    navigate(app, PageType::Games, ui.ctx());
                    app.game_sel = game.slug;
                    app.library_details = true;
                }
            }
        });
}

fn toolbar(app: &mut AeAppEguiApp, ui: &mut Ui) {
    let all = ui.max_rect();
    let history = ui.ctx().data(|data| data.get_temp::<History>(history_key()));
    let back = app.library_details || history.as_ref().is_some_and(|h| h.index > 0);
    let forward = history.as_ref().is_some_and(|h| h.index + 1 < h.pages.len());
    for (index, enabled) in [back, forward].into_iter().enumerate() {
        let rect = Rect::from_min_size(all.min + vec2(index as f32 * 44.0, 0.0), vec2(32.0, 32.0));
        let response = ui.interact(rect, ui.id().with(("history", index)), egui::Sense::click());
        ui.painter().rect_filled(rect, 8.0, if enabled { BACKGROUND } else { BORDER });
        if enabled {
            ui.painter().rect_stroke(rect, 8.0, Stroke::new(2.0, Color32::from_rgb(99, 97, 111)));
        }
        let icon = if index == 0 {
            "CHEVRON_LEFT"
        } else {
            "CHEVRON_RIGHT"
        };
        if !original_icon(
            ui,
            rect.shrink(8.0),
            icon,
            if enabled { TEXT } else { MUTED },
        ) {
            ui.painter().text(
                rect.center(),
                Align2::CENTER_CENTER,
                if index == 0 { "‹" } else { "›" },
                FontId::proportional(21.0),
                TEXT,
            );
        }
        if response.clicked() && enabled {
            change_history(app, ui.ctx(), index == 1);
        }
    }
    let search = Rect::from_min_size(
        all.min + vec2(92.0, 0.0),
        vec2(320.0_f32.min((all.width() - 300.0).max(180.0)), 32.0),
    );
    ui.painter().rect_filled(search, 6.0, Color32::WHITE);
    ui.painter().rect_stroke(
        search,
        6.0,
        Stroke::new(2.0, Color32::from_rgb(115, 113, 124)),
    );
    original_icon(
        ui,
        Rect::from_min_size(search.min + vec2(12.0, 8.0), vec2(16.0, 16.0)),
        "Search",
        MUTED,
    );
    ui.scope(|ui| {
        ui.visuals_mut().override_text_color = Some(Color32::from_rgb(38, 38, 46));
        let input = Rect::from_min_max(search.min + vec2(36.0, 0.0), search.max - vec2(4.0, 0.0));
        if ui
            .put(
                input,
                egui::TextEdit::singleline(&mut app.game_view_bar.search_buffer)
                    .font(FontId::proportional(14.0))
                    .frame(false)
                    .margin(vec2(4.0, 7.0))
                    .hint_text("Search games and add-ons"),
            )
            .changed()
        {
            navigate(app, PageType::Games, ui.ctx());
        }
    });
    if all.width() > 680.0 {
        for (index, name) in ["Heart", "Cart", "Notification"].into_iter().enumerate() {
            let x = all.right() - 182.0 + index as f32 * 60.0 + if index == 2 { 14.0 } else { 0.0 };
            let rect = Rect::from_min_size(pos2(x, all.top()), vec2(48.0, 32.0));
            ui.painter().rect_filled(rect, 8.0, SIDEBAR);
            ui.painter().rect_stroke(rect, 8.0, Stroke::new(2.0, Color32::from_rgb(99, 97, 111)));
            if !original_icon(
                ui,
                Rect::from_center_size(rect.center(), vec2(16.0, 16.0)),
                name,
                TEXT,
            ) {
                let center = rect.center();
                let line = Stroke::new(1.5, TEXT);
                ui.painter()
                    .line_segment([center + vec2(-7.0, -4.0), center + vec2(-3.0, -4.0)], line);
                ui.painter()
                    .line_segment([center + vec2(-3.0, -4.0), center + vec2(-1.0, 3.0)], line);
                ui.painter()
                    .line_segment([center + vec2(-1.0, 3.0), center + vec2(6.0, 3.0)], line);
                ui.painter().circle_filled(center + vec2(0.0, 6.0), 1.5, TEXT);
                ui.painter().circle_filled(center + vec2(5.0, 6.0), 1.5, TEXT);
            }
            ui.interact(rect, ui.id().with(name), egui::Sense::hover()).on_hover_text(
                match index {
                    0 => "Wishlist is not connected yet.",
                    1 => "Purchases open on EA's website.",
                    _ => "No notifications available.",
                },
            );
        }
        ui.painter().line_segment(
            [
                pos2(all.right() - 62.0, all.top() + 4.0),
                pos2(all.right() - 62.0, all.top() + 28.0),
            ],
            Stroke::new(2.0, BORDER),
        );
    }
}

fn social(app: &mut AeAppEguiApp, ui: &mut Ui) {
    let all = ui.max_rect();
    let collapsed = all.width() < 100.0;
    let profile = Rect::from_min_size(all.min + vec2(12.0, 18.0), vec2(48.0, 48.0));
    account_avatar(app, ui, profile);
    if app.backend_state == BackendStallState::BingChilling {
        status_dot(ui, profile, GREEN);
    }
    let profile_hit = Rect::from_min_size(all.min + vec2(0.0, 12.0), vec2(all.width(), 60.0));
    let response = ui.interact(
        profile_hit,
        ui.id().with("own-profile-menu"),
        egui::Sense::click(),
    );
    if !collapsed {
        ui.painter().text(
            all.min + vec2(72.0, 24.0),
            Align2::LEFT_TOP,
            &app.user_name,
            bold(14.0),
            TEXT,
        );
        ui.painter().text(
            all.min + vec2(72.0, 45.0),
            Align2::LEFT_TOP,
            if app.is_preview() {
                "Local preview"
            } else if app.backend_state == BackendStallState::BingChilling {
                "Online"
            } else {
                "Not connected"
            },
            FontId::proportional(11.0),
            MUTED,
        );
        original_icon(
            ui,
            Rect::from_min_size(all.min + vec2(all.width() - 28.0, 34.0), vec2(16.0, 16.0)),
            "CHEVRON_DOWN",
            TEXT,
        );
    }
    let menu_key = egui::Id::new("ea-profile-menu-open");
    let mut open = ui.ctx().data(|data| data.get_temp::<bool>(menu_key)).unwrap_or(false);
    if response.clicked() {
        open = !open;
    }
    if open {
        let anchor = pos2(all.left(), all.top() + 80.0);
        let area = egui::Area::new(egui::Id::new("ea-profile-menu"))
            .order(egui::Order::Foreground)
            .fixed_pos(anchor)
            .show(ui.ctx(), |ui| {
                egui::Frame::none()
                    .fill(Color32::from_rgb(20, 20, 32))
                    .stroke(Stroke::new(2.0, BORDER))
                    .rounding(10.0)
                    .inner_margin(8.0)
                    .show(ui, |ui| {
                        ui.set_width(140.0);
                        ui.spacing_mut().item_spacing.y = 0.0;
                        for label in [
                            "Online",
                            "Away",
                            "Invisible",
                            "My profile",
                            "Take the tour",
                            "Redeem code",
                            "Settings",
                            "Log out",
                        ] {
                            if matches!(label, "My profile" | "Settings") {
                                ui.add_space(8.0);
                                ui.separator();
                                ui.add_space(8.0);
                            }
                            let enabled = matches!(label, "My profile" | "Settings");
                            let (row, hit) =
                                ui.allocate_exact_size(vec2(140.0, 40.0), egui::Sense::click());
                            if hit.hovered() {
                                ui.painter().rect_filled(row, 4.0, SURFACE_HOVER);
                            }
                            if let Some(color) = match label {
                                "Online" => Some(GREEN),
                                "Away" => Some(Color32::from_rgb(255, 230, 107)),
                                "Invisible" => Some(MUTED),
                                _ => None,
                            } {
                                ui.painter().circle_filled(
                                    pos2(row.left() + 13.0, row.center().y),
                                    8.0,
                                    color,
                                );
                            } else {
                                original_icon(
                                    ui,
                                    Rect::from_center_size(
                                        pos2(row.left() + 13.0, row.center().y),
                                        vec2(20.0, 20.0),
                                    ),
                                    if label == "Settings" {
                                        "Settings"
                                    } else {
                                        "Avatar"
                                    },
                                    TEXT,
                                );
                            }
                            ui.painter().text(
                                pos2(row.left() + 32.0, row.center().y),
                                Align2::LEFT_CENTER,
                                label,
                                FontId::proportional(14.0),
                                TEXT,
                            );
                            if hit.clicked() && enabled {
                                if label == "Settings" {
                                    navigate(app, PageType::Settings, ui.ctx());
                                } else {
                                    ui.ctx().data_mut(|data| data.insert_temp(profile_key(), true));
                                }
                                open = false;
                            }
                            if !enabled {
                                hit.on_hover_text("This account action is not connected yet.");
                            }
                        }
                    });
            });
        if ui.input(|input| input.pointer.any_pressed())
            && !response.hovered()
            && !area.response.hovered()
        {
            open = false;
        }
    }
    ui.ctx().data_mut(|data| data.insert_temp(menu_key, open));
    let online = app
        .friends
        .iter()
        .filter(|friend| {
            matches!(
                friend.online,
                aeapp::rtm::client::BasicPresence::Online
                    | aeapp::rtm::client::BasicPresence::Away
                    | aeapp::rtm::client::BasicPresence::Dnd
            )
        })
        .count();
    if !collapsed {
        ui.painter().text(
            all.min + vec2(12.0, 112.0),
            Align2::LEFT_TOP,
            format!(
                "Friends {online}/{}",
                if app.is_preview() {
                    1
                } else {
                    app.friends.len()
                }
            ),
            bold(13.0),
            MUTED,
        );
        for (offset, icon) in [(84.0, "Offline"), (44.0, "FRIENDS")] {
            let button = Rect::from_min_size(
                pos2(all.right() - offset, all.top() + 104.0),
                vec2(32.0, 32.0),
            );
            ui.painter().rect_stroke(
                button,
                8.0,
                Stroke::new(2.0, Color32::from_rgb(99, 97, 111)),
            );
            original_icon(ui, button.shrink(7.0), icon, TEXT);
            ui.interact(button, ui.id().with(icon), egui::Sense::hover())
                .on_hover_text("Friend management is not connected yet.");
        }
    }
    let friends = Rect::from_min_max(all.min + vec2(12.0, 156.0), all.max - vec2(12.0, 72.0));
    ui.allocate_ui_at_rect(friends, |ui| {
        egui::ScrollArea::vertical().id_source("ea-friend-list").auto_shrink(false).show(
            ui,
            |ui| {
                if app.is_preview() {
                    friend_row(app, ui, "", "Demo friend", false, collapsed);
                } else {
                    for friend in &app.friends {
                        let online = matches!(
                            friend.online,
                            aeapp::rtm::client::BasicPresence::Online
                                | aeapp::rtm::client::BasicPresence::Away
                                | aeapp::rtm::client::BasicPresence::Dnd
                        );
                        friend_row(app, ui, &friend.id, &friend.name, online, collapsed);
                    }
                }
            },
        );
    });
    let toggle = Rect::from_min_size(
        pos2(all.right() - 52.0, all.bottom() - 56.0),
        vec2(32.0, 32.0),
    );
    ui.painter().rect_stroke(
        toggle,
        8.0,
        Stroke::new(2.0, Color32::from_rgb(99, 97, 111)),
    );
    ui.painter().text(
        toggle.center(),
        Align2::CENTER_CENTER,
        if collapsed { "⇤" } else { "⇥" },
        FontId::proportional(20.0),
        TEXT,
    );
    if ui
        .interact(
            toggle,
            ui.id().with("collapse-friends"),
            egui::Sense::click(),
        )
        .on_hover_text(if collapsed { "Expand" } else { "Collapse" })
        .clicked()
    {
        ui.ctx()
            .data_mut(|data| data.insert_temp(egui::Id::new("ea-friends-collapsed"), !collapsed));
    }
}
fn friend_row(
    app: &AeAppEguiApp,
    ui: &mut Ui,
    id: &str,
    name: &str,
    online: bool,
    collapsed: bool,
) {
    let (row, _) = ui.allocate_exact_size(vec2(ui.available_width(), 64.0), egui::Sense::hover());
    let picture = Rect::from_min_size(row.min, vec2(48.0, 48.0));
    if let Some(texture) = if app.is_preview() {
        None
    } else {
        app.img_cache.get(crate::ui_image::UIImageType::Avatar(id.to_owned()))
    } {
        egui::Image::new((texture.id(), texture.size_vec2()))
            .rounding(12.0)
            .paint_at(ui, picture);
        status_dot(ui, picture, if online { GREEN } else { MUTED });
    } else {
        avatar(ui, picture, name, Some(if online { GREEN } else { MUTED }));
    }
    if !collapsed {
        clipped_text(
            ui,
            Rect::from_min_size(row.min + vec2(60.0, 15.0), vec2(row.width() - 60.0, 24.0)),
            name,
            14.0,
            TEXT,
        );
    }
}
pub fn show(app: &mut AeAppEguiApp, ui: &mut Ui) {
    record_page(app, ui.ctx());
    let all = ui.available_rect_before_wrap();
    let title = Rect::from_min_size(all.min, vec2(all.width(), 40.0));
    titlebar(app, ui, title);
    let nav_width = 256.0;
    let collapsed = ui
        .ctx()
        .data(|data| data.get_temp::<bool>(egui::Id::new("ea-friends-collapsed")))
        .unwrap_or(all.width() < 1280.0);
    let social_width = if collapsed { 72.0 } else { 256.0 };
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
        pos2(sidebar.max.x + 32.0, all.min.y + 64.0),
        pos2(friends.min.x - 32.0, all.max.y - 16.0),
    );
    if app.page_view == PageType::Home && app.backend_state == BackendStallState::BingChilling {
        home_backdrop(
            ui,
            Rect::from_min_max(
                pos2(sidebar.right(), all.top() + 40.0),
                pos2(friends.left(), all.bottom()),
            ),
        );
    }
    let top = Rect::from_min_size(center.min, vec2(center.width(), 32.0));
    ui.allocate_ui_at_rect(top, |ui| toolbar(app, ui));
    let mut content = Rect::from_min_max(center.min + vec2(0.0, 60.0), center.max);
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
                    PageType::Store => crate::views::catalog_view::browse(app, ui),
                    PageType::EaPlay => crate::views::catalog_view::ea_play(app, ui),
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
    crate::views::profile_view::show(app, ui.ctx(), &mut profile_open);
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

#[derive(Clone)]
struct PromoTexture(Option<egui::TextureHandle>);
fn promotion(ui: &Ui, rect: Rect, filename: &str, rounding: f32) -> bool {
    let id = egui::Id::new(("aeapp-promotion", filename));
    let value = ui.ctx().data(|data| data.get_temp::<PromoTexture>(id)).unwrap_or_else(|| {
        let texture = crate::local_asset_directory()
            .and_then(|directory| std::fs::read(directory.join("promotions").join(filename)).ok())
            .and_then(|bytes| image::load_from_memory(&bytes).ok())
            .map(|image| {
                let image = image.thumbnail(2560, 2560).into_rgba8();
                let pixels = egui::ColorImage::from_rgba_unmultiplied(
                    [image.width() as usize, image.height() as usize],
                    image.as_raw(),
                );
                ui.ctx().load_texture(
                    format!("EA promotion: {filename}"),
                    pixels,
                    egui::TextureOptions::LINEAR,
                )
            });
        let result = PromoTexture(texture);
        ui.ctx().data_mut(|data| data.insert_temp(id, result.clone()));
        result
    });
    if let Some(texture) = value.0 {
        egui::Image::new((texture.id(), texture.size_vec2()))
            .rounding(rounding)
            .paint_at(ui, rect);
        true
    } else {
        false
    }
}
fn horizontal_shade(ui: &Ui, rect: Rect, points: &[(f32, u8)]) {
    for pair in points.windows(2) {
        let mut mesh = egui::Mesh::default();
        let left = rect.left() + rect.width() * pair[0].0;
        let right = rect.left() + rect.width() * pair[1].0;
        let a = Color32::from_rgba_unmultiplied(
            BACKGROUND.r(),
            BACKGROUND.g(),
            BACKGROUND.b(),
            pair[0].1,
        );
        let b = Color32::from_rgba_unmultiplied(
            BACKGROUND.r(),
            BACKGROUND.g(),
            BACKGROUND.b(),
            pair[1].1,
        );
        mesh.colored_vertex(pos2(left, rect.top()), a);
        mesh.colored_vertex(pos2(right, rect.top()), b);
        mesh.colored_vertex(pos2(right, rect.bottom()), b);
        mesh.colored_vertex(pos2(left, rect.bottom()), a);
        mesh.add_triangle(0, 1, 2);
        mesh.add_triangle(0, 2, 3);
        ui.painter().add(egui::Shape::mesh(mesh));
    }
}
fn home_backdrop(ui: &mut Ui, main: Rect) {
    let offset = ui
        .ctx()
        .data(|data| data.get_temp::<f32>(egui::Id::new("ea-home-scroll-y")))
        .unwrap_or(0.0);
    let bounds = Rect::from_min_size(main.min - vec2(0.0, offset), vec2(main.width(), 448.0));
    let mut clipped = ui.child_ui(main, Layout::default(), None);
    clipped.set_clip_rect(main.intersect(Rect::from_min_size(main.min, vec2(main.width(), 448.0))));
    let factor = main.width() / 1408.0;
    let image = Rect::from_min_size(
        bounds.min + vec2(200.0 * factor, 0.0),
        vec2(1200.0 * factor, 675.0 * factor),
    );
    if !promotion(&clipped, image, "apex-blood-pact.jpg", 0.0) {
        ui.painter().rect_filled(bounds, 0.0, SURFACE);
    }
    horizontal_shade(
        &clipped,
        bounds,
        &[(0.0, 255), (0.34, 255), (0.55, 175), (0.72, 15), (1.0, 5)],
    );
    let bottom = Rect::from_min_size(
        bounds.left_bottom() - vec2(0.0, 150.0),
        vec2(bounds.width(), 150.0),
    );
    let mut mesh = egui::Mesh::default();
    for (pos, alpha) in [
        (bottom.left_top(), 0),
        (bottom.right_top(), 0),
        (bottom.right_bottom(), 255),
        (bottom.left_bottom(), 255),
    ] {
        mesh.colored_vertex(
            pos,
            Color32::from_rgba_unmultiplied(BACKGROUND.r(), BACKGROUND.g(), BACKGROUND.b(), alpha),
        );
    }
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    clipped.painter().add(egui::Shape::mesh(mesh));
}
fn home(app: &mut AeAppEguiApp, ui: &mut Ui) {
    let result = egui::ScrollArea::vertical().id_source("ea-home-content").auto_shrink(false).show(
        ui,
        |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            let width = ui.available_width();
            let (hero, _) = ui.allocate_exact_size(vec2(width, 364.0), egui::Sense::hover());
            ui.painter().text(
                hero.min + vec2(0.0, 27.0),
                Align2::LEFT_TOP,
                "Play Apex Legends™: Blood Pact",
                display(36.0),
                TEXT,
            );
            ui.painter().text(
                hero.min + vec2(0.0, 92.0),
                Align2::LEFT_TOP,
                "New Event, now live",
                bold(16.0),
                TEXT,
            );
            ui.painter().text(
                hero.min + vec2(0.0, 133.0),
                Align2::LEFT_TOP,
                "Pick a side and gain werewolf or demonic powers in Wildcard",
                FontId::proportional(16.0),
                TEXT,
            );
            let play = Rect::from_min_size(hero.min + vec2(0.0, 203.0), vec2(150.0, 48.0));
            if ui
                .put(
                    play,
                    egui::Button::new(
                        RichText::new("PLAY NOW").font(bold(16.0)).color(Color32::WHITE),
                    )
                    .fill(Color32::from_rgb(53, 91, 255))
                    .stroke(Stroke::NONE)
                    .rounding(8.0),
                )
                .clicked()
                && !app.is_preview()
            {
                if let Some(game) =
                    app.games.values().find(|game| game.slug.contains("apex")).cloned()
                {
                    app.game_sel = game.slug;
                    app.page_view = PageType::Games;
                    app.library_details = true;
                } else {
                    ui.ctx().open_url(egui::OpenUrl::new_tab(
                        "https://www.ea.com/games/apex-legends/apex-legends",
                    ));
                }
            }
            ui.advance_cursor_after_rect(hero);
            let inset = 32.0;
            let gap = 24.0;
            let card_width = ((width - inset * 2.0 - gap * 2.0) / 3.0).max(100.0);
            let image_height = card_width * 9.0 / 16.0;
            let (row, _) =
                ui.allocate_exact_size(vec2(width, image_height + 71.0), egui::Sense::hover());
            for (index, (image, title, body, url)) in [
                (
                    "battlefield.jpg",
                    "Get the Game of the Year Edition",
                    "Unlock the Ultimate Shortcut Kit, a Battlefield Pro Token, & more",
                    "https://www.ea.com/games/battlefield/battlefield-6",
                ),
                (
                    "madden-most-feared.png",
                    "Be the Most Feared",
                    "Step into the Lab and become a Madden monster",
                    "https://www.ea.com/games/madden-nfl/madden-nfl-27/event/m27-most-feared",
                ),
                (
                    "sims4.jpg",
                    "Download for free & create",
                    "Style custom Sims, design their homes, and play with life",
                    "https://www.ea.com/games/the-sims/the-sims-4",
                ),
            ]
            .into_iter()
            .enumerate()
            {
                let card = Rect::from_min_size(
                    row.min + vec2(inset + index as f32 * (card_width + gap), 0.0),
                    vec2(card_width, image_height + 71.0),
                );
                ui.painter().rect_filled(card, 16.0, SURFACE);
                let art = Rect::from_min_size(card.min, vec2(card_width, image_height));
                if !promotion(ui, art, image, 12.0) {
                    ui.painter().rect_filled(art, 12.0, Color32::from_rgb(40, 43, 61));
                }
                ui.painter().text(
                    card.min + vec2(16.0, image_height + 14.0),
                    Align2::LEFT_TOP,
                    title,
                    bold(13.0),
                    TEXT,
                );
                clipped_text(
                    ui,
                    Rect::from_min_size(
                        card.min + vec2(16.0, image_height + 36.0),
                        vec2(card_width - 32.0, 24.0),
                    ),
                    body,
                    13.0,
                    TEXT,
                );
                if ui
                    .interact(
                        card,
                        ui.id().with(("promotion", index)),
                        egui::Sense::click(),
                    )
                    .clicked()
                    && !app.is_preview()
                {
                    ui.ctx().open_url(egui::OpenUrl::new_tab(url));
                }
            }
            ui.add_space(74.0);
            if !crate::views::catalog_view::home_products(app, ui) {
                ui.label(RichText::new("From your library").font(bold(16.0)));
                ui.add_space(32.0);
                let mut games: Vec<_> = app.games.values().cloned().collect();
                games.sort_by(|a, b| a.name.cmp(&b.name));
                if !games.is_empty() {
                    let columns = ((width + 24.0) / 274.0).floor().max(1.0) as usize;
                    let card_width = (width - (columns - 1) as f32 * 24.0) / columns as f32;
                    let (row, _) = ui
                        .allocate_exact_size(vec2(width, card_width + 40.0), egui::Sense::hover());
                    for (index, game) in games.iter().take(columns).enumerate() {
                        let art = Rect::from_min_size(
                            row.min + vec2(index as f32 * (card_width + 24.0), 0.0),
                            vec2(card_width, card_width),
                        );
                        crate::views::game_view::cover_artwork(app, ui, art, game);
                        ui.painter().text(
                            art.left_bottom() + vec2(0.0, 8.0),
                            Align2::LEFT_TOP,
                            &game.name,
                            bold(12.0),
                            TEXT,
                        );
                        if ui
                            .interact(art, ui.id().with(&game.slug), egui::Sense::click())
                            .clicked()
                        {
                            app.game_sel = game.slug.clone();
                            app.page_view = PageType::Games;
                            app.library_details = true;
                        }
                    }
                }
            }
        },
    );
    ui.ctx().data_mut(|data| {
        data.insert_temp(egui::Id::new("ea-home-scroll-y"), result.state.offset.y)
    });
}
