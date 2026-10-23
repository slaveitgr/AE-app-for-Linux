use crate::{
    bridge_thread::AeAppLibRequest, set_app_modal, shell, AeAppEguiApp, GameDetailsWrapper,
    GameInfo, PopupModal, ACCENT_COLOR,
};
use egui::{pos2, vec2, Align2, Color32, FontId, Rect, RichText, Sense, Stroke, Ui};
use strum_macros::EnumIter;

#[derive(Debug, PartialEq, Default, EnumIter)]
pub enum GameViewBarGenre {
    #[default]
    AllGames,
    Shooters,
    Simulation,
}
#[derive(Debug, PartialEq, Default, EnumIter)]
pub enum GameViewBarPlatform {
    #[default]
    AllPlatforms,
    Windows,
    Mac,
}
pub struct GameViewBar {
    pub genre_filter: GameViewBarGenre,
    pub platform_filter: GameViewBarPlatform,
    pub game_size: f32,
    pub search_buffer: String,
}

fn request_game_action(app: &mut AeAppEguiApp, request: AeAppLibRequest) -> bool {
    if app.backend.backend_commander.send(request).is_err() {
        app.nonfatal_errors.push(crate::bridge_thread::BackendError::ChannelDisconnected);
        return false;
    }
    true
}

fn palette(slug: &str) -> (Color32, Color32) {
    if slug.contains("sims") {
        (
            Color32::from_rgb(54, 136, 193),
            Color32::from_rgb(93, 237, 171),
        )
    } else if slug.contains("titanfall") {
        (
            Color32::from_rgb(146, 88, 53),
            Color32::from_rgb(251, 188, 104),
        )
    } else if slug.contains("speed") {
        (
            Color32::from_rgb(84, 35, 139),
            Color32::from_rgb(255, 83, 163),
        )
    } else if slug.contains("mass-effect") {
        (
            Color32::from_rgb(32, 67, 105),
            Color32::from_rgb(227, 80, 70),
        )
    } else if slug.contains("takes") {
        (
            Color32::from_rgb(136, 79, 54),
            Color32::from_rgb(255, 204, 88),
        )
    } else {
        (
            Color32::from_rgb(85, 95, 100),
            Color32::from_rgb(220, 189, 138),
        )
    }
}

