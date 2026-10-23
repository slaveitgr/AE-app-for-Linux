use crate::{
    bridge_thread::AeAppLibRequest, set_app_modal, shell, AeAppEguiApp, GameDetailsWrapper,
    GameInfo, PopupModal, ACCENT_COLOR,
};
use egui::{
    pos2, vec2, Align2, Color32, FontId, Margin, Rect, RichText, Rounding, Sense, Stroke, Ui,
};
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

fn card(app: &mut AeAppEguiApp, ui: &mut Ui, game: &GameInfo, rect: Rect) {
    let width = rect.width();
    let height = width * 1.18;
    let image_rect = Rect::from_min_size(rect.min, vec2(width, height));
    let response = ui.interact(rect, ui.id().with(&game.slug), Sense::click());
    artwork(app, ui, image_rect, game);
    if response.hovered() {
        ui.painter().rect_stroke(image_rect.expand(2.0), 2.0, Stroke::new(2.0, ACCENT_COLOR));
    }
    let title_rect = Rect::from_min_size(
        image_rect.left_bottom() + vec2(0.0, 12.0),
        vec2(width, 23.0),
    );
    let mut title = egui::text::LayoutJob::simple_singleline(
        game.name.clone(),
        FontId::proportional(15.0),
        shell::TEXT,
    );
    title.wrap.max_width = width;
    title.wrap.max_rows = 1;
    title.wrap.break_anywhere = true;
    title.wrap.overflow_character = Some('…');
    let galley = ui.fonts(|fonts| fonts.layout_job(title));
    ui.painter().with_clip_rect(title_rect).galley(title_rect.min, galley, shell::TEXT);
    let status_rect =
        Rect::from_min_size(title_rect.left_bottom() + vec2(0.0, 3.0), vec2(width, 20.0));
    let status = if app.is_preview() {
        if game.installed {
            "Demo · installed state"
        } else {
            "Demo · library state"
        }
    } else if game.installed {
        "Installed"
    } else {
        "Ready to download"
    };
    ui.painter().with_clip_rect(status_rect).text(
        status_rect.left_top(),
        Align2::LEFT_TOP,
        status,
        FontId::proportional(12.0),
        if game.installed {
            shell::GREEN
        } else {
            shell::MUTED
        },
    );
    if response.clicked() {
        app.game_sel = game.slug.clone();
        app.library_details = true;
    }
}

pub fn games_view(app: &mut AeAppEguiApp, ui: &mut Ui) {
    if app.library_details {
        game_view_details_panel(app, ui);
        return;
    }
    ui.horizontal(|ui| {
        ui.label(RichText::new("Your library").size(34.0).strong());
        ui.add_space(9.0);
        shell::pill(ui, &app.games.len().to_string(), shell::MUTED);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.add_enabled(!app.is_preview(), egui::Button::new("Refresh library")).clicked() {
                request_game_action(app, AeAppLibRequest::RefreshGamesRequest);
            }
        });
    });
    ui.add_space(7.0);
    ui.label(RichText::new("Good games. All in one place.").color(shell::MUTED));
    ui.add_space(23.0);
    ui.horizontal(|ui| {
        ui.selectable_value(&mut app.installed_only, false, "All games");
        ui.selectable_value(&mut app.installed_only, true, "Installed");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new("A–Z  ↓").color(shell::MUTED).size(13.0));
        });
    });
    ui.add_space(12.0);
    ui.separator();
    ui.add_space(20.0);
    let search = app.game_view_bar.search_buffer.to_lowercase();
    let mut games: Vec<GameInfo> = app
        .games
        .values()
        .filter(|g| (!app.installed_only || g.installed) && g.name.to_lowercase().contains(&search))
        .cloned()
        .collect();
    games.sort_by(|a, b| a.name.cmp(&b.name));
    if games.is_empty() {
        if app.games.is_empty() {
            shell::empty(ui, "Your library is waiting", "Games owned by the connected EA account will appear here. Use Refresh library to check again.");
        } else {
            shell::empty(
                ui,
                "No matching games",
                "Try another search or switch to All games.",
            );
        }
        return;
    }
    egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
        let gap = 24.0;
        let available = ui.available_width();
        let columns = ((available + gap) / (240.0 + gap)).floor().max(1.0) as usize;
        let width = (available - (columns - 1) as f32 * gap) / columns as f32;
        let height = width * 1.18 + 76.0;
        for row in games.chunks(columns) {
            let (row_rect, _) = ui.allocate_exact_size(vec2(available, height), Sense::hover());
            for (column, game) in row.iter().enumerate() {
                let rect = Rect::from_min_size(row_rect.min + vec2(column as f32 * (width + gap), 0.0), vec2(width, height));
                card(app, ui, game, rect);
            }
            ui.add_space(10.0);
        }
        if app.is_preview() {
            ui.label(RichText::new("Illustrative demo artwork and sample titles. This screen does not represent account ownership or game compatibility.").size(11.0).color(shell::MUTED));
        }
    });
}

