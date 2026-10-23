use crate::{bridge_thread::BackendError, util::markdown::html_to_easymark, GameDetails};
use aeapp::core::{
    service_layer::{
        ServiceGameSystemRequirements, ServiceGameSystemRequirementsRequestBuilder,
        ServiceLayerError, ServicePlatform, ServiceSystemRequirements,
        SERVICE_REQUEST_GAMESYSTEMREQUIREMENTS,
    },
    LockedAeApp,
};

pub async fn game_details_request(
    aeapp_arc: LockedAeApp,
    slug: String,
) -> Result<GameDetails, BackendError> {
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

    let service = aeapp.service_layer().clone();
    let locale = aeapp.locale().short_str().to_owned();
    drop(aeapp);
    let rq = service.request(
        SERVICE_REQUEST_GAMESYSTEMREQUIREMENTS,
        ServiceGameSystemRequirementsRequestBuilder::default()
            .slug(slug.clone())
            .locale(locale)
            .build()
            .map_err(|_| ServiceLayerError::MissingField)?,
    );
    let rq: ServiceGameSystemRequirements = rq.await?;

    let (min, rec) = windows_requirements(rq.system_requirements());

    // This verified service supplies requirements, not account playtime or achievements.
    Ok(GameDetails {
        time: None,
        achievements_unlocked: None,
        achievements_total: None,
        path,
        system_requirements_min: min,
        system_requirements_rec: rec,
    })
}

fn requirement_text(html: &str) -> Option<String> {
    let text = html_to_easymark(html);
    (!text.trim().is_empty()).then_some(text)
}

fn windows_requirements(
    requirements: &[ServiceSystemRequirements],
) -> (Option<String>, Option<String>) {
    requirements
        .iter()
        .find(|requirements| requirements.platform() == &ServicePlatform::Pc)
        .map(|requirements| {
            (
                requirement_text(requirements.minimum()),
                requirement_text(requirements.recommended()),
            )
        })
        .unwrap_or((None, None))
}

#[cfg(test)]
mod tests {
    use super::{requirement_text, windows_requirements};
    use aeapp::core::service_layer::ServiceSystemRequirements;

    #[test]
    fn empty_requirements_are_unavailable_not_a_loaded_blank_section() {
        assert_eq!(requirement_text(""), None);
        assert_eq!(requirement_text("   "), None);
        assert!(
            requirement_text("<p>Windows 10</p>").is_some_and(|text| text.contains("Windows 10"))
        );
    }

    #[test]
    fn windows_requirements_do_not_use_a_mac_first_or_mac_only_response() {
        let requirements: Vec<ServiceSystemRequirements> =
            serde_json::from_value(serde_json::json!([
                {"platform": "MAC_PC", "minimum": "macOS", "recommended": "Mac hardware"},
                {"platform": "PC", "minimum": "Windows 10", "recommended": "Windows hardware"}
            ]))
            .unwrap();
        assert_eq!(
            windows_requirements(&requirements),
            (Some("Windows 10".into()), Some("Windows hardware".into()))
        );
        assert_eq!(windows_requirements(&requirements[..1]), (None, None));
        assert_eq!(windows_requirements(&[]), (None, None));
    }
}
