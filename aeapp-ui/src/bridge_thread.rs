use egui::Context;
use log::{error, info, warn};

use crate::{
    bridge::{
        game_details::game_details_request,
        get_friends::get_friends_request,
        get_games::{get_games_request, open_game_folder_request, refresh_games_request},
        login_oauth::login_oauth,
        start_game::start_game_request,
    },
    event_thread::{AeAppEventRequest, AeAppEventResponse, EventThread},
    ui_image::UIImageCacheLoaderCommand,
    views::friends_view::UIFriend,
    GameDetails, GameInfo, GameSettings,
};
use aeapp::{
    content::manager::{
        ContentManager, ContentManagerError, DownloadAction, DownloadStatus, QueuedGameBuilder,
        QueuedGameBuilderError,
    },
    core::{
        auth::storage::{AuthError, TokenError},
        launch::LaunchError,
        library::LibraryError,
        manifest::{self, ManifestError, MANIFEST_RELATIVE_PATH},
        service_layer::{
            ServiceGameImagesRequestBuilderError, ServiceHeroBackgroundImageRequestBuilderError,
            ServiceLayerError, ServicePlayer,
        },
        AeApp, AeAppCreationError, AeAppOptionsBuilder, AeAppOptionsBuilderError, LockedAeApp,
    },
    lsx::service::LSXServerError,
    rtm::RtmError,
    util::{
        native::NativeError,
        registry::{check_registry_validity, set_up_registry, RegistryError},
    },
};
use std::sync::mpsc::{SendError, TryRecvError};
use std::{
    panic,
    path::PathBuf,
    sync::mpsc::{Receiver, Sender},
    time::{Duration, SystemTime},
};

// TODO(headassbtw): integrate these all into the enums
pub struct InteractThreadLoginResponse {
    pub you: ServicePlayer,
}

pub struct InteractThreadGameListResponse {
    pub game: GameInfo,
    pub settings: GameSettings,
}

pub struct InteractThreadFriendListResponse {
    pub friend: UIFriend,
}

pub struct InteractThreadGameDetailsResponse {
    pub slug: String,
    pub response: GameDetails,
}

pub struct InteractThreadLocateGameFailure {
    pub reason: ManifestError,
    pub xml_path: String,
}

pub enum InteractThreadLocateGameResponse {
    Success,
    Error(InteractThreadLocateGameFailure),
}

pub struct InteractThreadDownloadProgressResponse {
    pub bytes: usize,
    pub bytes_total: usize,
}

pub enum AeAppLibRequest {
    StartService,
    LoginRequestOauth,
    GetGamesRequest,
    GetFriendsRequest,
    GetMembershipRequest,
    PrepareInstallRequest(u64, String, PathBuf),
    GetGameDetailsRequest(String),
    StartGameRequest(GameInfo, Option<GameSettings>),
    StopGameRequest(String),
    OpenGameFolderRequest(String),
    RefreshGamesRequest,
    ControlDownload(DownloadAction),
    InstallGameRequest(String, PathBuf),
    LocateGameRequest(String),
    ShutdownRequest,
}

pub enum AeAppLibResponse {
    LoginResponse(Result<InteractThreadLoginResponse, Box<BackendError>>),
    LoginCacheEmpty,
    ServiceNeedsStarting,
    ServiceStarted,
    LibraryReset,
    GameInfoResponse(InteractThreadGameListResponse),
    FriendInfoResponse(InteractThreadFriendListResponse),
    MembershipResponse(crate::membership::Membership),
    InstallPreparationResponse(
        u64,
        PathBuf,
        Result<aeapp::content::install::InstallSpace, String>,
    ),
    GameDetailsResponse(InteractThreadGameDetailsResponse),
    LocateGameResponse(InteractThreadLocateGameResponse),
    // Alerts, rather than responses:
    CriticalError(Box<BackendError>),
    NonFatalError(Box<BackendError>),
    ActiveGameChanged(Option<String>),
    DownloadProgressChanged(String, InteractThreadDownloadProgressResponse),
    DownloadFinished(String),
    DownloadQueueUpdate(Option<String>, Vec<String>),
    DownloadStatusChanged(DownloadStatus),
}
pub struct BridgeThread {
    pub backend_listener: Receiver<AeAppLibResponse>,
    pub backend_commander: Sender<AeAppLibRequest>,

