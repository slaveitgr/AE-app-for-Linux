use crate::{
    bridge_thread::{AeAppLibRequest, BackendError},
    shell, AeAppEguiApp, ACCENT_COLOR,
};
use aeapp::content::manager::{DownloadAction, DownloadPhase, DownloadStatus};
use egui::{pos2, vec2, Align2, Color32, FontId, Rect, RichText, Stroke, Ui};
use humansize::DECIMAL;
use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

static CONTROL_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone)]
struct PendingControl {
    id: u64,
    offer: String,
    label: &'static str,
    started: Instant,
}
#[derive(Clone, Default)]
struct ControlState {
    pending: Option<PendingControl>,
    error: Option<String>,
}
impl ControlState {
    fn complete(&mut self, id: u64, result: Result<(), String>) {
        // An old response must never release a newer command's busy state.
        if self.pending.as_ref().is_some_and(|pending| pending.id == id) {
            self.pending = None;
            self.error = result.err();
        }
    }
    fn backend_unavailable(&mut self, reason: &str) {
        if self.pending.take().is_some() {
            self.error = Some(reason.to_owned());
        }
    }
}
fn control_key() -> egui::Id {
    egui::Id::new("ea-download-control")
}
fn control_state(ctx: &egui::Context) -> ControlState {
    ctx.data(|data| data.get_temp::<ControlState>(control_key())).unwrap_or_default()
}
pub fn apply_control_result(ctx: &egui::Context, id: u64, result: Result<(), String>) {
    let mut state = control_state(ctx);
    state.complete(id, result);
    ctx.data_mut(|data| data.insert_temp(control_key(), state));
    ctx.request_repaint();
}
pub fn backend_unavailable(ctx: &egui::Context, reason: &str) {
    let mut state = control_state(ctx);
    state.backend_unavailable(reason);
    ctx.data_mut(|data| data.insert_temp(control_key(), state));
    ctx.request_repaint();
}

#[derive(Clone)]
pub struct QueuedDownload {
    pub slug: String,
    pub offer: String,
    pub downloaded_bytes: usize,
    pub total_bytes: usize,
}
pub fn status_key() -> egui::Id {
    egui::Id::new("ea-download-status")
}
pub fn order_key() -> egui::Id {
    egui::Id::new("ea-download-order")
}
pub fn apply_status(ctx: &egui::Context, status: DownloadStatus) {
    ctx.data_mut(|data| data.insert_temp(status_key(), status));
}
/// Used only by the isolated preview; never starts a transfer.
pub fn seed_preview_status(ctx: &egui::Context, offer_id: String) {
    apply_status(
        ctx,
        DownloadStatus {
            offer_id: Some(offer_id),
            phase: DownloadPhase::Downloading,
            bytes: 512_000_000,
            total: 2_048_000_000,
            error: None,
        },
    );
}
fn fraction(bytes: usize, total: usize) -> Option<f32> {
    if total == 0 {
        None
    } else {
        Some((bytes as f64 / total as f64).clamp(0.0, 1.0) as f32)
    }
}
fn resolve_status(
    current: &QueuedDownload,
    cached: Option<DownloadStatus>,
) -> (DownloadStatus, bool) {
    let matching =
        cached.filter(|status| status.offer_id.as_deref() == Some(current.offer.as_str()));
    let confirmed = matching.is_some();
    let mut status = matching.unwrap_or_else(|| DownloadStatus {
        offer_id: Some(current.offer.clone()),
        ..DownloadStatus::default()
    });
    // Progress notifications are more frequent than phase snapshots. Both bridge
    // responses update these counters, so they also reset correctly on Retry.
    status.bytes = current.downloaded_bytes;
    status.total = current.total_bytes;
    (status, confirmed)
}
fn current_status(ctx: &egui::Context, current: &QueuedDownload) -> (DownloadStatus, bool) {
    resolve_status(
        current,
        ctx.data(|data| data.get_temp::<DownloadStatus>(status_key())),
    )
}
fn primary_control(phase: DownloadPhase, confirmed: bool) -> Option<&'static str> {
    if !confirmed {
        return None;
    }
    match phase {
        DownloadPhase::Downloading => Some("Pause"),
        DownloadPhase::Paused => Some("Resume"),
        DownloadPhase::Failed | DownloadPhase::Cancelled => Some("Retry"),
        _ => None,
    }
}
fn can_cancel(phase: DownloadPhase, confirmed: bool) -> bool {
    confirmed
        && matches!(
            phase,
            DownloadPhase::Downloading
                | DownloadPhase::Paused
                | DownloadPhase::Failed
                | DownloadPhase::Cancelled
        )
}
fn pending_label(ctx: &egui::Context, offer: &str) -> Option<&'static str> {
    control_state(ctx)
        .pending
        .filter(|pending| pending.offer == offer)
        .map(|pending| pending.label)
}
fn progress_text(status: &DownloadStatus) -> String {
    if let Some(amount) = fraction(status.bytes, status.total) {
        format!(
            "{} / {}  ·  {:.0}%",
            humansize::SizeFormatter::new(status.bytes.min(status.total), DECIMAL),
            humansize::SizeFormatter::new(status.total, DECIMAL),
            amount * 100.0
        )
    } else if status.bytes > 0 {
        format!(
            "{} downloaded · total size unavailable",
            humansize::SizeFormatter::new(status.bytes, DECIMAL)
        )
    } else {
        match status.phase {
            DownloadPhase::Idle => "Waiting for download information…",
            DownloadPhase::Finalizing => "Downloaded files are being installed.",
            DownloadPhase::Finished => "Installation completed.",
            _ => "Download size unavailable.",
        }
        .to_owned()
    }
}