/// Original geometric placeholder art, deliberately marked as demo data by the shell.
fn poster(ui: &Ui, rect: Rect, game: &GameInfo, demo: bool) {
    let (base, accent) = palette(&game.slug);
    let p = ui.painter().with_clip_rect(rect);
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(rect.left_top(), base);
    mesh.colored_vertex(rect.right_top(), base.gamma_multiply(0.65));
    mesh.colored_vertex(rect.right_bottom(), Color32::from_rgb(15, 20, 29));
    mesh.colored_vertex(rect.left_bottom(), base.gamma_multiply(0.25));
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    p.add(egui::Shape::mesh(mesh));
    let at = |x: f32, y: f32| {
        pos2(
            rect.left() + rect.width() * x,
            rect.top() + rect.height() * y,
        )
    };
    let s = rect.width().min(rect.height() * 0.9);
    p.circle_filled(at(0.78, 0.24), s * 0.25, accent.gamma_multiply(0.55));
    p.circle_stroke(
        at(0.78, 0.24),
        s * 0.29,
        Stroke::new(1.0, accent.gamma_multiply(0.4)),
    );
    if game.slug.contains("sims") {
        p.add(egui::Shape::convex_polygon(
            vec![
                at(0.50, 0.22),
                at(0.69, 0.43),
                at(0.50, 0.75),
                at(0.31, 0.43),
            ],
            accent,
            Stroke::NONE,
        ));
        p.add(egui::Shape::convex_polygon(
            vec![at(0.50, 0.22), at(0.50, 0.75), at(0.31, 0.43)],
            Color32::from_rgb(34, 164, 128),
            Stroke::NONE,
        ));
    } else if game.slug.contains("speed") {
        for i in 0..8 {
            let y = 0.29 + i as f32 * 0.07;
            p.line_segment(
                [at(-0.15, y), at(1.1, y - 0.3)],
                Stroke::new(
                    if i % 2 == 0 { 4.0 } else { 1.0 },
                    accent.gamma_multiply(0.5),
                ),
            );
        }
        p.add(egui::Shape::convex_polygon(
            vec![
                at(0.12, 0.64),
                at(0.25, 0.48),
                at(0.65, 0.44),
                at(0.87, 0.63),
                at(0.85, 0.72),
                at(0.13, 0.72),
            ],
            Color32::from_rgb(17, 20, 33),
            Stroke::new(1.0, accent),
        ));
        p.circle_filled(at(0.28, 0.71), s * 0.08, Color32::BLACK);
        p.circle_filled(at(0.72, 0.71), s * 0.08, Color32::BLACK);
    } else {
        p.add(egui::Shape::convex_polygon(
            vec![
                at(-0.1, 0.8),
                at(0.25, 0.39),
                at(0.48, 0.66),
                at(0.69, 0.43),
                at(1.1, 0.83),
            ],
            base.gamma_multiply(0.5),
            Stroke::NONE,
        ));
        p.add(egui::Shape::convex_polygon(
            vec![
                at(-0.1, 0.98),
                at(0.33, 0.64),
                at(0.57, 0.85),
                at(0.85, 0.57),
                at(1.1, 0.95),
            ],
            Color32::from_rgb(17, 24, 33),
            Stroke::NONE,
        ));
        let figure = Color32::from_rgb(16, 23, 30);
        p.circle_filled(at(0.54, 0.43), s * 0.05, figure);
        p.add(egui::Shape::convex_polygon(
            vec![
                at(0.48, 0.49),
                at(0.60, 0.49),
                at(0.66, 0.75),
                at(0.43, 0.75),
            ],
            figure,
            Stroke::NONE,
        ));
        p.line_segment(
            [at(0.50, 0.70), at(0.44, 0.89)],
            Stroke::new(s * 0.045, figure),
        );
        p.line_segment(
            [at(0.58, 0.70), at(0.64, 0.89)],
            Stroke::new(s * 0.045, figure),
        );
        if game.slug.contains("jedi") {
            p.line_segment(
                [at(0.64, 0.66), at(0.88, 0.38)],
                Stroke::new(3.0, Color32::from_rgb(133, 218, 255)),
            );
        }
    }
    if demo && rect.width() / rect.height() < 1.5 {
        p.text(
            at(0.08, 0.07),
            Align2::LEFT_TOP,
            "EA  /  DEMO ART",
            FontId::proportional((s * 0.042).max(9.0)),
            Color32::from_white_alpha(190),
        );
        let title = if game.slug.contains("jedi") {
            "STAR WARS\nJEDI: SURVIVOR"
        } else if game.slug.contains("titanfall") {
            "TITANFALL 2"
        } else if game.slug.contains("sims") {
            "THE SIMS 4"
        } else if game.slug.contains("speed") {
            "NEED FOR SPEED\nHEAT"
        } else if game.slug.contains("mass") {
            "MASS EFFECT\nLEGENDARY EDITION"
        } else {
            "IT TAKES TWO"
        };
        p.text(
            at(0.08, 0.85),
            Align2::LEFT_BOTTOM,
            title,
            FontId::proportional((s * 0.087).max(15.0)),
            Color32::WHITE,
        );
    } else if !demo {
        p.text(
            rect.center(),
            Align2::CENTER_CENTER,
            game.name.chars().next().unwrap_or('G'),
            FontId::proportional(s * 0.35),
            Color32::from_white_alpha(170),
        );
    }
}

pub(crate) fn artwork(app: &AeAppEguiApp, ui: &Ui, rect: Rect, game: &GameInfo) {
    if !app.is_preview() {
        if let Some(hero) = app.img_cache.get(crate::ui_image::UIImageType::Hero(game.slug.clone()))
        {
            let target_aspect = rect.width() / rect.height();
            let source_aspect = hero.aspect_ratio();
            let uv = if source_aspect > target_aspect {
                let w = target_aspect / source_aspect;
                Rect::from_min_max(pos2((1.0 - w) * 0.5, 0.0), pos2((1.0 + w) * 0.5, 1.0))
            } else {
                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0))
            };
            ui.painter().image(hero.id(), rect, uv, Color32::WHITE);
            return;
        }
    }
    poster(ui, rect, game, app.is_preview());
}