    pub rtm_listener: Receiver<AeAppEventResponse>,
    pub rtm_commander: Sender<AeAppEventRequest>, // currently unused except for shutdown
}

#[derive(thiserror::Error, Debug)]
pub enum BackendError {
    #[error(transparent)]
    Auth(#[from] AuthError),
    #[error(transparent)]
    BackgroundServiceControl(#[from] aeapp::util::BackgroundServiceControlError),
    #[error(transparent)]
    BackgroundServiceClient(#[from] aeapp::core::error::BackgroundServiceClientError),
    #[error(transparent)]
    ContentManager(#[from] ContentManagerError),
    #[error(transparent)]
    Launch(#[from] LaunchError),
    #[error(transparent)]
    Library(#[from] LibraryError),
    #[error(transparent)]
    LSXServer(#[from] LSXServerError),
    #[error(transparent)]
    AeAppCreation(#[from] AeAppCreationError),
    #[error(transparent)]
    AeAppOptionsBuilder(#[from] AeAppOptionsBuilderError),
    #[error(transparent)]
    Native(#[from] NativeError),
    #[error(transparent)]
    QueuedGameBuilder(#[from] QueuedGameBuilderError),
    #[error(transparent)]
    RegistryError(#[from] RegistryError),
    #[error(transparent)]
    Rtm(#[from] RtmError),
    #[error(transparent)]
    SendResponse(#[from] SendError<AeAppLibResponse>),
    #[error(transparent)]
    SendImageCacheLoaderCommand(#[from] SendError<UIImageCacheLoaderCommand>),
    #[error(transparent)]
    ServiceGameImagesRequestBuilder(#[from] ServiceGameImagesRequestBuilderError),
    #[error(transparent)]
    ServiceHeroBackgroundImageRequestBuilder(#[from] ServiceHeroBackgroundImageRequestBuilderError),
    #[error(transparent)]
    ServiceLayer(#[from] ServiceLayerError),
    #[error(transparent)]
    Token(#[from] TokenError),
    #[error(transparent)]
    TryRecv(#[from] TryRecvError),

    #[error("backend-frontend communication channel disconnected")]
    ChannelDisconnected,
    #[error("tried to perform an action that requires being logged in, but was logged out")]
    LoggedOut,
    #[error("no downloadable live build is available for {0}")]
    NoLiveBuild(String),
    #[error("{0}")]
    InstallPreparation(String),
}

impl BridgeThread {
    fn update_queue(content_manager: &ContentManager, backend_responder: Sender<AeAppLibResponse>) {
        let current = if let Some(now) = content_manager.queue().current() {
            Some(now.offer_id().to_owned())
        } else {
            None
        };

        let mut queue: Vec<String> = Vec::new();

        for game in content_manager.queue().queued() {
            queue.push(game.offer_id().to_owned());
        }

        let _ = backend_responder.send(AeAppLibResponse::DownloadQueueUpdate(current, queue));
        let _ = backend_responder.send(AeAppLibResponse::DownloadStatusChanged(
            content_manager.download_status(),
        ));
    }

    pub fn new(ctx: &Context, remote_provider_channel: Sender<UIImageCacheLoaderCommand>) -> Self {
        puffin::profile_function!();
        let (backend_commander, backend_cmd_listener) = std::sync::mpsc::channel();
        let (backend_responder, backend_listener) = std::sync::mpsc::channel();

        let (rtm_commander, rtm_cmd_listener) = std::sync::mpsc::channel();
        let (rtm_responder, rtm_listener) = std::sync::mpsc::channel();
        let context = ctx.clone();

        tokio::task::spawn(async move {
            let die_fallback_transmitter = backend_responder.clone();
            //panic::set_hook(Box::new( |_| {}));
            let result = BridgeThread::run(
                backend_cmd_listener,
                backend_responder,
                rtm_cmd_listener,
                rtm_responder,
                remote_provider_channel,
                &context,
            )
            .await;
            if let Err(err) = result {
                die_fallback_transmitter
                    .send(AeAppLibResponse::CriticalError(Box::from(err)))
                    .unwrap();
            } else {
                info!("Interact thread shut down")
            }
        });

        Self {
            backend_listener,
            backend_commander,
            rtm_listener,
            rtm_commander,
        }
    }

    async fn run(
        backend_cmd_listener: Receiver<AeAppLibRequest>,
        backend_responder: Sender<AeAppLibResponse>,
        rtm_cmd_listener: Receiver<AeAppEventRequest>,
        rtm_responder: Sender<AeAppEventResponse>,
        remote_provider_channel: Sender<UIImageCacheLoaderCommand>,
        ctx: &Context,
    ) -> Result<(), BackendError> {
        // first things first check registry
        // the flow is different for windows/linux but windows needs an extra user prompt,
        // so we're doing both here, instead of selectively cfg'd functions!
        #[cfg(not(windows))]
        {
            if let Err(err) = check_registry_validity() {
                warn!("{}, fixing...", err);
                set_up_registry()?;
            }
        }
        #[cfg(windows)]
        {
            use aeapp::{
                core::background_service::request_registry_setup,
                util::{
                    registry::check_registry_validity,
                    service::{
                        is_service_running, is_service_valid, register_service_user, start_service,
                    },
                },
            };
            if !is_elevated::is_elevated() {
                if !is_service_valid()? {
                    info!("Installing service...");
                    backend_responder.send(AeAppLibResponse::ServiceNeedsStarting)?;
                    'wait_for_user_to_authorize: loop {
                        let request = backend_cmd_listener.try_recv();
                        if request.is_err() {
                            tokio::time::sleep(Duration::from_millis(10)).await;
                            continue;
                        }

                        match request.unwrap() {
                            AeAppLibRequest::StartService => {
                                register_service_user()?;
                                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                                break 'wait_for_user_to_authorize;
                            }
                            AeAppLibRequest::ShutdownRequest => return Ok(()),
                            _ => {}
                        }
                    }
                }

                if !is_service_running()? {
                    info!("Starting service...");
                    start_service().await?;
                }
            }

            if let Err(err) = check_registry_validity() {
                warn!("{}, fixing...", err);
                request_registry_setup().await?;
            }
        }
        let aeapp_arc: LockedAeApp = AeApp::new_with_options(
            AeAppOptionsBuilder::default()
                .dummy_local_user(false)
                .load_auth_storage(true)
                .build()?,
        )
        .await?;

        let logged_in = {
            let aeapp = aeapp_arc.lock().await;
            aeapp.start_lsx(aeapp_arc.clone()).await?;
            info!("LSX started");

            let mut auth_storage = aeapp.auth_storage().lock().await;
            auth_storage.logged_in().await?
        };

        if !logged_in {
            backend_responder.send(AeAppLibResponse::LoginCacheEmpty)?;
            'outer: loop {
                let request = backend_cmd_listener.try_recv();
                if request.is_err() {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    continue;
                }

                match request? {
                    AeAppLibRequest::LoginRequestOauth => {
                        let channel = backend_responder.clone();
                        let aeapp = aeapp_arc.clone();
                        let context = ctx.clone();
                        match login_oauth(aeapp, channel, &context).await {
                            Ok(()) => break 'outer,
                            Err(error) => {
                                backend_responder
                                    .send(AeAppLibResponse::LoginResponse(Err(Box::new(error))))?;
                                ctx.request_repaint();
                            }
                        }
                    }
                    AeAppLibRequest::ShutdownRequest => return Ok(()),
                    _ => {}
                }
            }
        }

        {
            let aeapp = aeapp_arc.lock().await;
            let user = aeapp.local_user().await?;

            if logged_in {
                let message = AeAppLibResponse::LoginResponse(Ok(InteractThreadLoginResponse {
                    you: user.player().as_ref().unwrap().to_owned(),
                }));
                backend_responder.send(message)?;
            }
            let res = remote_provider_channel.send(UIImageCacheLoaderCommand::ProvideRemote(
                crate::ui_image::UIImageType::Avatar(user.id().to_string()),
                user.player()
                    .as_ref()
                    .ok_or(ServiceLayerError::MissingField)?
                    .avatar()
                    .as_ref()
                    .ok_or(ServiceLayerError::MissingField)?
                    .medium()
                    .path()
                    .to_string(),
            ));
            if let Err(err) = res {
                error!("failed to send user pfp to loader: {:?}", err);
            }
            ctx.request_repaint();
        }

        {
            let mut aeapp = aeapp_arc.lock().await;
            Self::update_queue(aeapp.content_manager(), backend_responder.clone());
        }
        let _ = EventThread::new(
            &ctx.clone(),
            aeapp_arc.clone(),
            rtm_cmd_listener,
            rtm_responder,
        );

        let mut future = SystemTime::now();
        future = future.checked_add(Duration::from_millis(50)).unwrap();
        let mut playing_cache: Option<String> = None;
        let mut next_download_status = SystemTime::now();
        'outer: loop {
            let now = SystemTime::now();
            if now >= future {
                // this sucks but it's non-blocking so oh well what are you going to do about it! it's on a non-ui thread anyway, i'm wasteful with it
                future = now.checked_add(Duration::from_millis(50)).unwrap();

                let mut aeapp = aeapp_arc.lock().await;
                aeapp.update().await;
                let now_playing = aeapp.playing();

                if let Some(ctx) = now_playing {
                    if let Some(offer) = ctx.offer() {
                        if playing_cache.is_none() {
                            playing_cache = Some(offer.slug().clone());
                            backend_responder.send(AeAppLibResponse::ActiveGameChanged(Some(
                                offer.slug().clone(),
                            )))?;
                        }
                    }
                } else {
                    if playing_cache.is_some() {
                        playing_cache = None;
                        backend_responder.send(AeAppLibResponse::ActiveGameChanged(None)).unwrap();
                    };
                }

                if let Some(dl) = aeapp.content_manager().current() {
                    backend_responder.send(AeAppLibResponse::DownloadProgressChanged(
                        dl.offer_id().to_string(),
                        InteractThreadDownloadProgressResponse {
                            bytes: dl.bytes_downloaded(),
                            bytes_total: dl.bytes_total(),
                        },
                    ))?;
                }

                if now >= next_download_status {
                    Self::update_queue(aeapp.content_manager(), backend_responder.clone());
                    next_download_status = now.checked_add(Duration::from_secs(1)).unwrap();
                    ctx.request_repaint();
                }
                for ev in aeapp.consume_pending_events() {
                    match ev {
                        aeapp::core::AeAppEvent::ReceivedLSXRequest(_, _) => {}
                        aeapp::core::AeAppEvent::InstallFinished(offer_id) => {
                            backend_responder.send(AeAppLibResponse::DownloadFinished(offer_id))?;
                            Self::update_queue(aeapp.content_manager(), backend_responder.clone());
                        }
                    }
                }
            }
            let request = backend_cmd_listener.try_recv();
            if request.is_err() {
                tokio::time::sleep(Duration::from_millis(10)).await;
                continue;
            }

            let action = match request? {
                AeAppLibRequest::LoginRequestOauth | AeAppLibRequest::StartService => {
                    error!("bro tried to log in twice");
                    Ok(())
                }
                AeAppLibRequest::GetGamesRequest => {
                    let channel = backend_responder.clone();
                    let channel1 = remote_provider_channel.clone();
                    let aeapp = aeapp_arc.clone();
                    let context = ctx.clone();
                    async move { get_games_request(aeapp, channel, channel1, &context).await }.await
                }
                AeAppLibRequest::GetFriendsRequest => {
                    let channel = backend_responder.clone();
                    let channel1 = remote_provider_channel.clone();
                    let aeapp = aeapp_arc.clone();
                    let context = ctx.clone();
                    async move { get_friends_request(aeapp, channel, channel1, &context).await }
                        .await
                }
                AeAppLibRequest::GetMembershipRequest => {
                    let app = aeapp_arc.clone();
                    let channel = backend_responder.clone();
                    let context = ctx.clone();
                    tokio::spawn(async move {
                        let membership = tokio::time::timeout(
                            Duration::from_secs(30),
                            crate::membership::load(app),
                        )
                        .await
                        .ok()
                        .and_then(Result::ok)
                        .unwrap_or(crate::membership::Membership::Unknown);
                        let _ = channel.send(AeAppLibResponse::MembershipResponse(membership));
                        context.request_repaint();
                    });
                    Ok(())
                }
                AeAppLibRequest::PrepareInstallRequest(id, offer, path) => {
                    let auth = aeapp_arc.lock().await.auth_storage().clone();
                    let channel = backend_responder.clone();
                    let context = ctx.clone();
                    tokio::spawn(async move {
                        let service = aeapp::content::ContentService::new(auth);
                        let result = tokio::time::timeout(
                            Duration::from_secs(60),
                            aeapp::content::install::prepare_install(
                                &service,
                                &offer,
                                path.clone(),
                            ),
                        )
                        .await
                        .unwrap_or_else(|_| {
                            Err("Disk-space calculation timed out. Try again.".into())
                        });
                        let _ = channel.send(AeAppLibResponse::InstallPreparationResponse(
                            id, path, result,
                        ));
                        context.request_repaint();
                    });
                    Ok(())
                }
                AeAppLibRequest::GetGameDetailsRequest(slug) => {
                    let channel = backend_responder.clone();
                    let aeapp = aeapp_arc.clone();
                    let context = ctx.clone();
                    async move { game_details_request(aeapp, slug.clone(), channel, &context).await }.await
                }
                AeAppLibRequest::LocateGameRequest(path) => {
                    #[cfg(unix)]
                    aeapp::core::launch::aeapp_linux_setup().await?;
                    let mut path = path;
                    if path.ends_with("/") || path.ends_with("\\") {
                        path.remove(path.len() - 1);
                    }
                    let path = PathBuf::from(path);
                    let manifest = manifest::read(path.join(MANIFEST_RELATIVE_PATH)).await;
                    if let Ok(manifest) = manifest {
                        let guh = manifest.run_touchup(&path).await;
                        if let Err(err) = guh {
                            let _ = backend_responder.send(AeAppLibResponse::LocateGameResponse(
                                InteractThreadLocateGameResponse::Error(
                                    InteractThreadLocateGameFailure {
                                        reason: err,
                                        xml_path: path
                                            .join(MANIFEST_RELATIVE_PATH)
                                            .to_str()
                                            .unwrap()
                                            .to_string(),
                                    },
                                ),
                            ));
                        } else {
                            let _ = backend_responder.send(AeAppLibResponse::LocateGameResponse(
                                InteractThreadLocateGameResponse::Success,
                            ));
                        }
                    } else {
                        let _ = backend_responder.send(AeAppLibResponse::LocateGameResponse(
                            InteractThreadLocateGameResponse::Error(
                                InteractThreadLocateGameFailure {
                                    reason: manifest.unwrap_err(),
                                    xml_path: path
                                        .join(MANIFEST_RELATIVE_PATH)
                                        .to_str()
                                        .unwrap()
                                        .to_string(),
                                },
                            ),
                        ));
                    }
                    info!("finished locating");
                    ctx.request_repaint();
                    Ok(())
                }
                AeAppLibRequest::InstallGameRequest(offer, path) => {
                    let app = aeapp_arc.clone();
                    let channel = backend_responder.clone();
                    let context = ctx.clone();
                    tokio::spawn(async move {
                        let result = async {
                            let auth = app.lock().await.auth_storage().clone();
                            let service = aeapp::content::ContentService::new(auth);
                            let space = tokio::time::timeout(
                                Duration::from_secs(60),
                                aeapp::content::install::prepare_install(
                                    &service,
                                    &offer,
                                    path.clone(),
                                ),
                            )
                            .await
                            .map_err(|_| {
                                BackendError::InstallPreparation(
                                    "Disk-space calculation timed out. Try again.".into(),
                                )
                            })?
                            .map_err(BackendError::InstallPreparation)?;
                            if space.available < space.required {
                                return Err(BackendError::InstallPreparation(
                                    "There is not enough disk space in the selected game folder."
                                        .into(),
                                ));
                            }
                            let mut aeapp = app.lock().await;
                            let game = QueuedGameBuilder::default()
                                .offer_id(offer)
                                .build_id(space.build_id)
                                .path(path)
                                .build()?;
                            aeapp.content_manager().add_install(game).await?;
                            Ok::<(), BackendError>(())
                        }
                        .await;
                        let mut aeapp = app.lock().await;
                        Self::update_queue(aeapp.content_manager(), channel.clone());
                        if let Err(error) = result {
                            let _ = channel.send(AeAppLibResponse::NonFatalError(Box::new(error)));
                        }
                        context.request_repaint();
                    });
                    Ok(())
                }
                AeAppLibRequest::ControlDownload(action) => {
                    let mut aeapp = aeapp_arc.lock().await;
                    let result = aeapp
                        .content_manager()
                        .control_download(action)
                        .await
                        .map_err(BackendError::ContentManager);
                    Self::update_queue(aeapp.content_manager(), backend_responder.clone());
                    ctx.request_repaint();
                    result
                }
                AeAppLibRequest::StopGameRequest(offer) => {
                    aeapp::core::launch::stop_game(aeapp_arc.clone(), &offer)
                        .await
                        .map_err(BackendError::Launch)
                }
                AeAppLibRequest::OpenGameFolderRequest(offer) => {
                    open_game_folder_request(aeapp_arc.clone(), &offer).await
                }
                AeAppLibRequest::RefreshGamesRequest => {
                    refresh_games_request(
                        aeapp_arc.clone(),
                        backend_responder.clone(),
                        remote_provider_channel.clone(),
                        ctx,
                    )
                    .await
                }
                AeAppLibRequest::StartGameRequest(info, settings) => {
                    match start_game_request(aeapp_arc.clone(), info, settings).await {
                        Ok(()) => Ok(()),
                        Err(error) => {
                            // A bad per-game setting must not disconnect the account/backend.
                            let aeapp = aeapp_arc.lock().await;
                            let active = aeapp.playing().as_ref().and_then(|game| {
                                game.offer().as_ref().map(|offer| offer.slug().to_owned())
                            });
                            backend_responder.send(AeAppLibResponse::ActiveGameChanged(active))?;
                            Err(BackendError::Launch(error))
                        }
                    }
                }
                AeAppLibRequest::ShutdownRequest => break 'outer Ok(()), //TODO: kill the bridge thread
            };
            if let Err(err) = action {
                let _ = backend_responder.send(AeAppLibResponse::NonFatalError(Box::from(err)));
            }

            puffin::GlobalProfiler::lock().new_frame();
        }
    }
}
