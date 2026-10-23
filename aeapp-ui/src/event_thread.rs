use egui::Context;
use std::sync::mpsc::{Receiver, Sender};

use crate::bridge_thread::BackendError;
use aeapp::core::{
    service_layer::{
        ServiceFriends, ServiceGetMyFriendsRequestBuilder, SERVICE_REQUEST_GETMYFRIENDS,
    },
    LockedAeApp,
};
use log::info;

// TODO(headassbtw): integrate this into the enum too (out of scope for the PR i wrote this in)
pub struct EventThreadFriendStatusResponse {
    pub id: String,
    pub presence: aeapp::rtm::client::RichPresence,
}

pub enum AeAppEventResponse {
    FriendStatusResponse(EventThreadFriendStatusResponse),
}

pub enum AeAppEventRequest {
    SubscribeToFriendPresence,
    ShutdownRequest,
}

pub struct EventThread {}

impl EventThread {
    pub fn new(
        ctx: &Context,
        aeapp: LockedAeApp,
        rtm_cmd_listener: Receiver<AeAppEventRequest>,
        rtm_responder: Sender<AeAppEventResponse>,
    ) -> Self {
        let context = ctx.clone();

        tokio::task::spawn(async move {
            let result = EventThread::run(rtm_cmd_listener, rtm_responder, &context, aeapp).await;
            if result.is_err() {
                panic!("Event thread failed! {}", result.err().unwrap());
            } else {
                info!("Event thread shut down")
            }
        });

        Self {}
    }

    async fn run(
        rtm_cmd_listener: Receiver<AeAppEventRequest>,
        rtm_responder: Sender<AeAppEventResponse>,
        ctx: &Context,
        aeapp_arc: LockedAeApp,
    ) -> Result<(), BackendError> {
        let mut aeapp = aeapp_arc.lock().await;

        let friends: ServiceFriends = aeapp
            .service_layer()
            .request(
                SERVICE_REQUEST_GETMYFRIENDS,
                ServiceGetMyFriendsRequestBuilder::default()
                    .offset(0)
                    .limit(100)
                    .is_mutual_friends_enabled(false)
                    .build()
                    .unwrap(),
            )
            .await?;

        let rtm = aeapp.rtm();
        rtm.login().await?;

        let players: Vec<String> =
            friends.friends().items().iter().map(|f| f.id().to_owned()).collect();
        info!("Subscribed to {} players", players.len());

        rtm.subscribe(&players).await?;
        drop(aeapp);

        'outer: loop {
            let mut aeapp = aeapp_arc.lock().await;
            aeapp.rtm().heartbeat().await?;

            {
                let store = aeapp.rtm().presence_store().lock().await;
                for entry in store.iter() {
                    let _ = rtm_responder.send(AeAppEventResponse::FriendStatusResponse(
                        EventThreadFriendStatusResponse {
                            id: entry.0.to_string(),
                            presence: entry.1,
                        },
                    ));
                    // This can cause excessive repainting if it keeps updating friends we know about
                    egui::Context::request_repaint(&ctx);
                }
            }

            drop(aeapp);

            let request = rtm_cmd_listener.try_recv();
            if request.is_err() {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                continue;
            }

            match request? {
                AeAppEventRequest::SubscribeToFriendPresence => {}
                AeAppEventRequest::ShutdownRequest => break 'outer Ok(()),
            }

            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    }
}
