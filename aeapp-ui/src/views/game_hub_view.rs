//! Windows EA game-hub layout; account statistics and offers are never fabricated.
use super::game_view::{cover_artwork, game_menu, primary_game_action};
use crate::bridge_thread::AeAppLibRequest;
use crate::{shell, ui_image::UIImageType, AeAppEguiApp, GameDetailsWrapper, GameInfo, PageType};
use egui::{pos2, vec2, Align2, Color32, FontId, Rect, Sense, Stroke, Ui};
const PANEL: Color32 = Color32::from_rgba_premultiplied(18, 19, 31, 235);

fn text(ui: &Ui, rect: Rect, value: &str, font: FontId, color: Color32) {
    let galley = ui.painter().layout(value.to_owned(), font, color, rect.width());
    ui.painter().with_clip_rect(rect).galley(rect.min, galley, color);
}
fn label(ui: &Ui, origin: egui::Pos2, value: &str, size: f32, color: Color32, width: f32) {
    text(
        ui,
        Rect::from_min_size(origin, vec2(width, 44.0)),
        value,
        FontId::proportional(size),
        color,
    );
}
fn rule(ui: &Ui, left: egui::Pos2, width: f32) {
    ui.painter().line_segment(
        [left, left + vec2(width, 0.0)],
        Stroke::new(1.0, shell::BORDER),
    );
}
fn external(app: &AeAppEguiApp, ui: &mut Ui, game: &GameInfo, caption: &str) {
    if ui.add_enabled(!app.is_preview(), egui::Button::new(caption)).clicked() {
        ui.ctx().open_url(egui::OpenUrl::new_tab(super::catalog_view::product_url(
            ui, &game.slug,
        )));
    }
}
/// Painted before the toolbar, just like the original Windows game background.
pub fn backdrop(app: &AeAppEguiApp, ui: &Ui, rect: Rect) {
    let Some(game) = app.games.get(&app.game_sel) else {
        return;
    };
    let texture = app
        .img_cache
        .get(UIImageType::Background(game.slug.clone()))
        .or_else(|| app.img_cache.get(UIImageType::Hero(game.slug.clone())))
        .or_else(|| app.img_cache.get(UIImageType::Cover(game.slug.clone())));
    if let Some(texture) = texture {
        let height = (rect.width() / texture.aspect_ratio()).max(720.0);
        let image = Rect::from_min_size(rect.min, vec2(rect.width(), height));
        egui::Image::new((texture.id(), texture.size_vec2())).paint_at(ui, image);
        ui.painter().rect_filled(rect, 0.0, Color32::from_rgba_unmultiplied(30, 30, 48, 190));
        let fade = Rect::from_min_max(pos2(rect.left(), rect.top() + 500.0), rect.right_bottom());
        let mut mesh = egui::Mesh::default();
        for (position, alpha) in [
            (fade.left_top(), 0),
            (fade.right_top(), 0),
            (fade.right_bottom(), 255),
            (fade.left_bottom(), 255),
        ] {
            mesh.colored_vertex(position, Color32::from_rgba_unmultiplied(30, 30, 48, alpha));
        }
        mesh.add_triangle(0, 1, 2);
        mesh.add_triangle(0, 2, 3);
        ui.painter().add(egui::Shape::mesh(mesh));
    }
}

