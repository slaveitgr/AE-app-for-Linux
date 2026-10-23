use crate::{
    bridge_thread::{AeAppLibRequest, BackendError},
    shell, AeAppEguiApp, ACCENT_COLOR,
};
use aeapp::content::manager::{DownloadAction, DownloadPhase, DownloadStatus};
use egui::{pos2, vec2, Align2, Color32, FontId, Rect, RichText, Stroke, Ui};
use humansize::DECIMAL;

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
fn send(app: &mut AeAppEguiApp, action: DownloadAction) {
    if app.is_preview() {
        return;
    }
    if app.backend.backend_commander.send(AeAppLibRequest::ControlDownload(action)).is_err() {
        app.nonfatal_errors.push(BackendError::ChannelDisconnected);
    }
}
fn phase_text(phase: DownloadPhase) -> &'static str {
    match phase {
        DownloadPhase::Idle => "Preparing download…",
        DownloadPhase::Downloading => "Downloading",
        DownloadPhase::Paused => "Paused",
        DownloadPhase::Finalizing => "Finishing installation…",
        DownloadPhase::Finished => "Installed",
        DownloadPhase::Failed => "Download stopped",
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
    position: usize,
) {
    let current = status.is_some();
    let phase = status.map(|s| s.phase).unwrap_or(DownloadPhase::Idle);
    let failed = status.is_some_and(|s| s.error.is_some() && s.phase == DownloadPhase::Failed);
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
    let state = if current {
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
            let progress = format!(
                "{} / {}  ·  {:.0}%",
                humansize::SizeFormatter::new(status.bytes.min(status.total), DECIMAL),
                humansize::SizeFormatter::new(status.total, DECIMAL),
                amount * 100.0
            );
            text(
                ui,
                Rect::from_min_size(pos2(x, rect.top() + 96.0), vec2(width, 18.0)),
                &progress,
                11.0,
                shell::MUTED,
                1,
            );
        } else {
            text(
                ui,
                Rect::from_min_size(pos2(x, rect.top() + 96.0), vec2(width, 18.0)),
                "Size will be checked when the transfer starts.",
                11.0,
                shell::MUTED,
                1,
            );
        }
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
            let button = match phase {
                DownloadPhase::Paused => {
                    Some(("Resume", DownloadAction::Resume(download.offer.clone())))
                }
                DownloadPhase::Failed => {
                    Some(("Retry", DownloadAction::Resume(download.offer.clone())))
                }
                DownloadPhase::Downloading => {
                    Some(("Pause", DownloadAction::Pause(download.offer.clone())))
                }
                _ => None,
            };
            if let Some((label, action)) = button {
                if control_button(ui, pause, label, !app.is_preview()).clicked() {
                    send(app, action);
                }
            }
            if matches!(
                phase,
                DownloadPhase::Downloading | DownloadPhase::Paused | DownloadPhase::Failed
            ) {
                let cancel = pause.translate(vec2(103.0, 0.0));
                if control_button(ui, cancel, "Cancel", !app.is_preview())
                    .on_hover_text("Stops this transfer and keeps downloaded files.")
                    .clicked()
                {
                    send(app, DownloadAction::Cancel(download.offer.clone()));
                }
            }
        }
        if failed {
            if let Some(error) = &status.error {
                text(
                    ui,
                    Rect::from_min_size(pos2(x, rect.top() + 159.0), vec2(width, 28.0)),
                    error,
                    11.0,
                    Color32::from_rgb(255, 153, 153),
                    2,
                );
            }
        }
    } else {
        let next = Rect::from_min_size(pos2(x, rect.top() + 77.0), vec2(150.0, 31.0));
        if control_button(ui, next, "Download next", position > 0 && !app.is_preview()).clicked() {
            send(app, DownloadAction::MoveToFront(download.offer.clone()));
        }
        let remove = Rect::from_min_size(next.min + vec2(163.0, 0.0), vec2(95.0, 31.0));
        if control_button(ui, remove, "Remove", !app.is_preview())
            .on_hover_text("Remove this waiting item from the queue.")
            .clicked()
        {
            send(app, DownloadAction::RemoveQueued(download.offer.clone()));
        }
    }
    ui.add_space(13.0);
}
pub fn downloads_view(app: &mut AeAppEguiApp, ui: &mut Ui) {
    let status = ui
        .ctx()
        .data(|data| data.get_temp::<DownloadStatus>(status_key()))
        .unwrap_or_default();
    let order = ui.ctx().data(|data| data.get_temp::<Vec<String>>(order_key())).unwrap_or_default();
    let mut queued: Vec<_> = app.install_queue.values().cloned().collect();
    queued.sort_by_key(|game| {
        order.iter().position(|offer| offer == &game.offer).unwrap_or(usize::MAX)
    });
    egui::ScrollArea::vertical()
        .id_source("download-manager-list")
        .auto_shrink(false)
        .show(ui, |ui| {
            if let Some(current) = app.installing_now.clone() {
                let fallback = DownloadStatus {
                    offer_id: Some(current.offer.clone()),
                    bytes: current.downloaded_bytes,
                    total: current.total_bytes,
                    ..DownloadStatus::default()
                };
                let current_status = if status.offer_id.as_deref() == Some(current.offer.as_str()) {
                    &status
                } else {
                    &fallback
                };
                render_row(app, ui, &current, Some(current_status), 0);
            }
            for (position, game) in queued.iter().enumerate() {
                render_row(app, ui, game, None, position);
            }
            if app.installing_now.is_none() && queued.is_empty() {
                shell::empty(ui, "No downloads", "Downloads and updates appear here when you add a game.");
            }
            ui.add_space(8.0);
            ui.label(RichText::new("Pause keeps the active transfer in memory. Cancel stops it and keeps downloaded files.").size(11.0).color(shell::MUTED));
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
}
