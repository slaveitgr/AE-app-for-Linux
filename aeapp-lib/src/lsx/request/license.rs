use log::{debug, info};
use std::{env, path::PathBuf};

use crate::{
    core::{auth::hardware::HardwareInfo, launch::LaunchMode},
    lsx::{
        connection::LockedConnectionState,
        request::LSXRequestError,
        types::{LSXRequestLicense, LSXRequestLicenseResponse, LSXResponseType},
    },
    make_lsx_handler_response,
    ooa::{detect_ooa_version, request_license, LicenseAuth},
};

pub async fn handle_license_request(
    state: LockedConnectionState,
    request: LSXRequestLicense,
) -> Result<Option<LSXResponseType>, LSXRequestError> {
    info!("Requesting OOA License and Denuvo Token");

    if let Ok(token) = env::var("AEAPP_DENUVO_TOKEN") {
        return make_lsx_handler_response!(Response, RequestLicenseResponse, { attr_License: token.to_owned() });
    }

    let arc = state.write().await.aeapp_arc();
    let mut aeapp = arc.lock().await;

    let playing = aeapp.playing().as_ref().unwrap();
    let content_id = playing.content_id().to_owned();
    let game_path = PathBuf::from(playing.game_path());
    let mode = playing.mode();

    let auth = match mode {
        LaunchMode::Offline(_) => {
            return make_lsx_handler_response!(Response, RequestLicenseResponse, { attr_License: String::new() });
        }
        LaunchMode::Online(_) => LicenseAuth::AccessToken(aeapp.access_token().await?),
        LaunchMode::OnlineOffline(_, persona, password) => {
            LicenseAuth::Direct(persona.to_owned(), password.to_owned())
        }
    };

    // Bind the runtime token to the same hardware identity as the game's
    // initial OOA license; newer Activation DLLs use a different hash format.
    let version = detect_ooa_version(game_path).await.unwrap_or(1);
    let hw_info = HardwareInfo::new(version);
    let license = request_license(
        &content_id,
        &hw_info.generate_hardware_hash(),
        &auth,
        Some(request.attr_RequestTicket.as_str()),
        Some(request.attr_TicketEngine.as_str()),
    )
    .await?;

    if license.game_token.is_none() {
        return Err(LSXRequestError::Denuvo);
    }

    info!("Successfully retrieved license tokens");

    let token = license.game_token.as_ref().unwrap();

    debug!("Got Denuvo Token: {}", token);

    make_lsx_handler_response!(Response, RequestLicenseResponse, { attr_License: token.to_owned() })
}
