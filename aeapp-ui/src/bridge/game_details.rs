use egui::Context;
use std::sync::mpsc::Sender;

use crate::{
    bridge_thread::{AeAppLibResponse, BackendError, InteractThreadGameDetailsResponse},
    util::markdown::html_to_easymark,
    GameDetails,
};
use aeapp::core::{
    service_layer::{
        ServiceGameSystemRequirements, ServiceGameSystemRequirementsRequestBuilder,
        SERVICE_REQUEST_GAMESYSTEMREQUIREMENTS,
    },
    LockedAeApp,
};

pub async fn game_details_request(
    aeapp_arc: LockedAeApp,
    slug: String,
    channel: Sender<AeAppLibResponse>,
    ctx: &Context,
) -> Result<(), BackendError> {
    let mut aeapp = aeapp_arc.lock().await;
    let path = if let Some(offer) = aeapp.mut_library().game_by_base_slug(&slug).await? {
        if offer.is_installed().await {
            offer
                .execute_path(false)
                .await
                .ok()
                .and_then(|executable| {
                    executable.parent().map(|folder| folder.display().to_string())
                })
                .unwrap_or_default()
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    let rq = aeapp.service_layer().request(
        SERVICE_REQUEST_GAMESYSTEMREQUIREMENTS,
        ServiceGameSystemRequirementsRequestBuilder::default()
            .slug(slug.clone())
            .locale(aeapp.locale().short_str().to_owned())
            .build()
            .unwrap(),
    );
    let rq: ServiceGameSystemRequirements = rq.await?;

    //TODO: parse async

    let (min, rec) = if rq.system_requirements().len() >= 1 {
        (
            Some(html_to_easymark(rq.system_requirements()[0].minimum())),
            Some(html_to_easymark(rq.system_requirements()[0].recommended())),
        )
    } else {
        (None, None)
    };

    let res = AeAppLibResponse::GameDetailsResponse(InteractThreadGameDetailsResponse {
        slug: slug.clone(),
        response: GameDetails {
            time: 0,
            achievements_unlocked: 0,
            achievements_total: 0,
            path,
            system_requirements_min: min,
            system_requirements_rec: rec,
        },
    });
    let _ = channel.send(res);
    egui::Context::request_repaint(&ctx);
    Ok(())
}
