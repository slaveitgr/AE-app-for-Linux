//! Windows EA game-hub layout; account statistics and offers are never fabricated.
use super::game_view::{cover_artwork, game_menu, primary_game_action};
use crate::bridge_thread::AeAppLibRequest;
use crate::{shell, ui_image::UIImageType, AeAppEguiApp, GameDetailsWrapper, GameInfo, PageType};
use egui::{pos2, vec2, Align2, Color32, FontId, Rect, Sense, Stroke, Ui};
const PANEL: Color32 = Color32::from_rgba_premultiplied(18, 19, 31, 235);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GameHubTab {
    Overview = 0,
    Currency = 1,
    Addons = 2,
}

fn tab_key(slug: &str) -> egui::Id {
    egui::Id::new(("aeapp-game-hub-tab", slug))
}

/// Open a known library title on the requested tab, regardless of its previous selection.
pub(crate) fn open_tab(app: &mut AeAppEguiApp, ctx: &egui::Context, slug: &str, tab: GameHubTab) {
    if !app.games.contains_key(slug) {
        return;
    }
    app.game_sel = slug.to_owned();
    app.page_view = PageType::Games;
    app.library_details = true;
    ctx.data_mut(|data| data.insert_temp(tab_key(slug), tab as usize));
}

fn begin_details_request(
    details: &mut GameDetailsWrapper,
    sender: &std::sync::mpsc::Sender<AeAppLibRequest>,
    slug: &str,
) -> bool {
    if !matches!(
        details,
        GameDetailsWrapper::Unloaded | GameDetailsWrapper::Failed(_)
    ) {
        return false;
    }
    match sender.send(AeAppLibRequest::GetGameDetailsRequest(slug.to_owned())) {
        Ok(()) => {
            *details = GameDetailsWrapper::Loading;
            true
        }
        Err(_) => {
            *details = GameDetailsWrapper::Failed(
                "The game-details service is unavailable. Reconnect and try again.".into(),
            );
            false
        }
    }
}

fn request_details(app: &mut AeAppEguiApp, slug: &str) {
    if app.is_preview() {
        return;
    }
    if let Some(game) = app.games.get_mut(slug) {
        begin_details_request(&mut game.details, &app.backend.backend_commander, slug);
    }
}

fn details_notice(app: &mut AeAppEguiApp, ui: &mut Ui, game: &GameInfo) {
    match &game.details {
        GameDetailsWrapper::Loading => {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Loading game details…");
            });
            ui.add_space(18.0);
        }
        GameDetailsWrapper::Failed(message) => {
            egui::Frame::none().fill(shell::SURFACE).rounding(8.0).inner_margin(16.0).show(
                ui,
                |ui| {
                    ui.set_width((ui.available_width() - 32.0).max(0.0));
                    ui.label(
                        egui::RichText::new("Game details couldn't load").font(shell::bold(16.0)),
                    );
                    ui.label(message);
                    ui.add_space(8.0);
                    if ui.add_enabled(!app.is_preview(), egui::Button::new("Retry")).clicked() {
                        request_details(app, &game.slug);
                    }
                },
            );
            ui.add_space(18.0);
        }
        _ => {}
    }
}

fn playtime_text(time: Option<u32>) -> String {
    match time {
        Some(time) => format!("{} hrs {} mins", time / 10, (time % 10) * 6),
        None => "Unavailable".to_owned(),
    }
}

