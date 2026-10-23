use std::sync::mpsc::{self, Receiver, Sender};

use anyhow::{bail, Result};
use log::info;
use aeapp::core::{
    auth::{
        context::AuthContext, login::begin_oauth_login_flow, nucleus_token_exchange, TokenResponse,
    },
    LockedAeApp, AeApp, AeAppOptionsBuilder,
};

pub struct InteractThreadLoginResponse {
    pub success: bool,
    pub name: String,
}

pub enum AeAppLibRequest {
    LoginRequest,
    GetGamesRequest,
    GetFriendsRequest,
    GetUserAvatarRequest(String, String),
    GetGameImagesRequest(String),
    GetGameDetailsRequest(String),
    StartGameRequest(String, bool),
    ShutdownRequest,
}

pub enum AeAppLibResponse {
    LoginResponse(InteractThreadLoginResponse),
    LoginCacheEmpty,
    GameInfoResponse(),
    FriendInfoResponse(),
    UserAvatarResponse(),
    GameDetailsResponse(),
    GameUIImagesResponse(),
    InteractionThreadDiedResponse,
}

pub struct BridgeThread {
    pub rx: Receiver<AeAppLibResponse>,
    pub tx: Sender<AeAppLibRequest>,
}

impl BridgeThread {
    pub fn new() -> Self {
        let (tx0, rx1) = mpsc::channel();
        let (tx1, rx0) = mpsc::channel();

        tokio::task::spawn(async move {
            let die_fallback_transmitter = tx1.clone();
            //panic::set_hook(Box::new( |_| {}));
            let result = BridgeThread::run(rx1, tx1).await;
            if result.is_err() {
                die_fallback_transmitter
                    .send(AeAppLibResponse::InteractionThreadDiedResponse)
                    .unwrap();
                panic!("Interact thread failed! {}", result.err().unwrap());
            } else {
                info!("Interact thread shut down")
            }
        });

        Self { rx: rx0, tx: tx0 }
    }

    async fn run(rx1: Receiver<AeAppLibRequest>, tx1: Sender<AeAppLibResponse>) -> Result<()> {
        let aeapp_arc: LockedAeApp = AeApp::new_with_options(
            AeAppOptionsBuilder::default()
                .dummy_local_user(false)
                .load_auth_storage(true)
                .build()?,
        )
        .await?;

        {
            let aeapp = aeapp_arc.lock().await;
            if aeapp.start_lsx(aeapp_arc.clone()).await.is_ok() {
                info!("LSX started");
            } else {
                info!("LSX failed to start!");
            }

            let mut auth_storage = aeapp.auth_storage().lock().await;
            let logged_in = auth_storage.logged_in().await?;
            if logged_in {
                drop(auth_storage);

                let user = aeapp.local_user().await?;
                let message = AeAppLibResponse::LoginResponse(InteractThreadLoginResponse {
                    success: true,
                    name: user.player().as_ref().unwrap().display_name().to_owned(),
                });

                tx1.send(message)?;
            } else {
                tx1.send(AeAppLibResponse::LoginCacheEmpty)?;
            }
        }

        'outer: loop {
            let request = rx1.try_recv();
            if request.is_err() {
                continue;
            }

            match request? {
                AeAppLibRequest::LoginRequest => {
                    let channel = tx1.clone();
                    let aeapp = aeapp_arc.clone();
                    async move {
                        let aeapp = aeapp.lock().await;

                        let mut auth_storage = aeapp.auth_storage().lock().await;
                        let logged_in = auth_storage.logged_in().await?;
                        if !logged_in {
                            let res = login_flow().await.unwrap();
                            aeapp.auth_storage().lock().await.add_account(&res);
                        };

                        channel
                            .send(AeAppLibResponse::LoginResponse(
                                InteractThreadLoginResponse {
                                    success: true,
                                    name: aeapp
                                        .local_user()
                                        .await?
                                        .player()
                                        .as_ref()
                                        .unwrap()
                                        .display_name()
                                        .to_owned(),
                                },
                            ))
                            .unwrap();

                        Ok::<(), anyhow::Error>(())
                    }
                    .await?;
                }
                AeAppLibRequest::GetGamesRequest => {
                    let channel = tx1.clone();
                    let aeapp = aeapp_arc.clone();
                }
                AeAppLibRequest::GetFriendsRequest => {
                    let channel = tx1.clone();
                    let aeapp = aeapp_arc.clone();
                }
                AeAppLibRequest::GetGameImagesRequest(slug) => {
                    let channel = tx1.clone();
                    let aeapp = aeapp_arc.clone();
                }
                AeAppLibRequest::GetUserAvatarRequest(id, url) => {
                    let channel = tx1.clone();
                }
                AeAppLibRequest::GetGameDetailsRequest(slug) => {
                    let channel = tx1.clone();
                    let aeapp = aeapp_arc.clone();
                }
                AeAppLibRequest::StartGameRequest(offer_id, hardcode) => {
                    //start_game_request(aeapp_arc.clone(), offer_id.clone(), hardcode).await;
                }
                AeAppLibRequest::ShutdownRequest => break 'outer Ok(()),
            }
        }
    }
}

pub async fn login_flow() -> Result<TokenResponse> {
    let mut auth_context = AuthContext::new()?;
    begin_oauth_login_flow(&mut auth_context).await?;

    if auth_context.code().is_none() {
        bail!("Login failed!");
    }

    info!("Received login...");

    let token_res = nucleus_token_exchange(&auth_context).await;
    if token_res.is_err() {
        bail!("Login failed: {}", token_res.err().unwrap().to_string());
    }

    Ok(token_res?)
}