pub(crate) fn cover_artwork(app: &AeAppEguiApp, ui: &Ui, rect: Rect, game: &GameInfo) {
    if let Some(cover) = app.img_cache.get(crate::ui_image::UIImageType::Cover(game.slug.clone())) {
        let aspect = cover.aspect_ratio();
        let target_aspect = rect.width() / rect.height();
        let uv = if aspect > target_aspect {
            let width = target_aspect / aspect;
            Rect::from_min_max(
                pos2((1.0 - width) * 0.5, 0.0),
                pos2((1.0 + width) * 0.5, 1.0),
            )
        } else {
            let height = aspect / target_aspect;
            Rect::from_min_max(
                pos2(0.0, (1.0 - height) * 0.5),
                pos2(1.0, (1.0 + height) * 0.5),
            )
        };
        egui::Image::new((cover.id(), cover.size_vec2()))
            .uv(uv)
            .rounding(16.0)
            .paint_at(ui, rect);
        return;
    }
    // Independent rounded placeholder art, also used while an owned cover loads.
    let (base, accent) = palette(&game.slug);
    let painter = ui.painter().with_clip_rect(rect);
    painter.rect_filled(rect, 16.0, base.gamma_multiply(0.6));
    let inner = rect.shrink(18.0);
    let center = inner.center();
    painter.circle_filled(
        center - vec2(0.0, inner.height() * 0.09),
        inner.width() * 0.28,
        base,
    );
    painter.add(egui::Shape::convex_polygon(
        vec![
            center + vec2(0.0, -inner.height() * 0.34),
            center + vec2(inner.width() * 0.22, -inner.height() * 0.03),
            center + vec2(0.0, inner.height() * 0.28),
            center + vec2(-inner.width() * 0.22, -inner.height() * 0.03),
        ],
        accent.gamma_multiply(0.75),
        Stroke::NONE,
    ));
    let mut title = egui::text::LayoutJob::simple(
        game.name.clone(),
        shell::bold((rect.width() * 0.072).clamp(14.0, 22.0)),
        Color32::WHITE,
        inner.width(),
    );
    title.wrap.max_rows = 2;
    title.wrap.overflow_character = Some('…');
    let galley = ui.fonts(|fonts| fonts.layout_job(title));
    painter.galley(
        pos2(inner.left(), inner.bottom() - galley.size().y),
        galley,
        Color32::WHITE,
    );
}

