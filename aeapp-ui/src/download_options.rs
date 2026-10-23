use crate::{
    bridge_thread::{AeAppLibRequest, InteractThreadLocateGameResponse},
    shell, AeAppEguiApp, FrontendSettings, GameInfo,
};
use aeapp::content::install::InstallSpace;
use egui::{
    pos2, vec2, Align2, Color32, Context, FontId, Id, Order, Rect, RichText, Sense, Stroke,
};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

static CHECK_ID: AtomicU64 = AtomicU64::new(1);
const PANEL: Color32 = Color32::from_rgb(30, 30, 48);
const ERROR: Color32 = Color32::from_rgb(255, 71, 76);

pub struct InstallModalState {
    pub(crate) locate_path: String,
    pub(crate) install_folder: String,
    pub(crate) locating: bool,
    pub(crate) locate_response: Option<InteractThreadLocateGameResponse>,
    check_id: u64,
    checked_offer: Option<String>,
    checked_path: Option<PathBuf>,
    result: Option<Result<InstallSpace, String>>,
    enqueue_id: Option<u64>,
    enqueue_error: Option<String>,
    display_game: Option<GameInfo>,
    editing: bool,
    locate_mode: bool,
}
impl InstallModalState {
    pub fn new(settings: &FrontendSettings) -> Self {
        Self {
            locate_path: String::new(),
            install_folder: settings.default_install_folder.clone(),
            locating: false,
            locate_response: None,
            check_id: 0,
            checked_offer: None,
            checked_path: None,
            result: None,
            enqueue_id: None,
            enqueue_error: None,
            display_game: None,
            editing: false,
            locate_mode: false,
        }
    }
    pub fn for_game(settings: &FrontendSettings, slug: &str) -> Self {
        let mut state = Self::new(settings);
        let base = if state.install_folder.trim().is_empty() {
            std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default().join("Games/EA")
        } else {
            PathBuf::from(&state.install_folder)
        };
        state.install_folder = base.join(slug).to_string_lossy().into_owned();
        state
    }
    pub(crate) fn apply_result(
        &mut self,
        id: u64,
        path: PathBuf,
        result: Result<InstallSpace, String>,
    ) {
        if id != 0
            && id == self.check_id
            && self.checked_offer.is_some()
            && self.checked_path.as_ref() == Some(&path)
            && PathBuf::from(self.install_folder.trim()) == path
        {
            self.result = Some(result);
        }
    }
    fn invalidate_check(&mut self) {
        self.check_id = 0;
        self.checked_offer = None;
        self.checked_path = None;
        self.result = None;
        self.enqueue_error = None;
    }
    fn needs_check(&self, offer: &str, path: &PathBuf) -> bool {
        self.checked_offer.as_deref() != Some(offer) || self.checked_path.as_ref() != Some(path)
    }
    fn can_start(&self, offer: &str, path: &PathBuf) -> bool {
        !self.editing
            && !self.locate_mode
            && !self.locating
            && !self.is_enqueuing()
            && !self.needs_check(offer, path)
            && matches!(&self.result, Some(Ok(space))
                if space.required > 0 && !space.build_id.trim().is_empty()
                    && space.available >= space.required)
    }
    pub(crate) fn is_enqueuing(&self) -> bool {
        self.enqueue_id.is_some()
    }
    pub(crate) fn backend_unavailable(&mut self, reason: &str) {
        if self.enqueue_id.take().is_some() {
            self.enqueue_error = Some(reason.to_owned());
        }
        if self.checked_path.is_some() && self.result.is_none() {
            self.check_id = 0;
            self.result = Some(Err(reason.to_owned()));
        }
    }
    fn begin_enqueue(&mut self) -> Option<u64> {
        if self.is_enqueuing() {
            return None;
        }
        let id = CHECK_ID.fetch_add(1, Ordering::Relaxed);
        self.enqueue_id = Some(id);
        self.enqueue_error = None;
        Some(id)
    }
    pub(crate) fn apply_enqueue_result(
        &mut self,
        id: u64,
        result: Result<(), String>,
    ) -> Option<bool> {
        if self.enqueue_id != Some(id) {
            return None;
        }
        self.enqueue_id = None;
        match result {
            Ok(()) => Some(true),
            Err(reason) => {
                self.enqueue_error = Some(reason);
                Some(false)
            }
        }
    }
    pub(crate) fn seed_preview(&mut self, state: &str) {
        self.check_id = 0;
        self.checked_offer = None;
        self.checked_path = Some(PathBuf::from(self.install_folder.trim()));
        self.editing = false;
        self.locate_mode = false;
        self.locating = false;
        self.locate_response = None;
        // Frozen fixtures never receive an acknowledgment from the disconnected backend.
        self.enqueue_id = (state == "enqueuing").then_some(u64::MAX);
        self.enqueue_error = (state == "enqueue-error").then(|| {
            "Local preview: the download queue could not be saved. No download was queued.".into()
        });
        self.result = match state {
            "calculating" => None,
            "error" => Some(Err("The archive size could not be read. Try again.".into())),
            _ => Some(Ok(InstallSpace {
                build_id: "preview-only".into(),
                required: 78_310_000_000,
                available: if state == "insufficient" {
                    11_360_000_000
                } else {
                    120_000_000_000
                },
            })),
        };
    }
}