fn achievement_count(unlocked: Option<u16>, total: Option<u16>) -> Option<String> {
    match (unlocked, total) {
        (Some(unlocked), Some(total)) if unlocked <= total => {
            Some(format!("{unlocked} / {total} unlocked"))
        }
        _ => None,
    }
}

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
    let slug = app.game_sel.clone();
    if app
        .games
        .get(&slug)
        .is_some_and(|game| matches!(game.details, GameDetailsWrapper::Unloaded))
    {
        request_details(app, &slug);
    }
    let Some(game) = app.games.get(&slug).cloned() else {
        app.library_details = false;
        return;
    };
    ui.spacing_mut().item_spacing.y = 0.0;
    let tab_id = tab_key(&game.slug);
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
            details_notice(app, ui, &game);
            match selected {
                1 => {
                    ui.label(
                        egui::RichText::new("Currency offers are unavailable here.")
                            .color(shell::MUTED),
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
                            egui::RichText::new("No owned add-ons were returned for this game.")
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
        "Unavailable",
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
    let playtime = playtime_text(match &game.details {
        GameDetailsWrapper::Available(details) => details.time,
        _ => None,
    });
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
        "Unavailable",
        FontId::proportional(14.0),
        shell::TEXT,
    );
    rule(ui, inner.min + vec2(0.0, 39.0), inner.width());
    label(
        ui,
        inner.min + vec2(0.0, 56.0),
        "Age rating unavailable.",
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
    let (count, no_achievements) = match &game.details {
        GameDetailsWrapper::Available(details) => (
            achievement_count(details.achievements_unlocked, details.achievements_total),
            details.achievements_total == Some(0)
                && matches!(details.achievements_unlocked, None | Some(0)),
        ),
        _ => (None, false),
    };
    let title = count
        .map(|count| format!("Achievements ({count})"))
        .unwrap_or_else(|| "Achievements".to_owned());
    ui.label(egui::RichText::new(title).font(shell::bold(16.0)));
    ui.add_space(20.0);
    ui.label(
        egui::RichText::new(if no_achievements {
            "No achievements are reported for this game."
        } else {
            "Achievement information is unavailable."
        })
        .color(shell::MUTED),
    );
    ui.add_space(28.0);
    external(app, ui, game, "VIEW GAME ON EA.COM");
    if let GameDetailsWrapper::Available(details) = &game.details {
        if details.system_requirements_min.is_none() && details.system_requirements_rec.is_none() {
            ui.add_space(28.0);
            ui.label(
                egui::RichText::new("System requirements are unavailable.").color(shell::MUTED),
            );
        }
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

#[cfg(test)]
mod tests {
    use super::{achievement_count, begin_details_request, playtime_text};
    use crate::{bridge_thread::AeAppLibRequest, GameDetails, GameDetailsWrapper};

    fn unavailable_details() -> GameDetails {
        GameDetails {
            time: None,
            achievements_unlocked: None,
            achievements_total: None,
            path: String::new(),
            system_requirements_min: None,
            system_requirements_rec: None,
        }
    }

    #[test]
    fn verified_zero_playtime_is_different_from_unavailable() {
        assert_eq!(playtime_text(None), "Unavailable");
        assert_eq!(playtime_text(Some(0)), "0 hrs 0 mins");
        assert_eq!(playtime_text(Some(12)), "1 hrs 12 mins");
    }

    #[test]
    fn achievement_counts_require_known_consistent_values_and_preserve_zero() {
        assert_eq!(achievement_count(None, None), None);
        assert_eq!(achievement_count(Some(0), None), None);
        assert_eq!(achievement_count(None, Some(12)), None);
        assert_eq!(achievement_count(Some(13), Some(12)), None);
        assert_eq!(
            achievement_count(Some(0), Some(0)),
            Some("0 / 0 unlocked".into())
        );
        assert_eq!(
            achievement_count(Some(0), Some(12)),
            Some("0 / 12 unlocked".into())
        );
    }

    #[test]
    fn failed_response_can_retry_and_loading_does_not_send_duplicates() {
        let (sender, receiver) = std::sync::mpsc::channel();
        let mut details: GameDetailsWrapper = Err("Service unavailable".to_owned()).into();
        assert!(
            matches!(&details, GameDetailsWrapper::Failed(message) if message == "Service unavailable")
        );

        assert!(begin_details_request(&mut details, &sender, "owned-game"));
        assert!(matches!(details, GameDetailsWrapper::Loading));
        assert!(
            matches!(receiver.try_recv(), Ok(AeAppLibRequest::GetGameDetailsRequest(slug)) if slug == "owned-game")
        );
        assert!(!begin_details_request(&mut details, &sender, "owned-game"));
        assert!(receiver.try_recv().is_err());

        details = Ok(unavailable_details()).into();
        assert!(matches!(details, GameDetailsWrapper::Available(ref data) if data.time.is_none()));
        assert!(!begin_details_request(&mut details, &sender, "owned-game"));
    }

    #[test]
    fn disconnected_request_has_retryable_failure_instead_of_stuck_loading() {
        let (sender, receiver) = std::sync::mpsc::channel();
        drop(receiver);
        let mut details = GameDetailsWrapper::Unloaded;
        assert!(!begin_details_request(&mut details, &sender, "owned-game"));
        assert!(matches!(details, GameDetailsWrapper::Failed(_)));

        let (reconnected, receiver) = std::sync::mpsc::channel();
        assert!(begin_details_request(
            &mut details,
            &reconnected,
            "owned-game"
        ));
        assert!(matches!(details, GameDetailsWrapper::Loading));
        assert!(receiver.try_recv().is_ok());
    }
}