/// Always visible in the Windows-style sidebar, independently of the current page.
/// Reads the same status as the full download manager and never controls a transfer.
pub fn sidebar_downloads(app: &AeAppEguiApp, ui: &mut Ui, rect: Rect) -> bool {
    let current = app.installing_now.as_ref();
    let status = current.map(|download| current_status(ui.ctx(), download).0);
    let queued = app
        .install_queue
        .values()
        .filter(|item| current.is_none_or(|current| current.offer != item.offer))
        .count();
    let game = current.and_then(|download| {
        app.games
            .get(&download.slug)
            .or_else(|| app.games.values().find(|game| game.offer == download.offer))
    });
    let title = game.map(|game| game.name.as_str()).unwrap_or("Game download");
    let progress = status.as_ref().and_then(|status| fraction(status.bytes, status.total));
    let state = if let Some(pending) =
        current.and_then(|download| pending_label(ui.ctx(), &download.offer))
    {
        pending.to_owned()
    } else if let Some(status) = &status {
        if let Some(progress) = progress {
            format!("{} · {:.0}%", phase_text(status.phase), progress * 100.0)
        } else {
            phase_text(status.phase).to_owned()
        }
    } else if queued > 0 {
        format!("{queued} queued")
    } else {
        "No active downloads".to_owned()
    };
    let selected = app.page_view == crate::PageType::Downloads;
    let response = ui.interact(
        rect,
        ui.id().with("sidebar-download-manager"),
        egui::Sense::click(),
    );
    response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Button,
            ui.is_enabled(),
            format!("Download manager: {state}"),
        )
    });
    if selected || response.hovered() {
        ui.painter().rect_filled(
            rect,
            0.0,
            if selected {
                shell::SURFACE_HOVER
            } else {
                shell::SURFACE
            },
        );
    }
    ui.painter().line_segment(
        [
            rect.left_top() + vec2(12.0, 0.0),
            rect.right_top() - vec2(12.0, 0.0),
        ],
        Stroke::new(1.0, shell::BORDER),
    );
    let center = pos2(rect.left() + 36.0, rect.center().y);
    let phase = status.as_ref().map(|status| status.phase);
    let color = match phase {
        Some(DownloadPhase::Paused) => shell::MUTED,
        Some(DownloadPhase::Failed) => Color32::from_rgb(255, 126, 126),
        Some(DownloadPhase::Finalizing | DownloadPhase::Finished) => shell::GREEN,
        _ => ACCENT_COLOR,
    };
    if current.is_some() || queued > 0 {
        ui.painter().circle_stroke(center, 20.0, Stroke::new(2.5, shell::BORDER));
        let spinning = progress.is_none()
            && matches!(
                phase,
                Some(DownloadPhase::Idle | DownloadPhase::Downloading | DownloadPhase::Finalizing)
            );
        let amount = if spinning {
            0.23
        } else {
            progress.unwrap_or(0.0)
        };
        if amount > 0.0 {
            let start = -std::f32::consts::FRAC_PI_2
                + if spinning {
                    ui.input(|input| input.time as f32) * 2.0
                } else {
                    0.0
                };
            let steps = (64.0 * amount).ceil().max(2.0) as usize;
            let points = (0..=steps)
                .map(|step| {
                    let angle = start + std::f32::consts::TAU * amount * step as f32 / steps as f32;
                    center + vec2(angle.cos(), angle.sin()) * 20.0
                })
                .collect();
            ui.painter().add(egui::Shape::line(points, Stroke::new(2.5, color)));
        }
        if spinning {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
        }
    }
    let icon = Rect::from_center_size(center, vec2(24.0, 24.0));
    let paused = phase == Some(DownloadPhase::Paused);
    if !shell::original_icon(
        ui,
        icon,
        if paused { "Pause" } else { "DOWNLOAD_QUEUE" },
        shell::TEXT,
    ) {
        if paused {
            for x in [-4.0, 4.0] {
                ui.painter().line_segment(
                    [center + vec2(x, -6.0), center + vec2(x, 6.0)],
                    Stroke::new(2.0, shell::TEXT),
                );
            }
        } else {
            let line = Stroke::new(1.8, shell::TEXT);
            ui.painter().line_segment([center + vec2(0.0, -7.0), center + vec2(0.0, 5.0)], line);
            ui.painter().line_segment([center + vec2(-5.0, 0.0), center + vec2(0.0, 5.0)], line);
            ui.painter().line_segment([center + vec2(0.0, 5.0), center + vec2(5.0, 0.0)], line);
            ui.painter().line_segment([center + vec2(-7.0, 9.0), center + vec2(7.0, 9.0)], line);
        }
    }
    let x = rect.left() + 72.0;
    let width = rect.width() - 84.0;
    let y = if current.is_some() {
        rect.top() + 17.0
    } else {
        rect.top() + 28.0
    };
    let heading = ui.painter().layout_no_wrap(
        "Download manager".to_owned(),
        shell::bold(13.0),
        shell::TEXT,
    );
    ui.painter().galley(pos2(x, y), heading, shell::TEXT);
    if current.is_some() {
        text(
            ui,
            Rect::from_min_size(pos2(x, y + 23.0), vec2(width, 18.0)),
            title,
            12.0,
            shell::TEXT,
            1,
        );
    }
    text(
        ui,
        Rect::from_min_size(
            pos2(
                x,
                if current.is_some() {
                    rect.bottom() - 29.0
                } else {
                    rect.top() + 51.0
                },
            ),
            vec2(width, 18.0),
        ),
        &state,
        11.0,
        if phase == Some(DownloadPhase::Failed) {
            color
        } else {
            shell::MUTED
        },
        1,
    );
    if queued > 0 && current.is_some() {
        let badge = pos2(rect.right() - 20.0, rect.top() + 23.0);
        ui.painter().circle_filled(badge, 9.0, ACCENT_COLOR);
        ui.painter().text(
            badge,
            Align2::CENTER_CENTER,
            if queued > 9 {
                "9+".to_owned()
            } else {
                queued.to_string()
            },
            shell::bold(10.0),
            Color32::WHITE,
        );
    }
    let hover = if current.is_some() {
        format!("{title}\n{state}\n{queued} queued\nOpen download manager")
    } else {
        format!("{state}\nOpen download manager")
    };
    response.on_hover_text(hover).clicked()
}
fn text(ui: &Ui, rect: Rect, value: &str, size: f32, color: Color32, rows: usize) {
    let mut job = egui::text::LayoutJob::simple_singleline(
        value.to_owned(),
        FontId::proportional(size),
        color,
    );
    job.wrap.max_width = rect.width();
    job.wrap.max_rows = rows;
    job.wrap.break_anywhere = true;
    job.wrap.overflow_character = Some('…');
    let galley = ui.fonts(|fonts| fonts.layout_job(job));
    ui.painter().with_clip_rect(rect).galley(rect.min, galley, color);
}
fn send(app: &mut AeAppEguiApp, ctx: &egui::Context, action: DownloadAction) {
    if app.is_preview() || control_state(ctx).pending.is_some() {
        return;
    }
    let (offer, label) = match &action {
        DownloadAction::Pause(offer) => (offer.clone(), "Pause requested…"),
        DownloadAction::Resume(offer) => (offer.clone(), "Resume / retry requested…"),
        DownloadAction::Cancel(offer) => (offer.clone(), "Cancel requested…"),
        DownloadAction::RemoveQueued(offer) => (offer.clone(), "Remove requested…"),
        DownloadAction::MoveToFront(offer) => (offer.clone(), "Queue change requested…"),
    };
    let id = CONTROL_ID.fetch_add(1, Ordering::Relaxed);
    let mut state = ControlState::default();
    if app
        .backend
        .backend_commander
        .send(AeAppLibRequest::ControlDownloadTracked(id, action))
        .is_err()
    {
        state.error =
            Some("The download service is not available. The request was not sent.".into());
        app.nonfatal_errors.push(BackendError::ChannelDisconnected);
    } else {
        state.pending = Some(PendingControl {
            id,
            offer,
            label,
            started: Instant::now(),
        });
    }
    ctx.data_mut(|data| data.insert_temp(control_key(), state));
}
fn phase_text(phase: DownloadPhase) -> &'static str {
    match phase {
        DownloadPhase::Idle => "Preparing download…",
        DownloadPhase::Downloading => "Downloading",
        DownloadPhase::Paused => "Paused",
        DownloadPhase::Finalizing => "Finishing installation…",
        DownloadPhase::Finished => "Installation complete",
        DownloadPhase::Failed => "Download failed",
        DownloadPhase::Cancelled => "Cancelled",
    }
}
fn control_button(ui: &mut Ui, rect: Rect, label: &str, enabled: bool) -> egui::Response {
    ui.add_enabled_ui(enabled, |ui| {
        ui.put(
            rect,
            egui::Button::new(label).wrap_mode(egui::TextWrapMode::Extend),
        )
    })
    .inner
}
fn render_row(
    app: &mut AeAppEguiApp,
    ui: &mut Ui,
    download: &QueuedDownload,
    status: Option<&DownloadStatus>,
    confirmed: bool,
    position: usize,
) {
    let current = status.is_some();
    let phase = status.map(|s| s.phase).unwrap_or(DownloadPhase::Idle);
    let failed = phase == DownloadPhase::Failed;
    let height = if current {
        if failed {
            193.0
        } else {
            161.0
        }
    } else {
        125.0
    };
    let (rect, _) =
        ui.allocate_exact_size(vec2(ui.available_width(), height), egui::Sense::hover());
    ui.painter().rect_filled(rect, 8.0, shell::SURFACE);
    ui.painter().rect_stroke(rect, 8.0, Stroke::new(1.0, shell::BORDER));
    let game = app
        .games
        .get(&download.slug)
        .or_else(|| app.games.values().find(|g| g.offer == download.offer))
        .cloned();
    let art_width = if rect.width() > 700.0 { 142.0 } else { 100.0 };
    let image = Rect::from_min_size(rect.min + vec2(14.0, 14.0), vec2(art_width, 90.0));
    if let Some(game) = &game {
        crate::views::game_view::artwork(app, ui, image, game);
    } else {
        ui.painter().rect_filled(image, 4.0, shell::BACKGROUND);
        ui.painter().text(
            image.center(),
            Align2::CENTER_CENTER,
            "EA",
            FontId::proportional(24.0),
            shell::MUTED,
        );
    }
    let x = image.right() + 18.0;
    let width = (rect.right() - x - 17.0).max(50.0);
    let title = game.as_ref().map(|g| g.name.as_str()).unwrap_or(download.offer.as_str());
    text(
        ui,
        Rect::from_min_size(pos2(x, rect.top() + 15.0), vec2(width, 25.0)),
        title,
        18.0,
        shell::TEXT,
        1,
    );
    let state = if let Some(label) = pending_label(ui.ctx(), &download.offer) {
        label.to_owned()
    } else if current {
        phase_text(phase).to_owned()
    } else {
        format!("Queued · {}", position + 1)
    };
    text(
        ui,
        Rect::from_min_size(pos2(x, rect.top() + 47.0), vec2(width, 22.0)),
        &state,
        12.0,
        if phase == DownloadPhase::Failed {
            Color32::from_rgb(255, 126, 126)
        } else {
            shell::MUTED
        },
        1,
    );
    if let Some(status) = status {
        let bar = Rect::from_min_size(pos2(x, rect.top() + 79.0), vec2(width, 7.0));
        ui.painter().rect_filled(bar, 4.0, shell::BORDER);
        if let Some(amount) = fraction(status.bytes, status.total) {
            ui.painter().rect_filled(
                Rect::from_min_size(bar.min, vec2(bar.width() * amount, bar.height())),
                4.0,
                ACCENT_COLOR,
            );
        }
        text(
            ui,
            Rect::from_min_size(pos2(x, rect.top() + 96.0), vec2(width, 18.0)),
            &progress_text(status),
            11.0,
            shell::MUTED,
            1,
        );
        if phase == DownloadPhase::Finalizing {
            text(
                ui,
                Rect::from_min_size(pos2(x, rect.top() + 125.0), vec2(width, 22.0)),
                "Setup is running. Controls return when it finishes.",
                11.0,
                shell::MUTED,
                1,
            );
        } else {
            let y = rect.top() + 121.0;
            let pause = Rect::from_min_size(pos2(x, y), vec2(91.0, 29.0));
            let enabled = !app.is_preview() && control_state(ui.ctx()).pending.is_none();
            if let Some(label) = primary_control(phase, confirmed) {
                let action = if phase == DownloadPhase::Downloading {
                    DownloadAction::Pause(download.offer.clone())
                } else {
                    DownloadAction::Resume(download.offer.clone())
                };
                if control_button(ui, pause, label, enabled).clicked() {
                    send(app, ui.ctx(), action);
                }
            }
            if can_cancel(phase, confirmed) {
                let cancel = pause.translate(vec2(103.0, 0.0));
                if control_button(ui, cancel, "Cancel", enabled)
                    .on_hover_text("Stops this transfer and keeps downloaded files. The next queued item can then start.")
                    .clicked()
                {
                    send(app, ui.ctx(), DownloadAction::Cancel(download.offer.clone()));
                }
            }
        }
        if failed {
            text(
                    ui,
                    Rect::from_min_size(pos2(x, rect.top() + 159.0), vec2(width, 28.0)),
                    status.error.as_deref().unwrap_or("No failure details are available. Retry restarts the transfer and verifies completed files."),
                    11.0,
                    Color32::from_rgb(255, 153, 153),
                    2,
                );
        }
    } else {
        let next = Rect::from_min_size(pos2(x, rect.top() + 77.0), vec2(150.0, 31.0));
        let enabled = !app.is_preview() && control_state(ui.ctx()).pending.is_none();
        if control_button(ui, next, "Download next", position > 0 && enabled).clicked() {
            send(
                app,
                ui.ctx(),
                DownloadAction::MoveToFront(download.offer.clone()),
            );
        }
        let remove = Rect::from_min_size(next.min + vec2(163.0, 0.0), vec2(95.0, 31.0));
        if control_button(ui, remove, "Remove", enabled)
            .on_hover_text("Remove this waiting item from the queue.")
            .clicked()
        {
            send(
                app,
                ui.ctx(),
                DownloadAction::RemoveQueued(download.offer.clone()),
            );
        }
    }
    ui.add_space(13.0);
}
pub fn downloads_view(app: &mut AeAppEguiApp, ui: &mut Ui) {
    let controls = control_state(ui.ctx());
    if let Some(pending) = controls.pending {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(if pending.started.elapsed() < Duration::from_secs(30) {
                pending.label
            } else {
                "Still waiting for the download service to finish this request…"
            });
        });
        ui.add_space(8.0);
        ui.ctx().request_repaint_after(Duration::from_millis(100));
    }
    if let Some(error) = controls.error {
        ui.horizontal_wrapped(|ui| {
            ui.colored_label(
                Color32::from_rgb(255, 153, 153),
                format!("Download request failed: {error}"),
            );
            if ui.button("Dismiss").clicked() {
                let mut state = control_state(ui.ctx());
                state.error = None;
                ui.ctx().data_mut(|data| data.insert_temp(control_key(), state));
            }
        });
        ui.add_space(8.0);
    }
    let order = ui.ctx().data(|data| data.get_temp::<Vec<String>>(order_key())).unwrap_or_default();
    let mut queued: Vec<_> = app
        .install_queue
        .values()
        .filter(|item| {
            app.installing_now.as_ref().is_none_or(|current| current.offer != item.offer)
        })
        .cloned()
        .collect();
    queued.sort_by(|a, b| {
        let position = |game: &QueuedDownload| {
            order.iter().position(|offer| offer == &game.offer).unwrap_or(usize::MAX)
        };
        position(a).cmp(&position(b)).then_with(|| a.offer.cmp(&b.offer))
    });
    egui::ScrollArea::vertical()
        .id_source("download-manager-list")
        .auto_shrink(false)
        .show(ui, |ui| {
            if let Some(current) = app.installing_now.clone() {
                let (status, confirmed) = current_status(ui.ctx(), &current);
                render_row(app, ui, &current, Some(&status), confirmed, 0);
            }
            for (position, game) in queued.iter().enumerate() {
                render_row(app, ui, game, None, false, position);
            }
            if app.installing_now.is_none() && queued.is_empty() {
                shell::empty(ui, "No downloads", "Downloads and updates appear here when you add a game.");
            }
            ui.add_space(8.0);
            ui.label(RichText::new("Pause keeps the active transfer in memory. Cancel keeps downloaded files and lets the next queued item start.").size(11.0).color(shell::MUTED));
            ui.label(RichText::new("After restarting the client, Resume verifies completed files and restarts incomplete files.").size(11.0).color(shell::MUTED));
        });
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unknown_size_has_no_invalid_percentage() {
        assert_eq!(fraction(0, 0), None);
        assert_eq!(fraction(10, 0), None);
    }
    #[test]
    fn progress_is_bounded_even_if_bytes_are_recounted() {
        assert_eq!(fraction(50, 100), Some(0.5));
        assert_eq!(fraction(150, 100), Some(1.0));
        assert_eq!(fraction(0, 100), Some(0.0));
    }
    #[test]
    fn an_old_offer_failure_cannot_enable_actions_for_the_new_current_offer() {
        let current = QueuedDownload {
            slug: "new".into(),
            offer: "new-offer".into(),
            downloaded_bytes: 10,
            total_bytes: 100,
        };
        let old = DownloadStatus {
            offer_id: Some("old-offer".into()),
            phase: DownloadPhase::Failed,
            error: Some("old failure".into()),
            ..Default::default()
        };
        let (status, confirmed) = resolve_status(&current, Some(old));
        assert!(!confirmed);
        assert_eq!(status.phase, DownloadPhase::Idle);
        assert!(status.error.is_none());
        assert_eq!(primary_control(status.phase, confirmed), None);
        assert!(!can_cancel(status.phase, confirmed));
    }
    #[test]
    fn matching_phase_uses_fresh_progress_and_allows_retry_counter_reset() {
        let mut current = QueuedDownload {
            slug: "game".into(),
            offer: "offer".into(),
            downloaded_bytes: 60,
            total_bytes: 100,
        };
        let snapshot = DownloadStatus {
            offer_id: Some("offer".into()),
            phase: DownloadPhase::Downloading,
            bytes: 10,
            total: 100,
            error: None,
        };
        let (status, confirmed) = resolve_status(&current, Some(snapshot.clone()));
        assert!(confirmed);
        assert_eq!(status.bytes, 60);
        current.downloaded_bytes = 0;
        let (status, _) = resolve_status(&current, Some(snapshot));
        assert_eq!(status.bytes, 0);
    }
    #[test]
    fn finalizing_finished_and_unconfirmed_states_have_no_transfer_actions() {
        for phase in [
            DownloadPhase::Idle,
            DownloadPhase::Finalizing,
            DownloadPhase::Finished,
        ] {
            assert_eq!(primary_control(phase, true), None);
            assert!(!can_cancel(phase, true));
        }
        for (phase, label) in [
            (DownloadPhase::Downloading, "Pause"),
            (DownloadPhase::Paused, "Resume"),
            (DownloadPhase::Failed, "Retry"),
            (DownloadPhase::Cancelled, "Retry"),
        ] {
            assert_eq!(primary_control(phase, true), Some(label));
            assert!(can_cancel(phase, true));
            assert_eq!(primary_control(phase, false), None);
            assert!(!can_cancel(phase, false));
        }
    }
    #[test]
    fn a_late_control_acknowledgment_does_not_clear_a_new_request() {
        let mut state = ControlState {
            pending: Some(PendingControl {
                id: 2,
                offer: "offer".into(),
                label: "Pause requested…",
                started: Instant::now(),
            }),
            error: None,
        };
        state.complete(1, Err("old failure".into()));
        assert_eq!(state.pending.as_ref().unwrap().id, 2);
        assert!(state.error.is_none());
        state.complete(2, Err("service refused the request".into()));
        assert!(state.pending.is_none());
        assert_eq!(state.error.as_deref(), Some("service refused the request"));
    }
    #[test]
    fn backend_failure_releases_control_without_a_late_success_hiding_the_error() {
        let mut state = ControlState {
            pending: Some(PendingControl {
                id: 5,
                offer: "offer".into(),
                label: "Pause requested…",
                started: Instant::now(),
            }),
            error: None,
        };
        state.backend_unavailable("Download service stopped");
        assert!(state.pending.is_none());
        state.complete(5, Ok(()));
        assert_eq!(state.error.as_deref(), Some("Download service stopped"));
    }
    #[test]
    fn unknown_size_during_finalization_does_not_claim_the_transfer_has_not_started() {
        let mut status = DownloadStatus {
            phase: DownloadPhase::Finalizing,
            ..Default::default()
        };
        assert_eq!(
            progress_text(&status),
            "Downloaded files are being installed."
        );
        status.phase = DownloadPhase::Downloading;
        status.bytes = 250;
        assert!(progress_text(&status).contains("total size unavailable"));
        assert!(!progress_text(&status).contains('%'));
    }
}