pub fn show(app: &mut AeAppEguiApp, ui: &mut Ui) {
    let Some(game) = app.games.get(&app.game_sel).cloned() else {
        app.library_details = false;
        return;
    };
    if matches!(game.details, GameDetailsWrapper::Unloaded) && !app.is_preview() {
        if app
            .backend
            .backend_commander
            .send(AeAppLibRequest::GetGameDetailsRequest(game.slug.clone()))
            .is_ok()
        {
            if let Some(info) = app.games.get_mut(&game.slug) {
                info.details = GameDetailsWrapper::Loading;
            }
        }
    }
    ui.spacing_mut().item_spacing.y = 0.0;
    let tab_id = egui::Id::new(("aeapp-game-hub-tab", &game.slug));
    let mut selected = ui.ctx().data(|d| d.get_temp::<usize>(tab_id)).unwrap_or(0).min(2);
    egui::ScrollArea::vertical()
        .id_source(("game-hub", &game.slug))
        .auto_shrink(false)
        .show(ui, |ui| {
            let width = ui.available_width();
            let header = ui.allocate_exact_size(vec2(width, 127.0), Sense::hover()).0;
            ui.painter().text(
                header.min,
                Align2::LEFT_TOP,
                &game.name,
                shell::display(32.0),
                shell::TEXT,
            );
            let tabs = Rect::from_min_size(header.min + vec2(0.0, 45.0), vec2(width, 34.0));
            ui.allocate_ui_at_rect(tabs, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    ui.spacing_mut().button_padding = vec2(12.0, 8.0);
                    for (index, name) in ["Overview", "Currency", "Add-ons"].into_iter().enumerate()
                    {
                        if ui
                            .add(
                                egui::Button::new(egui::RichText::new(name).size(15.0).color(
                                    if selected == index {
                                        Color32::from_rgb(20, 23, 36)
                                    } else {
                                        shell::TEXT
                                    },
                                ))
                                .fill(if selected == index {
                                    Color32::WHITE
                                } else {
                                    Color32::TRANSPARENT
                                })
                                .stroke(Stroke::NONE)
                                .rounding(8.0)
                                .min_size(vec2(0.0, 34.0)),
                            )
                            .clicked()
                        {
                            selected = index;
                        }
                    }
                });
            });
            ui.advance_cursor_after_rect(header);
            match selected {
                1 => {
                    ui.label(
                        egui::RichText::new("Currency offers are not loaded.").color(shell::MUTED),
                    );
                    ui.add_space(18.0);
                    external(app, ui, &game, "VIEW CURRENCY ON EA.COM");
                }
                2 => {
                    ui.horizontal(|ui| {
                        ui.label("Requires:");
                        ui.label(egui::RichText::new(&game.name).color(crate::ACCENT_COLOR));
                    });
                    ui.add_space(28.0);
                    ui.separator();
                    ui.add_space(16.0);
                    if game.dlc.is_empty() {
                        ui.label(
                            egui::RichText::new("Add-on details are not loaded.")
                                .color(shell::MUTED),
                        );
                    } else {
                        for addon in &game.dlc {
                            let row = ui.allocate_exact_size(vec2(width, 100.0), Sense::hover()).0;
                            ui.painter().rect_filled(
                                Rect::from_min_size(row.min, vec2(80.0, 80.0)),
                                12.0,
                                PANEL,
                            );
                            shell::original_icon(
                                ui,
                                Rect::from_min_size(row.min + vec2(24.0, 24.0), vec2(32.0, 32.0)),
                                "COLLECTION",
                                shell::MUTED,
                            );
                            text(
                                ui,
                                Rect::from_min_size(
                                    row.min + vec2(104.0, 8.0),
                                    vec2(width - 104.0, 40.0),
                                ),
                                addon.offer().display_name(),
                                shell::bold(16.0),
                                shell::TEXT,
                            );
                            label(
                                ui,
                                row.min + vec2(104.0, 48.0),
                                "Owned add-on",
                                14.0,
                                shell::MUTED,
                                width - 104.0,
                            );
                            rule(ui, row.left_bottom(), width);
                        }
                    }
                    ui.add_space(20.0);
                    external(app, ui, &game, "VIEW ADD-ONS ON EA.COM");
                }
                _ => overview(app, ui, &game),
            }
            ui.add_space(40.0);
        });
    ui.ctx().data_mut(|d| d.insert_temp(tab_id, selected));
}