fn open_details(app: &mut AeAppEguiApp, game: &GameInfo) {
    app.game_sel = game.slug.clone();
    app.library_details = true;
}
pub(crate) fn primary_game_action(app: &mut AeAppEguiApp, game: &GameInfo) {
    if app.is_preview() {
        return;
    }
    let queued = app.install_queue.contains_key(&game.offer)
        || app.installing_now.as_ref().is_some_and(|download| download.offer == game.offer);
    if queued {
        app.page_view = crate::PageType::Downloads;
    } else if !game.installed {
        set_app_modal!(app, Some(PopupModal::GameInstall(game.slug.clone())));
    } else if app.playing_game.is_none() {
        if !app.settings.ignore_ood_games && game.version.installed != game.version.latest {
            set_app_modal!(app, Some(PopupModal::GameLaunchOOD(game.slug.clone())));
        } else {
            let settings = app.settings.game_settings.get(&game.slug).cloned();
            app.playing_game = Some(game.slug.clone());
            if !request_game_action(
                app,
                AeAppLibRequest::StartGameRequest(game.clone(), settings),
            ) {
                app.playing_game = None;
            }
        }
    }
}
pub(crate) fn game_menu(app: &mut AeAppEguiApp, ui: &mut Ui, game: &GameInfo) {
    ui.set_min_width(220.0);
    if ui.button("View game details").clicked() {
        open_details(app, game);
        ui.close_menu();
    }
    ui.separator();
    let running = app.playing_game.as_ref() == Some(&game.slug);
    let queued = app.install_queue.contains_key(&game.offer)
        || app.installing_now.as_ref().is_some_and(|download| download.offer == game.offer);
    if running {
        if ui.add_enabled(!app.is_preview(), egui::Button::new("Stop game")).clicked() {
            request_game_action(app, AeAppLibRequest::StopGameRequest(game.offer.clone()));
            ui.close_menu();
        }
    } else {
        let label = if queued {
            "View download"
        } else if game.installed {
            "Play"
        } else {
            "Download"
        };
        if ui
            .add_enabled(
                !app.is_preview() && (queued || !game.installed || app.playing_game.is_none()),
                egui::Button::new(label),
            )
            .on_disabled_hover_text(if app.is_preview() {
                "Game actions are disabled in the local preview."
            } else {
                "Another game is running."
            })
            .clicked()
        {
            primary_game_action(app, game);
            ui.close_menu();
        }
    }
    if ui.button("Proton & game settings").clicked() {
        set_app_modal!(app, Some(PopupModal::GameSettings(game.slug.clone())));
        ui.close_menu();
    }
    if ui
        .add_enabled(
            !app.is_preview() && game.installed,
            egui::Button::new("Open installation folder"),
        )
        .clicked()
    {
        request_game_action(
            app,
            AeAppLibRequest::OpenGameFolderRequest(game.offer.clone()),
        );
        ui.close_menu();
    }
    if !game.dlc.is_empty() && ui.button("View add-ons").clicked() {
        ui.ctx().data_mut(|data| {
            data.insert_temp(egui::Id::new(("aeapp-game-hub-tab", &game.slug)), 2usize)
        });
        open_details(app, game);
        ui.close_menu();
    }
}
fn card(app: &mut AeAppEguiApp, ui: &mut Ui, game: &GameInfo, rect: Rect, section: &str) {
    let width = rect.width();
    let image_rect = Rect::from_min_size(rect.min, vec2(width, width));
    let response = ui.interact(
        image_rect,
        ui.id().with((section, &game.slug, "cover")),
        Sense::click(),
    );
    cover_artwork(app, ui, image_rect, game);
    if response.hovered() {
        ui.painter().rect_stroke(image_rect.expand(2.0), 17.0, Stroke::new(2.0, ACCENT_COLOR));
    }
    response.context_menu(|ui| game_menu(app, ui, game));
    let title_rect = Rect::from_min_size(
        image_rect.left_bottom() + vec2(0.0, 16.0),
        vec2(width, 18.0),
    );
    let mut title =
        egui::text::LayoutJob::simple_singleline(game.name.clone(), shell::bold(12.0), shell::TEXT);
    title.wrap.max_width = width;
    title.wrap.max_rows = 1;
    title.wrap.overflow_character = Some('…');
    let galley = ui.fonts(|fonts| fonts.layout_job(title));
    ui.painter().with_clip_rect(title_rect).galley(title_rect.min, galley, shell::TEXT);
    if ui
        .interact(
            title_rect,
            ui.id().with((section, &game.slug, "title")),
            Sense::click(),
        )
        .clicked()
    {
        open_details(app, game);
    }
    if !game.dlc.is_empty() {
        let addons = Rect::from_min_size(
            image_rect.left_bottom() + vec2(0.0, 41.0),
            vec2(130.0, 20.0),
        );
        let response = ui.interact(
            addons,
            ui.id().with((section, &game.slug, "addons")),
            Sense::click(),
        );
        ui.painter().text(
            addons.min,
            Align2::LEFT_TOP,
            "View add-ons",
            FontId::proportional(14.0),
            ACCENT_COLOR,
        );
        if response.clicked() {
            open_details(app, game);
        }
        response.on_hover_cursor(egui::CursorIcon::PointingHand);
    }
    let menu_rect =
        Rect::from_min_size(image_rect.right_top() + vec2(-43.0, 11.0), vec2(32.0, 32.0));
    let popup_id = ui.id().with((section, &game.slug, "menu_popup"));
    let menu_response = ui.interact(
        menu_rect,
        ui.id().with((section, &game.slug, "menu")),
        Sense::click(),
    );
    ui.painter().rect_filled(
        menu_rect,
        16.0,
        Color32::from_black_alpha(if menu_response.hovered() { 210 } else { 150 }),
    );
    for x in [-6.0, 0.0, 6.0] {
        ui.painter().circle_filled(menu_rect.center() + vec2(x, 0.0), 1.6, Color32::WHITE);
    }
    if menu_response.clicked() {
        ui.memory_mut(|memory| memory.toggle_popup(popup_id));
    }
    egui::popup_below_widget(
        ui,
        popup_id,
        &menu_response,
        egui::PopupCloseBehavior::CloseOnClick,
        |ui| game_menu(app, ui, game),
    );
    if response.clicked() && !menu_response.hovered() {
        open_details(app, game);
    }
}
fn game_grid(
    app: &mut AeAppEguiApp,
    ui: &mut Ui,
    games: &[GameInfo],
    columns: usize,
    width: f32,
    section: &str,
) {
    let available = ui.available_width();
    for (row_index, row) in games.chunks(columns).enumerate() {
        if row_index > 0 {
            ui.add_space(32.0);
        }
        let (row_rect, _) = ui.allocate_exact_size(vec2(available, width + 64.0), Sense::hover());
        for (column, game) in row.iter().enumerate() {
            let rect = Rect::from_min_size(
                row_rect.min + vec2(column as f32 * (width + 24.0), 0.0),
                vec2(width, width + 64.0),
            );
            if ui.is_rect_visible(rect) {
                card(app, ui, game, rect, section);
            }
        }
        ui.advance_cursor_after_rect(row_rect);
    }
}
fn section_heading(ui: &mut Ui, title: &str) -> Rect {
    let rect = ui.allocate_exact_size(vec2(ui.available_width(), 55.0), Sense::hover()).0;
    ui.painter().text(
        rect.min,
        Align2::LEFT_TOP,
        title,
        shell::bold(20.0),
        shell::TEXT,
    );
    rect
}

