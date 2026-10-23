//! Deliberately local demonstration data. No EA account, ownership or runtime is implied.
use crate::{
    bridge_thread::BridgeThread, GameDetails, GameDetailsWrapper, GameInfo, GameVersionInfo,
};
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

pub fn games() -> HashMap<String, GameInfo> {
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
                details: GameDetailsWrapper::Available(GameDetails {
                    time: 0,
                    achievements_unlocked: 0,
                    achievements_total: 0,
                    path: "Demo only · no game files".to_owned(),
                    system_requirements_min: None,
                    system_requirements_rec: None,
                }),
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
