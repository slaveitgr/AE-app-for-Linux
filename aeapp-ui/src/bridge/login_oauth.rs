use crate::bridge_thread::{AeAppLibResponse, BackendError, InteractThreadLoginResponse};
use aeapp::{
    core::{
        auth::{context::AuthContext, login, nucleus_token_exchange},
        service_layer::ServiceLayerError,
        LockedAeApp,
    },
    util::native::take_foreground_focus,
};
use egui::Context;
use std::sync::mpsc::Sender;

pub async fn login_oauth(
    aeapp_arc: LockedAeApp,
    channel: Sender<AeAppLibResponse>,
    ctx: &Context,
) -> Result<(), BackendError> {
    let aeapp = aeapp_arc.lock().await;

    {
        let mut auth_storage = aeapp.auth_storage().lock().await;
        let mut context = AuthContext::new()?;
        login::begin_oauth_login_flow(&mut context).await?;
        let token_res = nucleus_token_exchange(&context).await?;
        auth_storage.add_account(&token_res).await?;
    }

    let user = aeapp.local_user().await?;
    let message = AeAppLibResponse::LoginResponse(Ok(InteractThreadLoginResponse {
        you: user.player().as_ref().ok_or(ServiceLayerError::MissingField)?.to_owned(),
    }));

    channel.send(message)?;

    take_foreground_focus()?;
    ctx.request_repaint();
    Ok(())
}
