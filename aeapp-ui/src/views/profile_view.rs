use aeapp::rtm::client::BasicPresence;
use egui::{
    pos2, vec2, Align2, Color32, Context, FontId, Id, Order, Rect, RichText, Rounding, Sense,
    Stroke, Ui,
};

use crate::{
    shell, ui_image::UIImageType, views::game_view::cover_artwork, AeAppEguiApp, BackendStallState,
    PageType,
};

const PANEL: Color32 = Color32::from_rgb(30, 30, 48);
const TEXT: Color32 = Color32::from_rgb(244, 244, 248);
const MUTED: Color32 = Color32::from_rgb(157, 155, 168);
const BLUE: Color32 = Color32::from_rgb(39, 106, 252);

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum ProfileTab {
    #[default]
    Games,
    Friends,
}

fn clipped_text(ui: &Ui, rect: Rect, text: &str, font: FontId, color: Color32) {
    let mut job = egui::text::LayoutJob::simple_singleline(text.to_owned(), font, color);
    job.wrap.max_width = rect.width();
    job.wrap.max_rows = 1;
    job.wrap.overflow_character = Some('…');
    let galley = ui.fonts(|fonts| fonts.layout_job(job));
    ui.painter().with_clip_rect(rect).galley(rect.min, galley, color);
}

fn friend_status(presence: &BasicPresence, in_game: bool) -> (&'static str, Color32) {
    match presence {
        BasicPresence::Online if in_game => ("In game", shell::GREEN),
        BasicPresence::Online => ("Online", shell::GREEN),
        BasicPresence::Away => ("Away", Color32::from_rgb(244, 185, 66)),
        BasicPresence::Dnd => ("Do not disturb", Color32::from_rgb(255, 106, 113)),
        BasicPresence::Offline | BasicPresence::Unknown => ("Offline", MUTED),
    }
}

fn friend_avatar(app: &AeAppEguiApp, ui: &Ui, rect: Rect, id: &str) {
    let avatar = app
        .img_cache
        .get(UIImageType::Avatar(id.to_owned()))
        .unwrap_or_else(|| app.img_cache.placeholder_avatar.clone());
    let aspect = avatar.aspect_ratio();
    let uv = if aspect > 1.0 {
        let width = 1.0 / aspect;
        Rect::from_min_max(
            pos2((1.0 - width) * 0.5, 0.0),
            pos2((1.0 + width) * 0.5, 1.0),
        )
    } else {
        Rect::from_min_max(
            pos2(0.0, (1.0 - aspect) * 0.5),
            pos2(1.0, (1.0 + aspect) * 0.5),
        )
    };
    egui::Image::new((avatar.id(), avatar.size_vec2()))
        .uv(uv)
        .rounding(16.0)
        .paint_at(ui, rect);
}

