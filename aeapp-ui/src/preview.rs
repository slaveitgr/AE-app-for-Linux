//! Deliberately local demonstration data. No EA account, ownership or runtime is implied.
use crate::{
    bridge_thread::BridgeThread,
    views::downloads_view::{apply_status, QueuedDownload},
    GameDetails, GameDetailsWrapper, GameInfo, GameVersionInfo,
};
use aeapp::content::manager::{DownloadPhase, DownloadStatus};
use std::{collections::HashMap, sync::mpsc::channel};

pub fn disconnected_backend() -> BridgeThread {
    let (backend_commander, _) = channel();
    let (_, backend_listener) = channel();
    let (rtm_commander, _) = channel();
    let (_, rtm_listener) = channel();
    BridgeThread {
        backend_listener,
        backend_commander,
        rtm_listener,
        rtm_commander,
    }
}

fn details_fixture(state: &str) -> GameDetailsWrapper {
    match state {
        "loading" => GameDetailsWrapper::Loading,
        "failed" => GameDetailsWrapper::Failed(
            "Local preview: game details could not be loaded. Retry is disabled in preview.".into(),
        ),
        _ => GameDetailsWrapper::Available(GameDetails {
            time: None,
            achievements_unlocked: None,
            achievements_total: None,
            path: "Demo only · no game files".to_owned(),
            system_requirements_min: None,
            system_requirements_rec: None,
        }),
    }
}

fn download_fixture(state: &str) -> (QueuedDownload, DownloadStatus) {
    let total = if state == "unknown-size" {
        0
    } else {
        2_048_000_000
    };
    let bytes = if state == "finalizing" {
        total
    } else {
        512_000_000
    };
    let phase = match state {
        "paused" => DownloadPhase::Paused,
        "failed" => DownloadPhase::Failed,
        "finalizing" => DownloadPhase::Finalizing,
        _ => DownloadPhase::Downloading,
    };
    let current = QueuedDownload {
        slug: "need-for-speed-heat".to_owned(),
        offer: "preview:need-for-speed-heat".to_owned(),
        downloaded_bytes: bytes,
        total_bytes: total,
    };
    let status = DownloadStatus {
        offer_id: Some(current.offer.clone()),
        phase,
        bytes,
        total,
        error: (state == "failed").then(|| {
            "Local preview: the transfer failed. Retry is disabled in preview.".to_owned()
        }),
    };
    (current, status)
}

/// Seed a frozen local transfer with matching row and sidebar counters.
pub fn seed_download(ctx: &egui::Context, state: &str) -> QueuedDownload {
    let (current, status) = download_fixture(state);
    apply_status(ctx, status);
    current
}

pub fn games(details_state: &str) -> HashMap<String, GameInfo> {
    [
        ("need-for-speed-payback", "Need for Speed™ Payback", true),
        ("star-wars-jedi-survivor", "STAR WARS Jedi: Survivor", false),
        ("titanfall-2", "Titanfall 2", false),
        ("the-sims-4", "The Sims 4", false),
        ("need-for-speed-heat", "Need for Speed Heat", false),
        (
            "mass-effect-legendary",
            "Mass Effect Legendary Edition",
            false,
        ),
        ("it-takes-two", "It Takes Two", false),
    ]
    .into_iter()
    .map(|(slug, name, installed)| {
        (
            slug.to_owned(),
            GameInfo {
                slug: slug.to_owned(),
                offer: format!("preview:{slug}"),
                name: name.to_owned(),
                details: details_fixture(details_state),
                version: GameVersionInfo {
                    installed: "Demo".to_owned(),
                    latest: "Demo".to_owned(),
                    mandatory: false,
                },
                dlc: Vec::new(),
                installed,
                has_cloud_saves: false,
            },
        )
    })
    .collect()
}