fn text(ui: &egui::Ui, rect: Rect, value: &str, font: FontId, color: Color32) {
    let mut job = egui::text::LayoutJob::simple(value.to_owned(), font, color, rect.width());
    job.wrap.max_rows = 2;
    job.wrap.overflow_character = Some('…');
    let galley = ui.fonts(|fonts| fonts.layout_job(job));
    ui.painter().with_clip_rect(rect).galley(rect.min, galley, color);
}
fn button(ui: &mut egui::Ui, rect: Rect, label: &str, enabled: bool) -> egui::Response {
    let primary = label == "NEXT";
    let fill = if primary {
        if enabled {
            crate::ACCENT_COLOR
        } else {
            shell::SURFACE_HOVER
        }
    } else {
        Color32::TRANSPARENT
    };
    let border = if label.starts_with("CHANGE") || label == "DONE" {
        shell::MUTED
    } else {
        crate::ACCENT_COLOR
    };
    ui.add_enabled_ui(enabled, |ui| {
        ui.put(
            rect,
            egui::Button::new(RichText::new(label).font(shell::bold(12.0)))
                .fill(fill)
                .stroke(if primary {
                    Stroke::NONE
                } else {
                    Stroke::new(2.0, border)
                })
                .rounding(7.0),
        )
    })
    .inner
}