pub fn game_view_details_panel(app: &mut AeAppEguiApp, ui: &mut Ui) {
    if ui.button("←  Back to library").clicked() {
        app.library_details = false;
    }
    ui.add_space(17.0);
    let Some(game) = app.games.get(&app.game_sel).cloned() else {
        app.library_details = false;
        return;
    };
    if matches!(game.details, GameDetailsWrapper::Unloaded) && !app.is_preview() {
        let _ = app
            .backend
            .backend_commander
            .send(AeAppLibRequest::GetGameDetailsRequest(game.slug.clone()));
        if let Some(info) = app.games.get_mut(&game.slug) {
            info.details = GameDetailsWrapper::Loading;
        }
    }
    egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
        let (hero, _) = ui.allocate_exact_size(vec2(ui.available_width(), 265.0), Sense::hover());
        artwork(app, ui, hero, &game);
        ui.add_space(20.0);
        ui.label(RichText::new(&game.name).size(31.0).strong());
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            shell::pill(
                ui,
                if game.installed {
                    "Installed"
                } else {
                    "In your library"
                },
                shell::GREEN,
            );
            shell::pill(ui, "Windows · Proton", shell::MUTED);
        });
        ui.add_space(19.0);
        ui.horizontal(|ui| {
            let running = app.playing_game.as_ref() == Some(&game.slug);
            if running {
                if ui
                    .add_enabled(
                        !app.is_preview(),
                        egui::Button::new("■  Stop game").min_size(vec2(140.0, 42.0)),
                    )
                    .clicked()
                {
                    request_game_action(app, AeAppLibRequest::StopGameRequest(game.offer.clone()));
                }
            } else {
                let queued = app.install_queue.contains_key(&game.offer)
                    || app.installing_now.as_ref().is_some_and(|q| q.offer == game.offer);
                ui.add_enabled_ui(
                    !app.is_preview() && (queued || app.playing_game.is_none()),
                    |ui| {
                        let label = if queued {
                            "View download"
                        } else if game.installed {
                            "▶  Play"
                        } else {
                            "↓  Download"
                        };
                        if shell::primary_button(ui, label).clicked() {
                            if queued {
                                app.page_view = crate::PageType::Downloads;
                            } else if game.installed {
                                if !app.settings.ignore_ood_games
                                    && game.version.installed != game.version.latest
                                {
                                    set_app_modal!(
                                        app,
                                        Some(PopupModal::GameLaunchOOD(game.slug.clone()))
                                    );
                                } else {
                                    let settings =
                                        app.settings.game_settings.get(&game.slug).cloned();
                                    app.playing_game = Some(game.slug.clone());
                                    if !request_game_action(
                                        app,
                                        AeAppLibRequest::StartGameRequest(game.clone(), settings),
                                    ) {
                                        app.playing_game = None;
                                    }
                                }
                            } else {
                                set_app_modal!(
                                    app,
                                    Some(PopupModal::GameInstall(game.slug.clone()))
                                );
                            }
                        }
                    },
                );
            }
            if ui.add_sized(vec2(150.0, 42.0), egui::Button::new("Manage game")).clicked() {
                set_app_modal!(app, Some(PopupModal::GameSettings(game.slug.clone())));
            }
            if ui
                .add_enabled(
                    !app.is_preview() && game.installed,
                    egui::Button::new("Open game folder").min_size(vec2(150.0, 42.0)),
                )
                .clicked()
            {
                request_game_action(
                    app,
                    AeAppLibRequest::OpenGameFolderRequest(game.offer.clone()),
                );
            }
        });
        if app.is_preview() {
            ui.label(
                RichText::new("Demo only. No launch, installation or EA requests are made.")
                    .size(12.0)
                    .color(shell::MUTED),
            );
        }
        ui.add_space(23.0);
        ui.separator();
        ui.add_space(16.0);
        ui.heading("Game details");
        if let GameDetailsWrapper::Available(details) = &game.details {
            if app.is_preview() {
                ui.label(
                    RichText::new("Sample metadata. No account statistics are loaded.")
                        .color(shell::MUTED),
                );
            } else if details.time == 0 && details.achievements_total == 0 {
                ui.label(
                    RichText::new("Playtime and achievements are not loaded.").color(shell::MUTED),
                );
            } else {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("{:.1} hours played", details.time as f32 / 10.0))
                            .color(shell::MUTED),
                    );
                    ui.separator();
                    ui.label(
                        RichText::new(format!(
                            "{} / {} achievements",
                            details.achievements_unlocked, details.achievements_total
                        ))
                        .color(shell::MUTED),
                    );
                });
            }
            if !details.path.is_empty() {
                ui.add_space(8.0);
                ui.label(RichText::new(&details.path).size(12.0).color(shell::MUTED));
            }
            ui.add_space(18.0);
            if let Some(requirements) = &details.system_requirements_min {
                ui.heading("Minimum requirements");
                egui_demo_lib::easy_mark::easy_mark(ui, requirements);
            }
            if let Some(requirements) = &details.system_requirements_rec {
                ui.add_space(12.0);
                ui.heading("Recommended requirements");
                egui_demo_lib::easy_mark::easy_mark(ui, requirements);
            }
        } else {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Loading game details…");
            });
        }
        if !game.dlc.is_empty() {
            ui.add_space(20.0);
            ui.heading("Additional content");
            for content in &game.dlc {
                ui.label(content.offer().display_name());
            }
        }
        ui.add_space(20.0);
    });
}