pub fn games_view(app: &mut AeAppEguiApp, ui: &mut Ui) {
    if app.library_details {
        game_view_details_panel(app, ui);
        return;
    }
    ui.spacing_mut().item_spacing.y = 0.0;
    let width = ui.available_width();
    let (header, _) = ui.allocate_exact_size(vec2(width, 247.0), Sense::hover());
    ui.painter().text(
        header.min,
        Align2::LEFT_TOP,
        "Library",
        shell::display(32.0),
        shell::TEXT,
    );
    ui.painter().line_segment(
        [header.min + vec2(0.0, 70.0), header.min + vec2(width, 70.0)],
        Stroke::new(1.0, shell::BORDER),
    );
    let controls = Rect::from_min_size(header.right_top() + vec2(-274.0, 103.0), vec2(274.0, 32.0));
    ui.allocate_ui_at_rect(controls, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 16.0;
            if ui
                .add_enabled(
                    !app.is_preview(),
                    egui::Button::new(egui::RichText::new("REFRESH").font(shell::bold(12.0)))
                        .fill(Color32::TRANSPARENT)
                        .rounding(4.0)
                        .min_size(vec2(102.0, 32.0)),
                )
                .on_hover_text("Refresh games owned by the connected EA account.")
                .clicked()
            {
                request_game_action(app, AeAppLibRequest::RefreshGamesRequest);
            }
            if ui
                .add_enabled(
                    !app.is_preview(),
                    egui::Button::new(egui::RichText::new("REDEEM CODE").font(shell::bold(12.0)))
                        .fill(Color32::TRANSPARENT)
                        .rounding(4.0)
                        .min_size(vec2(156.0, 32.0)),
                )
                .on_hover_text("Open EA's code redemption page in your browser.")
                .clicked()
            {
                ui.ctx().open_url(egui::OpenUrl::new_tab("https://www.ea.com/redeem"));
            }
        });
    });
    ui.advance_cursor_after_rect(header);
    let search = app.game_view_bar.search_buffer.to_lowercase();
    let mut games: Vec<GameInfo> = app
        .games
        .values()
        .filter(|game| game.name.to_lowercase().contains(&search))
        .cloned()
        .collect();
    games.sort_by_cached_key(|game| game.name.to_lowercase());
    if games.is_empty() {
        if app.games.is_empty() {
            shell::empty(ui, "Your library is waiting", "Games owned by the connected EA account will appear here. Use Refresh to check again.");
        } else {
            shell::empty(ui, "No matching games", "Try another search.");
        }
        return;
    }
    let installed: Vec<GameInfo> = games.iter().filter(|game| game.installed).cloned().collect();
    egui::ScrollArea::vertical()
        .id_source("library_sections")
        .auto_shrink(false)
        .show(ui, |ui| {
            let available = ui.available_width();
            let columns = ((available + 24.0) / 264.0).floor().max(1.0) as usize;
            let cover_width = (available - (columns - 1) as f32 * 24.0) / columns as f32;
            section_heading(ui, &format!("Installed ({})", installed.len()));
            if installed.is_empty() {
                ui.add_sized(
                    [available, 60.0],
                    egui::Label::new(
                        egui::RichText::new("Your installed games will appear here.")
                            .color(shell::MUTED),
                    ),
                );
            } else {
                game_grid(app, ui, &installed, columns, cover_width, "installed");
            }
            ui.add_space(89.0);
            let heading = section_heading(ui, &format!("My games ({})", games.len()));
            let collapse_id = egui::Id::new("library_owned_collapsed");
            let mut collapsed =
                ui.ctx().data_mut(|data| data.get_temp::<bool>(collapse_id)).unwrap_or(false);
            let toggle_rect =
                Rect::from_min_size(heading.right_top() - vec2(32.0, 1.0), vec2(32.0, 28.0));
            let response = ui.interact(toggle_rect, collapse_id, Sense::click());
            if response.clicked() {
                collapsed = !collapsed;
            }
            let center = toggle_rect.center();
            let direction = if collapsed { 1.0 } else { -1.0 };
            let stroke = Stroke::new(1.8, shell::TEXT);
            ui.painter().line_segment(
                [
                    center + vec2(-5.0, -direction * 2.0),
                    center + vec2(0.0, direction * 3.0),
                ],
                stroke,
            );
            ui.painter().line_segment(
                [
                    center + vec2(0.0, direction * 3.0),
                    center + vec2(5.0, -direction * 2.0),
                ],
                stroke,
            );
            response.on_hover_text(if collapsed {
                "Expand My games"
            } else {
                "Collapse My games"
            });
            ui.ctx().data_mut(|data| data.insert_temp(collapse_id, collapsed));
            if !collapsed {
                let all_id = egui::Id::new("library_show_all_owned");
                let show_all =
                    ui.ctx().data_mut(|data| data.get_temp::<bool>(all_id)).unwrap_or(false);
                let visible = if show_all {
                    games.len()
                } else {
                    games.len().min(columns)
                };
                game_grid(app, ui, &games[..visible], columns, cover_width, "owned");
                if games.len() > columns {
                    ui.add_space(32.0);
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new(if show_all {
                                    "VIEW LESS"
                                } else {
                                    "VIEW ALL"
                                })
                                .font(shell::bold(12.0)),
                            )
                            .fill(Color32::TRANSPARENT)
                            .rounding(4.0)
                            .min_size(vec2(108.0, 32.0)),
                        )
                        .clicked()
                    {
                        ui.ctx().data_mut(|data| data.insert_temp(all_id, !show_all));
                    }
                }
            }
            ui.add_space(48.0);
        });
}

pub fn game_view_details_panel(app: &mut AeAppEguiApp, ui: &mut Ui) {
    super::game_hub_view::show(app, ui);
}
