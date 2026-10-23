const LANGUAGES: &str =
    "ar_SA,de_DE,en_US,es_ES,es_MX,fr_FR,it_IT,ja_JP,ko_KR,pl_PL,pt_BR,ru_RU,zh_CN,zh_TW";
//const LANGUAGES: &str = "de_DE,en_US,es_ES,es_MX,fr_FR,it_IT,ja_JP,pl_PL,pt_BR,ru_RU,zh_TW";
//const LANGUAGES: &str = "en_US,es_ES,fr_FR,pt_BR";

use crate::{
    core::service_layer::{ServiceGameProductUser, ServiceOwnershipStatus},
    lsx::{
        connection::LockedConnectionState,
        request::LSXRequestError,
        types::{
            LSXGameInfoId, LSXGetAllGameInfo, LSXGetAllGameInfoResponse, LSXGetGameInfo,
            LSXGetGameInfoResponse, LSXResponseType,
        },
    },
    make_lsx_handler_response,
};

pub async fn handle_game_info_request(
    state: LockedConnectionState,
    request: LSXGetGameInfo,
) -> Result<Option<LSXResponseType>, LSXRequestError> {
    let game_info = match request.attr_GameInfoId {
        LSXGameInfoId::FreeTrial => "false".to_string(),
        LSXGameInfoId::Languages => LANGUAGES.to_string(),
        LSXGameInfoId::InstalledLanguage => "en_US".to_string(),
        LSXGameInfoId::FullGamePurchased => {
            let mut connection = state.write().await;
            let app = connection.aeapp().await;
            app.playing()
                .as_ref()
                .and_then(|game| game.offer().as_ref())
                .is_some_and(|offer| {
                    has_full_game_access(offer.product().product().game_product_user())
                })
                .to_string()
        }
    };

    make_lsx_handler_response!(Response, GetGameInfoResponse, { attr_GameInfo: game_info })
}

// The SDK uses this flag for full-game access, including an active vault
// entitlement. A trial or disabled entitlement must not be promoted to full access.
fn has_full_game_access(user: &ServiceGameProductUser) -> bool {
    user.status() == &ServiceOwnershipStatus::Active
        && !user.ownership_methods().is_empty()
        && user.game_product_user_trial().is_none()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{core::settings::AeAppSetting, lsx::types::LSXGetSetting};

    #[test]
    fn payback_language_and_full_game_requests_deserialize() {
        let setting: LSXGetSetting = quick_xml::de::from_str(
            r#"<GetSetting SettingId="LANGUAGE" version="1"/>"#,
        )
        .unwrap();
        assert_eq!(setting.attr_SettingId, AeAppSetting::Language);
        let game: LSXGetGameInfo = quick_xml::de::from_str(
            r#"<GetGameInfo GameInfoId="FULLGAME_PURCHASED" version="1"/>"#,
        )
        .unwrap();
        assert_eq!(game.attr_GameInfoId, LSXGameInfoId::FullGamePurchased);
    }

    #[test]
    fn full_access_requires_an_active_non_trial_entitlement() {
        for (method, status, trial, expected) in [
            (Some("PURCHASE"), "ACTIVE", false, true),
            (Some("XGP_VAULT"), "ACTIVE", false, true),
            (Some("VAULT"), "ACTIVE", false, true),
            (Some("XGP_VAULT"), "DISABLED", false, false),
            (Some("XGP_VAULT"), "ACTIVE", true, false),
            (None, "ACTIVE", false, false),
        ] {
            let user: ServiceGameProductUser = serde_json::from_value(serde_json::json!({
                "ownershipMethods": method.into_iter().collect::<Vec<_>>(),
                "initialEntitlementDate": "2026-01-01T00:00:00Z",
                "entitlementId": null,
                "status": status,
                "gameProductUserTrial": trial.then(|| serde_json::json!({
                    "trialTimeRemainingSeconds": 60
                }))
            }))
            .unwrap();
            assert_eq!(has_full_game_access(&user), expected);
        }
    }
}

// <GetAllGameInfoResponse FullGamePurchased="true" FullGameReleased="true" InstalledVersion="0" MaxGroupSize="16" Languages="ar_SA,de_DE,en_US,es_ES,es_MX,fr_FR,it_IT,ja_JP,ko_KR,pl_PL,pt_BR,ru_RU,zh_CN,zh_TW" Expiration="0000-00-00T00:00:00" UpToDate="true" HasExpiration="false" InstalledLanguage="" EntitlementSource="STEAM" FullGameReleaseDate="2020-10-22T09:00:00" AvailableVersion="1.0.64.43203" DisplayName="Battlefield V Definitive Edition" FreeTrial="false" SystemTime="2023-06-23T04:22:10"/>

/// Just realized we're still telling every game that it's titanfall.
/// Should fix that at some point!
pub async fn handle_all_game_info_request(
    _: LockedConnectionState,
    _: LSXGetAllGameInfo,
) -> Result<Option<LSXResponseType>, LSXRequestError> {
    make_lsx_handler_response!(Response, GetAllGameInfoResponse, {
        attr_FullGamePurchased: true,
        attr_FullGameReleased: true,
        attr_InstalledVersion: "0".to_string(),
        attr_MaxGroupSize: 16,
        attr_Languages: LANGUAGES.to_string(),
        attr_Expiration: "0000-00-00T00:00:00".to_string(),
        attr_UpToDate: true,
        attr_HasExpiration: false,
        attr_EntitlementSource: "STEAM".to_string(),
        attr_AvailableVersion: "1.0.1.3".to_string(),
        attr_DisplayName: "Titanfall® 2 Deluxe Edition".to_string(),
        attr_FreeTrial: false,
        attr_InstalledLanguage: "en_US".to_string(),
        attr_FullGameReleaseDate: "2016-10-28T04:00:00".to_string(),
        attr_SystemTime: "2023-06-22T04:00:00".to_string()
    })
}