/// The own-account profile uses only the library/friends already held by the app.
pub fn show(app: &mut AeAppEguiApp, ctx: &Context, open: &mut bool) {
    if !*open {
        return;
    }
    if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
        *open = false;
        return;
    }

    let viewport = ctx.screen_rect();
    let size = vec2(
        600.0_f32.min((viewport.width() - 32.0).max(280.0)),
        764.0_f32.min((viewport.height() - 32.0).max(380.0)),
    );
    let panel = Rect::from_center_size(viewport.center(), size);
    let backdrop = egui::Area::new(Id::new("aeapp-profile-backdrop"))
        .order(Order::Foreground)
        .fixed_pos(viewport.min)
        .movable(false)
        .constrain(false)
        .show(ctx, |ui| {
            ui.set_min_size(viewport.size());
            ui.allocate_rect(viewport, Sense::click_and_drag());
            ui.painter().rect_filled(viewport, 0.0, Color32::from_black_alpha(174));
        });

    let tab_key = Id::new("aeapp-profile-selected-tab");
    let mut tab = ctx.data(|data| data.get_temp::<ProfileTab>(tab_key)).unwrap_or_default();
    let mut close_requested = false;
    let mut selected_game = None;
    let profile = egui::Area::new(Id::new("aeapp-profile-modal"))
        .order(Order::Foreground)
        .fixed_pos(panel.min)
        .movable(false)
        .constrain(false)
        .show(ctx, |ui| {
            ui.set_min_size(size);
            ui.set_clip_rect(viewport);
            ui.painter().add(egui::Shape::Rect(
                egui::epaint::Shadow {
                    offset: vec2(0.0, 14.0),
                    blur: 48.0,
                    spread: 2.0,
                    color: Color32::from_black_alpha(100),
                }
                .as_shape(panel, Rounding::same(16.0)),
            ));
            ui.painter().rect_filled(panel, 16.0, PANEL);
            let origin = panel.min;
            let center_x = panel.center().x;

            let close_rect = Rect::from_min_size(
                pos2(panel.right() - 48.0, panel.top() + 16.0),
                vec2(32.0, 32.0),
            );
            let close = ui
                .put(close_rect, egui::Button::new("").frame(false).rounding(8.0))
                .on_hover_text("Close profile");
            if close.hovered() {
                ui.painter().rect_filled(close_rect, 8.0, Color32::from_white_alpha(13));
            }
            let center = close_rect.center();
            for (start, end) in [
                (vec2(-4.5, -4.5), vec2(4.5, 4.5)),
                (vec2(-4.5, 4.5), vec2(4.5, -4.5)),
            ] {
                ui.painter().line_segment([center + start, center + end], Stroke::new(1.6, TEXT));
            }
            close_requested |= close.clicked();

            let avatar_rect = Rect::from_min_size(pos2(center_x - 24.0, origin.y + 32.0), vec2(48.0, 48.0));
            if app.is_preview() || !app.user_id.is_empty() {
                shell::account_avatar(app, ui, avatar_rect);
            } else {
                ui.painter().rect_filled(avatar_rect, 12.0, shell::SURFACE_HOVER);
                shell::original_icon(ui, avatar_rect.shrink(10.0), "Avatar", MUTED);
            }
            let username = if app.is_preview() {
                "DemoPlayer"
            } else if app.user_name.trim().is_empty() {
                "Your profile"
            } else {
                &app.user_name
            };
            ui.painter().text(pos2(center_x, origin.y + 98.0), Align2::CENTER_TOP, username, shell::bold(14.0), TEXT);
            let nickname = Rect::from_center_size(pos2(center_x, origin.y + 126.0), vec2(150.0, 22.0));
            ui.add_enabled_ui(false, |ui| {
                let response = ui.put(nickname, egui::Button::new("").frame(false));
                response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, false, "Set a nickname"));
                response
            })
            .inner
            .on_disabled_hover_text("Nickname editing is not available in aeapp.");
            ui.painter().text(nickname.center(), Align2::CENTER_CENTER, "Set a nickname", FontId::proportional(12.0), BLUE);

            let edit = Rect::from_min_size(origin + vec2((size.x - 600.0) * 0.5 + 216.0, 145.0), vec2(158.0, 32.0));
            let edit_response = ui.add_enabled_ui(!app.is_preview(), |ui| {
                ui.put(edit, egui::Button::new(RichText::new("EDIT PROFILE").font(shell::bold(12.0)).color(TEXT))
                    .fill(Color32::TRANSPARENT)
                    .stroke(Stroke::new(1.0, Color32::from_rgb(97, 97, 115)))
                    .rounding(8.0))
            }).inner;
            if edit_response.clicked() {
                ctx.open_url(egui::OpenUrl::new_tab("https://myaccount.ea.com/"));
            }
            edit_response.on_disabled_hover_text("Profile editing requires an EA account; this is a local demo.");

            let inner_width = (size.x - 64.0).max(216.0);
            let tabs = Rect::from_min_size(origin + vec2(32.0, 201.0), vec2(inner_width, 36.0));
            ui.painter().rect_filled(tabs, 8.0, Color32::from_rgb(24, 24, 38));
            for (index, (value, label)) in [(ProfileTab::Games, "Games"), (ProfileTab::Friends, "Friends")].into_iter().enumerate() {
                let item = Rect::from_min_size(tabs.min + vec2(index as f32 * inner_width * 0.5, 0.0), vec2(inner_width * 0.5, 36.0));
                let response = ui.interact(item, ui.id().with(("profile-tab", index)), Sense::click());
                if response.clicked() {
                    tab = value;
                }
                if tab == value {
                    ui.painter().rect_filled(item.shrink(4.0), 5.0, Color32::from_rgb(52, 52, 72));
                } else if response.hovered() {
                    ui.painter().rect_filled(item.shrink(4.0), 5.0, Color32::from_white_alpha(5));
                }
                ui.painter().text(item.center(), Align2::CENTER_CENTER, label, shell::bold(13.0), if tab == value { TEXT } else { MUTED });
            }

            let (count, label) = match (tab, app.is_preview()) {
                (ProfileTab::Games, true) => (app.games.len(), "Sample games"),
                (ProfileTab::Games, false) => (app.games.len(), "Games"),
                (ProfileTab::Friends, true) => (app.friends.len(), "Sample friends"),
                (ProfileTab::Friends, false) => (app.friends.len(), "Friends"),
            };
            ui.painter().text(origin + vec2(32.0, 250.0), Align2::LEFT_TOP, format!("{label} ({count})"), FontId::proportional(12.0), MUTED);

            let body = Rect::from_min_max(origin + vec2(32.0, 273.0), panel.right_bottom() - vec2(16.0, 36.0));
            ui.allocate_ui_at_rect(body, |ui| {
                ui.set_clip_rect(body);
                ui.spacing_mut().scroll.floating = false;
                egui::ScrollArea::vertical()
                    .id_source(("aeapp-profile-grid", tab == ProfileTab::Friends))
                    .auto_shrink([false, false])
                    .max_height(body.height())
                    .min_scrolled_width(inner_width)
                    .show(ui, |ui| {
                        let start = ui.cursor().min;
                        let card_size = (inner_width - 48.0) / 4.0;
                        let row_height = card_size + if tab == ProfileTab::Games { 16.0 } else { 56.0 };
                        let rows = count.div_ceil(4);
                        let extent = vec2(inner_width, (rows as f32 * row_height - 16.0).max(body.height()));
                        ui.allocate_space(extent);
                        match tab {
                            ProfileTab::Games => {
                                let mut games: Vec<_> = app.games.values().collect();
                                games.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()).then_with(|| a.name.cmp(&b.name)).then_with(|| a.slug.cmp(&b.slug)));
                                for (index, game) in games.into_iter().enumerate() {
                                    let rect = Rect::from_min_size(start + vec2((index % 4) as f32 * (card_size + 16.0), (index / 4) as f32 * row_height), vec2(card_size, card_size));
                                    if !ui.is_rect_visible(rect) { continue; }
                                    cover_artwork(app, ui, rect, game);
                                    let response = ui.interact(rect, ui.id().with(("profile-game", &game.slug)), Sense::click()).on_hover_text(&game.name).on_hover_cursor(egui::CursorIcon::PointingHand);
                                    if response.hovered() {
                                        ui.painter().rect_stroke(rect, 16.0, Stroke::new(1.5, Color32::from_white_alpha(135)));
                                    }
                                    if response.clicked() { selected_game = Some(game.slug.clone()); }
                                }
                            }
                            ProfileTab::Friends => {
                                let mut friends: Vec<_> = app.friends.iter().collect();
                                friends.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()).then_with(|| a.name.cmp(&b.name)).then_with(|| a.id.cmp(&b.id)));
                                for (index, friend) in friends.into_iter().enumerate() {
                                    let rect = Rect::from_min_size(start + vec2((index % 4) as f32 * (card_size + 16.0), (index / 4) as f32 * row_height), vec2(card_size, card_size));
                                    if !ui.is_rect_visible(rect.expand2(vec2(0.0, 20.0))) { continue; }
                                    friend_avatar(app, ui, rect, &friend.id);
                                    let (status, color) = friend_status(&friend.online, friend.game.is_some());
                                    clipped_text(ui, Rect::from_min_size(rect.min + vec2(0.0, card_size + 8.0), vec2(card_size, 17.0)), &friend.name, shell::bold(12.0), TEXT);
                                    clipped_text(ui, Rect::from_min_size(rect.min + vec2(0.0, card_size + 26.0), vec2(card_size, 14.0)), status, FontId::proportional(10.0), color);
                                    ui.interact(rect, ui.id().with(("profile-friend", &friend.id)), Sense::hover()).on_hover_text(format!("{} · {status}\nFriend profile actions are not available in aeapp.", friend.name));
                                }
                            }
                        }
                        if count == 0 {
                            let message = if tab == ProfileTab::Games { "No games to show" } else { "No friends to show" };
                            ui.painter().text(start + vec2(inner_width * 0.5, 84.0), Align2::CENTER_CENTER, message, shell::bold(14.0), MUTED);
                        }
                    });
            });
            let note = if app.is_preview() {
                "Local demo · sample data · no EA account connected"
            } else if app.backend_state != BackendStallState::BingChilling {
                "No EA account connected"
            } else {
                ""
            };
            if !note.is_empty() {
                ui.painter().text(pos2(center_x, panel.bottom() - 17.0), Align2::CENTER_CENTER, note, FontId::proportional(10.0), MUTED);
            }
        });
    ctx.move_to_top(backdrop.response.layer_id);
    ctx.move_to_top(profile.response.layer_id);
    ctx.set_sublayer(backdrop.response.layer_id, profile.response.layer_id);
    ctx.data_mut(|data| data.insert_temp(tab_key, tab));
    if let Some(slug) = selected_game {
        app.game_sel = slug;
        app.page_view = PageType::Games;
        app.library_details = true;
        close_requested = true;
    }
    if close_requested {
        *open = false;
    }
}
