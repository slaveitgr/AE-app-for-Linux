use crate::{
    core::settings::AeAppSetting,
    lsx::{
        connection::LockedConnectionState,
        request::LSXRequestError,
        types::{
            LSXGetInternetConnectedState, LSXGetSetting, LSXGetSettingResponse,
            LSXInternetConnectedState, LSXResponseType, LSXSetDownloaderUtilization,
        },
    },
    make_lsx_handler_response,
};

pub async fn handle_settings_request(
    state: LockedConnectionState,
    request: LSXGetSetting,
) -> Result<Option<LSXResponseType>, LSXRequestError> {
    let setting = match request.attr_SettingId {
        AeAppSetting::IsIgoEnabled => "false".to_string(),
        AeAppSetting::IsIgoAvailable => "false".to_string(),
        AeAppSetting::Environment => "production".to_string(),
        AeAppSetting::Language => {
            let mut connection = state.write().await;
            let app = connection.aeapp().await;
            app.locale().full_str().to_owned()
        }
    };

    make_lsx_handler_response!(Response, GetSettingResponse, { attr_Setting: setting })
}

pub async fn handle_connectivity_request(
    _: LockedConnectionState,
    _: LSXGetInternetConnectedState,
) -> Result<Option<LSXResponseType>, LSXRequestError> {
    // TODO Actually check this
    make_lsx_handler_response!(Response, InternetConnectedState, { attr_connected: 1 })
}

pub async fn handle_set_downloader_util_request(
    _: LockedConnectionState,
    _: LSXSetDownloaderUtilization,
) -> Result<Option<LSXResponseType>, LSXRequestError> {
    // TODO Actually set this
    Ok(None)
}