fn overview(app: &mut AeAppEguiApp, ui: &mut Ui, game: &GameInfo) {
    let width = ui.available_width();
    let compact = width < 760.0;
    let region = ui
        .allocate_exact_size(
            vec2(width, if compact { 594.0 } else { 320.0 }),
            Sense::hover(),
        )
        .0;
    let info_width = if compact {
        width
    } else {
        (width * 0.32).clamp(260.0, 432.0)
    };
    let card_width = if compact {
        width
    } else {
        (width - info_width - 24.0).min(780.0).max(400.0)
    };
    let card = Rect::from_min_size(region.min, vec2(card_width, 320.0));
    let portrait_width = (card_width * 0.32).min(217.0);
    ui.painter().rect_filled(card, 16.0, PANEL);
    cover_artwork(
        app,
        ui,
        Rect::from_min_size(card.min, vec2(portrait_width, 320.0)),
        game,
    );
    let origin = card.min + vec2(portrait_width + 24.0, 24.0);
    let content_width = card.right() - origin.x - 24.0;
    shell::original_icon(
        ui,
        Rect::from_min_size(origin, vec2(24.0, 24.0)),
        "COLLECTION",
        shell::TEXT,
    );
    label(
        ui,
        origin + vec2(36.0, 3.0),
        if app.is_preview() {
            "Sample library game"
        } else {
            "In your library"
        },
        13.0,
        shell::TEXT,
        content_width - 36.0,
    );
    rule(ui, origin + vec2(0.0, 41.0), content_width);
    label(
        ui,
        origin + vec2(0.0, 59.0),
        "Last played",
        14.0,
        shell::MUTED,
        content_width,
    );
    ui.painter().text(
        origin + vec2(content_width, 59.0),
        Align2::RIGHT_TOP,
        "—",
        FontId::proportional(14.0),
        shell::TEXT,
    );
    rule(ui, origin + vec2(0.0, 93.0), content_width);
    label(
        ui,
        origin + vec2(0.0, 111.0),
        "Time played",
        14.0,
        shell::MUTED,
        content_width,
    );
    let playtime = match &game.details {
        GameDetailsWrapper::Available(details) if details.time > 0 && !app.is_preview() => {
            format!("{} hrs {} mins", details.time / 10, (details.time % 10) * 6)
        }
        _ => "Not loaded".to_owned(),
    };
    ui.painter().text(
        origin + vec2(content_width, 111.0),
        Align2::RIGHT_TOP,
        playtime,
        FontId::proportional(14.0),
        shell::TEXT,
    );
    let action = Rect::from_min_size(card.right_bottom() - vec2(166.0, 56.0), vec2(142.0, 32.0));
    let queued = app.install_queue.contains_key(&game.offer)
        || app.installing_now.as_ref().is_some_and(|q| q.offer == game.offer);
    let running = app.playing_game.as_ref() == Some(&game.slug);
    let title = if running {
        "STOP"
    } else if queued {
        "VIEW DOWNLOAD"
    } else if game.installed {
        "PLAY"
    } else {
        "DOWNLOAD"
    };
    let enabled =
        !app.is_preview() && (running || queued || !game.installed || app.playing_game.is_none());
    if ui
        .add_enabled_ui(enabled, |ui| {
            ui.put(
                action,
                egui::Button::new(
                    egui::RichText::new(title).font(shell::bold(12.0)).color(Color32::WHITE),
                )
                .fill(crate::ACCENT_COLOR)
                .stroke(Stroke::NONE)
                .rounding(8.0),
            )
        })
        .inner
        .clicked()
    {
        if running {
            let _ = app
                .backend
                .backend_commander
                .send(AeAppLibRequest::StopGameRequest(game.offer.clone()));
        } else {
            primary_game_action(app, game);
        }
    }
    let menu = Rect::from_min_size(action.min - vec2(44.0, 0.0), vec2(32.0, 32.0));
    ui.allocate_ui_at_rect(menu, |ui| {
        ui.menu_button("…", |ui| game_menu(app, ui, game));
    });
    let info = Rect::from_min_size(
        if compact {
            region.min + vec2(0.0, 344.0)
        } else {
            pos2(region.right() - info_width, region.top() + 70.0)
        },
        vec2(info_width, 250.0),
    );
    ui.painter().rect_filled(info, 16.0, PANEL);
    let inner = info.shrink(16.0);
    label(
        ui,
        inner.min + vec2(0.0, 8.0),
        "Release date",
        14.0,
        shell::MUTED,
        inner.width(),
    );
    ui.painter().text(
        inner.right_top() + vec2(0.0, 8.0),
        Align2::RIGHT_TOP,
        "Not loaded",
        FontId::proportional(14.0),
        shell::TEXT,
    );
    rule(ui, inner.min + vec2(0.0, 39.0), inner.width());
    label(
        ui,
        inner.min + vec2(0.0, 56.0),
        "Age rating is not loaded.",
        14.0,
        shell::MUTED,
        inner.width(),
    );
    label(
        ui,
        inner.min + vec2(0.0, 89.0),
        "Windows game · Proton",
        14.0,
        shell::TEXT,
        inner.width(),
    );
    rule(ui, inner.min + vec2(0.0, 132.0), inner.width());
    text(
        ui,
        Rect::from_min_size(inner.min + vec2(0.0, 146.0), vec2(inner.width(), 72.0)),
        "Prices, age ratings and legal disclosures are available on the official EA product page.",
        FontId::proportional(12.0),
        shell::MUTED,
    );
    ui.advance_cursor_after_rect(region);
    ui.add_space(25.0);
    ui.separator();
    ui.add_space(24.0);
    let title = match &game.details {
        GameDetailsWrapper::Available(details)
            if details.achievements_total > 0 && !app.is_preview() =>
        {
            format!(
                "Achievements ({} / {} unlocked)",
                details.achievements_unlocked, details.achievements_total
            )
        }
        _ => "Achievements".to_owned(),
    };
    ui.label(egui::RichText::new(title).font(shell::bold(16.0)));
    ui.add_space(20.0);
    ui.label(egui::RichText::new("Achievement details are not loaded.").color(shell::MUTED));
    ui.add_space(28.0);
    external(app, ui, game, "VIEW GAME ON EA.COM");
    if let GameDetailsWrapper::Available(details) = &game.details {
        if let Some(requirements) = &details.system_requirements_min {
            ui.add_space(50.0);
            ui.heading("Minimum requirements");
            egui_demo_lib::easy_mark::easy_mark(ui, requirements);
        }
        if let Some(requirements) = &details.system_requirements_rec {
            ui.add_space(24.0);
            ui.heading("Recommended requirements");
            egui_demo_lib::easy_mark::easy_mark(ui, requirements);
        }
    }
}