pub fn show(app: &mut AeAppEguiApp, ctx: &Context, slug: &str) {
    let live_game = app.games.get(slug).cloned();
    let live = live_game.is_some();
    // Library refresh may clear the live map while the queue is being saved.
    // Keep that pending modal visible; its snapshot never authorizes a request.
    let Some(game) = live_game.or_else(|| {
        app.installer_state.display_game.as_ref().filter(|game| game.slug == slug).cloned()
    }) else {
        if !app.installer_state.is_enqueuing() {
            app.modal = None;
        }
        return;
    };
    let preview = app.is_preview();
    let supported = app.args.allow_bf3 || !matches!(slug, "battlefield-3" | "battlefield-4");
    let state = &mut app.installer_state;
    let path = PathBuf::from(state.install_folder.trim());
    if !preview
        && live
        && !state.editing
        && !state.locate_mode
        && !state.is_enqueuing()
        && supported
        && state.needs_check(&game.offer, &path)
    {
        state.check_id = CHECK_ID.fetch_add(1, Ordering::Relaxed);
        state.checked_offer = Some(game.offer.clone());
        state.checked_path = Some(path.clone());
        state.result = None;
        if app
            .backend
            .backend_commander
            .send(AeAppLibRequest::PrepareInstallRequest(
                state.check_id,
                game.offer.clone(),
                path,
            ))
            .is_err()
        {
            state.result = Some(Err("The download service is not available.".into()));
        }
    }
    let screen = ctx.screen_rect();
    let size = vec2(
        600.0_f32.min(screen.width() - 32.0),
        484.0_f32.min(screen.height() - 32.0),
    );
    let panel = Rect::from_center_size(screen.center(), size);
    let backdrop = egui::Area::new(Id::new("aeapp-download-backdrop"))
        .order(Order::Foreground)
        .fixed_pos(screen.min)
        .movable(false)
        .constrain(false)
        .show(ctx, |ui| {
            ui.set_min_size(screen.size());
            ui.allocate_rect(screen, Sense::click_and_drag());
            ui.painter().rect_filled(screen, 0.0, Color32::from_black_alpha(32));
        });
    let mut close = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
    let mut start = false;
    let mut open_downloads = false;
    let already_queued = app.install_queue.contains_key(&game.offer)
        || app.installing_now.as_ref().is_some_and(|current| current.offer == game.offer);
    let modal = egui::Area::new(Id::new("aeapp-download-options")).order(Order::Foreground)
        .fixed_pos(panel.min).movable(false).constrain(false).show(ctx, |ui| {
            ui.set_min_size(size);
            ui.spacing_mut().interact_size = vec2(0.0, 0.0);
            ui.spacing_mut().button_padding = vec2(4.0, 2.0);
            ui.set_clip_rect(screen);
            ui.painter().rect_filled(panel, 16.0, PANEL);
            let at = panel.min;
            let width = size.x - 48.0;
            let field = |y, height| Rect::from_min_size(at + vec2(24.0, y), vec2(width, height));
            crate::views::game_view::cover_artwork(app, ui, Rect::from_min_size(at + vec2(24.0, 24.0), vec2(48.0, 48.0)), &game);
            text(ui, Rect::from_min_size(at+vec2(84.0, 30.0), vec2(width-100.0, 18.0)), &game.name, shell::bold(12.0), shell::MUTED);
            text(ui, Rect::from_min_size(at+vec2(84.0, 51.0), vec2(width-100.0, 22.0)), "Download options", shell::bold(16.0), shell::TEXT);
            let x = Rect::from_min_size(pos2(panel.right()-40.0, panel.top()+16.0), vec2(24.0,24.0));
            let busy = app.installer_state.is_enqueuing();
            close |= button(ui, x, "×", !app.installer_state.locating && !busy).clicked();
            if !supported {
                text(ui, field(118.0, 110.0), "Battlefield 3 and 4 downloads are not available because their Battlelog launch flow is not supported yet.", FontId::proportional(15.0), shell::TEXT);
            } else if app.installer_state.locate_mode {
                text(ui, field(118.0, 28.0), "LOCATE INSTALLED GAME", shell::bold(12.0), shell::MUTED);
                let changed = ui.add_enabled_ui(!app.installer_state.locating, |ui| {
                    ui.put(field(155.0, 40.0), egui::TextEdit::singleline(&mut app.installer_state.locate_path).hint_text("Existing game folder"))
                }).inner.changed();
                if changed { app.installer_state.locate_response = None; }
                let valid = PathBuf::from(&app.installer_state.locate_path).is_absolute();
                if button(ui, field(215.0, 32.0), "LOCATE GAME", valid && live && !preview && !app.installer_state.locating).clicked() {
                    if app.backend.backend_commander.send(AeAppLibRequest::LocateGameRequest(app.installer_state.locate_path.clone())).is_ok() {
                        app.installer_state.locating = true;
                        app.installer_state.locate_response = None;
                    } else {
                        app.nonfatal_errors.push(crate::bridge_thread::BackendError::ChannelDisconnected);
                    }
                }
                if app.installer_state.locating {
                    text(ui, field(275.0, 40.0), "Locating installed game…", FontId::proportional(14.0), shell::MUTED);
                    ctx.request_repaint_after(std::time::Duration::from_millis(100));
                }
                match &app.installer_state.locate_response {
                    Some(InteractThreadLocateGameResponse::Success) => {
                        // The manifest may belong to a different game. Read the real
                        // library state instead of marking the selected offer installed.
                        if app.backend.backend_commander.send(AeAppLibRequest::RefreshGamesRequest).is_err() {
                            app.nonfatal_errors.push(crate::bridge_thread::BackendError::ChannelDisconnected);
                        }
                        close = true;
                    }
                    Some(InteractThreadLocateGameResponse::Error(_)) => text(ui, field(275.0, 55.0), "The game could not be located. Check the folder and installer manifest, then try again.", FontId::proportional(14.0), ERROR),
                    None => {}
                }
            } else {
                text(ui, field(118.0, 20.0), "INSTALL LOCATION", shell::bold(12.0), shell::MUTED);
                ui.painter().hline(field(143.0, 0.0).x_range(), panel.top()+143.0, Stroke::new(2.0, shell::BORDER));
                let edit = app.installer_state.editing;
                let change = Rect::from_min_size(pos2(panel.right()-142.0, panel.top()+153.0), vec2(118.0,32.0));
                if button(ui, change, if edit { "DONE" } else { "CHANGE" }, live && !preview && !busy).clicked() {
                    app.installer_state.editing = !edit;
                    app.installer_state.invalidate_check();
                }
                let folder = Rect::from_min_size(at+vec2(24.0,154.0), vec2(width-132.0,35.0));
                if edit {
                    ui.put(folder, egui::TextEdit::singleline(&mut app.installer_state.install_folder));
                } else {
                    text(ui, folder.shrink2(vec2(0.0,7.0)), &app.installer_state.install_folder, FontId::proportional(14.0), shell::TEXT);
                    ui.interact(folder, ui.id().with("install-path"), Sense::hover()).on_hover_text(&app.installer_state.install_folder);
                }
                text(ui, field(205.0,20.0), "LANGUAGE", shell::bold(12.0), shell::MUTED);
                ui.painter().hline(field(228.0,0.0).x_range(), panel.top()+228.0, Stroke::new(2.0,shell::BORDER));
                let language = field(238.0,40.0);
                ui.painter().rect(language,8.0,shell::SIDEBAR,Stroke::new(2.0,shell::MUTED));
                ui.painter().text(language.min+vec2(14.0,12.0),Align2::LEFT_TOP,"English (installer default)",FontId::proportional(14.0),shell::MUTED);
                shell::original_icon(ui,Rect::from_center_size(language.right_center()-vec2(20.0,0.0),vec2(16.0,16.0)),"CHEVRON_DOWN",shell::MUTED);
                ui.interact(language,ui.id().with("install-language"),Sense::hover()).on_hover_text("The current Windows installer uses English. Other languages are not available yet.");
                for (y,label) in [(306.0,"Create desktop shortcut"),(342.0,"Update games automatically")] {
                    let box_rect = Rect::from_min_size(at+vec2(24.0,y),vec2(24.0,24.0));
                    ui.painter().rect(box_rect,2.0,shell::SIDEBAR,Stroke::new(2.0,shell::MUTED));
                    text(ui,Rect::from_min_size(at+vec2(60.0,y+3.0),vec2(width-36.0,24.0)),label,shell::bold(15.0),shell::MUTED);
                    ui.interact(field(y,26.0),ui.id().with(label),Sense::hover()).on_hover_text("This install option is not available yet.");
                }
            }
            if supported && live && !preview && !app.installer_state.locating && !busy {
                let label = if app.installer_state.locate_mode { "Back to download options" } else { "Locate installed game" };
                if ui.put(field(379.0,22.0),egui::Button::new(RichText::new(label).color(crate::ACCENT_COLOR).size(12.0)).frame(false)).clicked() {
                    app.installer_state.locate_mode = !app.installer_state.locate_mode;
                    app.installer_state.invalidate_check();
                }
            }
            let footer = panel.bottom()-60.0;
            let cancel = Rect::from_min_size(pos2(panel.right()-116.0,footer),vec2(92.0,32.0));
            close |= button(ui,cancel,"CANCEL",!app.installer_state.locating && !busy).clicked();
            if supported && !app.installer_state.locate_mode {
                let ready = live && app.installer_state.can_start(&game.offer, &PathBuf::from(app.installer_state.install_folder.trim()));
                let retry = !app.installer_state.editing && matches!(&app.installer_state.result,
                    Some(Err(_)) | Some(Ok(InstallSpace { required: 0, .. })));
                let recheck = !app.installer_state.editing && matches!(&app.installer_state.result,
                    Some(Ok(space)) if space.available < space.required || space.build_id.trim().is_empty());
                let next = Rect::from_min_size(pos2(panel.right()-198.0,footer),vec2(69.0,32.0));
                if already_queued && !busy {
                    // Preparation can fail after the queue is saved. Manage that
                    // existing entry rather than enqueueing the same offer twice.
                    open_downloads = button(ui,next,"OPEN",!preview).on_hover_text("Open the existing download to resume or retry it").clicked();
                } else if retry || recheck {
                    if button(ui,next,if retry { "RETRY" } else { "CHECK" },live && !preview && !busy).on_hover_text("Recalculate download size and available disk space").clicked() {
                        app.installer_state.invalidate_check();
                    }
                } else {
                    start = button(ui,next,if app.installer_state.enqueue_error.is_some() { "RETRY" } else { "NEXT" },ready && !preview).on_hover_text("Start download or add it to the queue").clicked();
                }
                let status = Rect::from_min_size(pos2(panel.left()+50.0,footer-1.0),vec2((width-224.0).max(100.0),48.0));
                if busy {
                    ui.put(Rect::from_min_size(pos2(panel.left()+25.0,footer+4.0),vec2(24.0,24.0)),egui::Spinner::new().size(22.0));
                    text(ui,status.shrink2(vec2(0.0,8.0)),"Adding download to queue…",FontId::proportional(12.0),shell::MUTED);
                    ctx.request_repaint_after(std::time::Duration::from_millis(100));
                } else if let Some(reason) = &app.installer_state.enqueue_error {
                    text(ui,status,reason,FontId::proportional(12.0),ERROR);
                    ui.interact(status,ui.id().with("enqueue-error"),Sense::hover()).on_hover_text(reason);
                } else if app.installer_state.editing {
                    text(ui,status,"Choose an installation folder, then select Done to check its disk space.",FontId::proportional(12.0),shell::MUTED);
                } else { match &app.installer_state.result {
                    Some(Ok(space)) if space.required == 0 || space.build_id.trim().is_empty() => {
                        text(ui,status,"Download size unavailable. Retry before starting the download.",FontId::proportional(12.0),ERROR);
                    }
                    Some(Ok(space)) => {
                        let insufficient = space.available < space.required;
                        text(ui,status,&format!("{:.2} GB  disk space required",space.required as f64/1e9),FontId::proportional(12.0),if insufficient { ERROR } else { shell::MUTED });
                        text(ui,status.translate(vec2(0.0,20.0)),&format!("{:.2} GB  available",space.available as f64/1e9),FontId::proportional(12.0),shell::MUTED);
                        if insufficient { ui.painter().text(pos2(panel.left()+25.0,footer+8.0),Align2::LEFT_TOP,"⚠",FontId::proportional(14.0),ERROR); }
                        ui.interact(status,ui.id().with("size-estimate"),Sense::hover()).on_hover_text("Full archive size estimate. Proton setup and existing downloads may need additional space.");
                    }
                    Some(Err(reason)) => {
                        text(ui,status,reason,FontId::proportional(12.0),ERROR);
                        ui.interact(status,ui.id().with("preflight-error"),Sense::hover()).on_hover_text(reason);
                    }
                    None => {
                        ui.put(Rect::from_min_size(pos2(panel.left()+25.0,footer+4.0),vec2(24.0,24.0)),egui::Spinner::new().size(22.0));
                        text(ui,status.shrink2(vec2(0.0,8.0)),"Calculating disk space…",FontId::proportional(12.0),shell::MUTED);
                        ctx.request_repaint_after(std::time::Duration::from_millis(100));
                    }
                }}
            }
        });
    ctx.move_to_top(backdrop.response.layer_id);
    ctx.move_to_top(modal.response.layer_id);
    if open_downloads {
        app.page_view = crate::PageType::Downloads;
        app.library_details = false;
        app.modal = None;
    }
    if start && !already_queued {
        let path = PathBuf::from(app.installer_state.install_folder.trim());
        if let Some(id) = app.installer_state.begin_enqueue() {
            app.installer_state.display_game = Some(game.clone());
            if app
                .backend
                .backend_commander
                .send(AeAppLibRequest::InstallGameRequest(id, game.offer, path))
                .is_err()
            {
                app.installer_state.apply_enqueue_result(
                    id,
                    Err(
                        "The download service is not available. The download was not queued."
                            .into(),
                    ),
                );
            }
        }
    }
    if close && !app.installer_state.locating && !app.installer_state.is_enqueuing() {
        app.modal = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stale_disk_space_results_cannot_enable_a_changed_destination() {
        let mut state = InstallModalState::new(&FrontendSettings::new());
        state.install_folder = "/games/new".into();
        state.check_id = 2;
        state.checked_offer = Some("offer".into());
        state.checked_path = Some("/games/new".into());
        let size = InstallSpace {
            build_id: "build".into(),
            required: 10,
            available: 20,
        };
        state.apply_result(1, "/games/new".into(), Ok(size.clone()));
        assert!(state.result.is_none());
        state.apply_result(2, "/games/old".into(), Ok(size.clone()));
        assert!(state.result.is_none());
        state.apply_result(2, "/games/new".into(), Ok(size));
        assert!(state.result.is_some());
        state.result = None;
        state.install_folder = "/games/other".into();
        state.apply_result(2, "/games/new".into(), Err("old failure".into()));
        assert!(state.result.is_none());
    }
    #[test]
    fn a_valid_estimate_cannot_enable_another_offer_at_the_same_path() {
        let mut state = InstallModalState::new(&FrontendSettings::new());
        let path = PathBuf::from("/games/shared");
        state.install_folder = path.to_string_lossy().into_owned();
        state.check_id = 7;
        state.checked_offer = Some("first-offer".into());
        state.checked_path = Some(path.clone());
        state.apply_result(
            7,
            path.clone(),
            Ok(InstallSpace {
                build_id: "build".into(),
                required: 10,
                available: 20,
            }),
        );
        assert!(state.can_start("first-offer", &path));
        assert!(state.needs_check("other-offer", &path));
        assert!(!state.can_start("other-offer", &path));
    }
    #[test]
    fn retry_or_path_edit_rejects_late_success_until_a_new_check_finishes() {
        let mut state = InstallModalState::new(&FrontendSettings::new());
        let path = PathBuf::from("/games/game");
        state.install_folder = path.to_string_lossy().into_owned();
        state.check_id = 9;
        state.checked_offer = Some("offer".into());
        state.checked_path = Some(path.clone());
        state.result = Some(Err("transient failure".into()));
        state.invalidate_check();
        state.apply_result(
            9,
            path.clone(),
            Ok(InstallSpace {
                build_id: "build".into(),
                required: 10,
                available: 20,
            }),
        );
        assert!(state.result.is_none());
        assert!(!state.can_start("offer", &path));
        assert!(state.needs_check("offer", &path));
    }
    #[test]
    fn missing_size_build_or_space_never_enables_installation() {
        let mut state = InstallModalState::new(&FrontendSettings::new());
        let path = PathBuf::from("/games/game");
        state.checked_offer = Some("offer".into());
        state.checked_path = Some(path.clone());
        for (build, required, available) in [("build", 0, 20), ("", 10, 20), ("build", 20, 10)] {
            state.result = Some(Ok(InstallSpace {
                build_id: build.into(),
                required,
                available,
            }));
            assert!(!state.can_start("offer", &path));
        }
        state.result = Some(Ok(InstallSpace {
            build_id: "build".into(),
            required: 10,
            available: 20,
        }));
        state.editing = true;
        assert!(!state.can_start("offer", &path));
        state.editing = false;
        state.locate_mode = true;
        assert!(!state.can_start("offer", &path));
    }
    #[test]
    fn enqueue_waits_for_matching_ack_and_keeps_failure_retryable() {
        let mut state = InstallModalState::new(&FrontendSettings::new());
        let first = state.begin_enqueue().unwrap();
        assert!(state.is_enqueuing());
        assert!(state.begin_enqueue().is_none());
        assert_eq!(
            state.apply_enqueue_result(first.wrapping_add(1), Ok(())),
            None
        );
        assert!(state.is_enqueuing());
        assert_eq!(
            state.apply_enqueue_result(first, Err("queue could not be saved".into())),
            Some(false)
        );
        assert!(!state.is_enqueuing());
        assert_eq!(
            state.enqueue_error.as_deref(),
            Some("queue could not be saved")
        );
        let retry = state.begin_enqueue().unwrap();
        assert_ne!(retry, first);
        assert!(state.enqueue_error.is_none());
        assert_eq!(state.apply_enqueue_result(first, Ok(())), None);
        assert!(state.is_enqueuing());
        assert_eq!(state.apply_enqueue_result(retry, Ok(())), Some(true));
        assert!(!state.is_enqueuing());
    }
    #[test]
    fn backend_failure_releases_enqueue_and_rejects_its_late_success() {
        let mut state = InstallModalState::new(&FrontendSettings::new());
        let id = state.begin_enqueue().unwrap();
        state.backend_unavailable("The download service stopped.");
        assert!(!state.is_enqueuing());
        assert_eq!(state.apply_enqueue_result(id, Ok(())), None);
        assert_eq!(
            state.enqueue_error.as_deref(),
            Some("The download service stopped.")
        );
    }
}
