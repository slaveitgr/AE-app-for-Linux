use aeapp::{core::LockedAeApp, rtm::client::BasicPresence};
use egui::Context;
use log::debug;
use std::sync::mpsc::Sender;

use crate::{
    bridge_thread::{AeAppLibResponse, BackendError, InteractThreadFriendListResponse},
    ui_image::UIImageCacheLoaderCommand,
    views::friends_view::UIFriend,
};

pub async fn get_friends_request(
    aeapp_arc: LockedAeApp,
    channel: Sender<AeAppLibResponse>,
    remote_provider_channel: Sender<UIImageCacheLoaderCommand>,
    ctx: &Context,
) -> Result<(), BackendError> {
    debug!("received request to load friends");
    let aeapp = aeapp_arc.lock().await;
    let logged_in = aeapp.auth_storage().lock().await.current().is_some();
    if !logged_in {
        return Err(BackendError::LoggedOut);
    }

    let friends = aeapp.friends(0).await?;
    for friend in friends {
        remote_provider_channel.send(UIImageCacheLoaderCommand::ProvideRemote(
            crate::ui_image::UIImageType::Avatar(friend.id().to_string()),
            friend.avatar().as_ref().unwrap().medium().path().to_string(),
        ))?;
        let friend_info = UIFriend {
            name: friend.display_name().to_string(),
            id: friend.id().to_string(),
            online: BasicPresence::Offline,
            game: None,
            game_presence: None,
        };

        let res = AeAppLibResponse::FriendInfoResponse(InteractThreadFriendListResponse {
            friend: friend_info,
        });
        channel.send(res)?;

        ctx.request_repaint();
    }

    Ok(())
}
