use crate::{
    bridge_thread::{AeAppLibResponse, BackendError, InteractThreadGameListResponse},
    ui_image::UIImageCacheLoaderCommand,
    GameDetailsWrapper, GameInfo, GameVersionInfo,
};
use aeapp::{
    core::{
        launch::LaunchError,
        service_layer::{
            ServiceGame, ServiceGameHub, ServiceGameHubCollection, ServiceGameImagesRequestBuilder,
            ServiceHeroBackgroundImageRequestBuilder, ServiceLayerClient,
            SERVICE_REQUEST_GAMEIMAGES, SERVICE_REQUEST_GETHEROBACKGROUNDIMAGE,
        },
        LockedAeApp,
    },
    util::native::{aeapp_dir, NativeError, SafeParent},
};
use egui::Context;
use log::{debug, error, info};
use std::{fs, sync::mpsc::Sender};

pub async fn refresh_games_request(
    aeapp_arc: LockedAeApp,
    channel: Sender<AeAppLibResponse>,
    image_channel: Sender<UIImageCacheLoaderCommand>,
    ctx: &Context,
) -> Result<(), BackendError> {
    {
        let mut aeapp = aeapp_arc.lock().await;
        aeapp.mut_library().refresh().await?;
    }
    #[cfg(unix)]
    aeapp::unix::wine::invalidate_aeapp_wine_registry().await;
    channel.send(AeAppLibResponse::LibraryReset)?;
    get_games_request(aeapp_arc, channel, image_channel, ctx).await
}

pub async fn open_game_folder_request(
    aeapp_arc: LockedAeApp,
    offer_id: &str,
) -> Result<(), BackendError> {
    let folder = {
        let mut aeapp = aeapp_arc.lock().await;
        let offer = aeapp
            .mut_library()
            .game_by_base_offer(offer_id)
            .await?
            .ok_or_else(|| LaunchError::NoOfferFound(offer_id.to_owned()))?;
        if !offer.is_installed().await {
            return Err(LaunchError::NotInstalled(offer_id.to_owned()).into());
        }
        let executable = offer.execute_path(false).await?;
        executable.safe_parent()?.canonicalize().map_err(NativeError::from)?
    };
    #[cfg(target_os = "linux")]
    let mut command = std::process::Command::new("xdg-open");
    #[cfg(target_os = "macos")]
    let mut command = std::process::Command::new("open");
    #[cfg(windows)]
    let mut command = std::process::Command::new("explorer.exe");
    command.arg(folder).spawn().map_err(NativeError::from)?;
    Ok(())
}

fn get_preferred_bg_hero(heroes: &Option<ServiceGameHubCollection>) -> Option<String> {
    let heroes = match heroes {
        Some(h) => h.items().get(0),
        None => return None,
    };

    let bg = match heroes {
        Some(bg) => bg.hero_background(),
        None => {
            return None;
        }
    };

    if let Some(img) = bg.aspect_16x9_image() {
        return Some(img.path().clone());
    }

    if let Some(img) = bg.aspect_2x1_image() {
        return Some(img.path().clone());
    }

    if let Some(img) = bg.aspect_10x3_image() {
        return Some(img.path().clone());
    }

    None
}

async fn get_preferred_hero_image(images: &Option<ServiceGame>) -> Option<String> {
    let key_art = match images {
        Some(images) => images.key_art(),
        None => {
            return None;
        }
    };

    let key_art = match key_art {
        Some(key_art) => key_art,
        None => {
            return None;
        }
    };

    if let Some(img) = key_art.aspect_10x3_image() {
        return Some(img.path().clone());
    }

    if let Some(img) = key_art.aspect_2x1_image() {
        return Some(img.path().clone());
    }

    if let Some(img) = key_art.aspect_16x9_image() {
        return Some(img.path().clone());
    }

    None
}

fn get_logo_image(images: &Option<ServiceGame>) -> Option<String> {
    let logo_set = match images {
        Some(images) => images.primary_logo(),
        None => {
            return None;
        }
    };

    let largest_logo = match logo_set {
        Some(logo) => logo.largest_image(),
        None => {
            return None;
        }
    };

    match largest_logo {
        Some(logo) => Some(logo.path().clone()),
        None => None,
    }
}

async fn handle_images(
    slug: String,
    locale: String,
    has_hero: bool,
    has_logo: bool,
    has_background: bool,
    channel: Sender<UIImageCacheLoaderCommand>,
    service_layer: ServiceLayerClient,
) -> Result<(), BackendError> {
    debug!("handling image downloads for {}", &slug);
    let images_0 = if has_hero && has_logo {
        None
    } else {
        Some(
            service_layer.request(
                SERVICE_REQUEST_GAMEIMAGES,
                ServiceGameImagesRequestBuilder::default()
                    .should_fetch_context_image(!has_logo)
                    .should_fetch_backdrop_images(!has_hero)
                    .game_slug(slug.clone())
                    .locale(locale.clone())
                    .build()?,
            ),
        )
    };
    let images_1 = if has_background {
        None
    } else {
        Some(
            service_layer.request(
                SERVICE_REQUEST_GETHEROBACKGROUNDIMAGE,
                ServiceHeroBackgroundImageRequestBuilder::default()
                    .game_slug(slug.clone())
                    .locale(locale.clone())
                    .build()?,
            ),
        )
    };

    let images_0 = if let Some(images) = images_0 {
        images.await?
    } else {
        None
    };

    if !has_hero {
        if let Some(hero) = get_preferred_hero_image(&images_0).await {
            channel.send(UIImageCacheLoaderCommand::ProvideRemote(
                crate::ui_image::UIImageType::Hero(slug.clone()),
                hero,
            ))?
        }
    }

    if !has_logo {
        if let Some(logo) = get_logo_image(&images_0) {
            channel.send(UIImageCacheLoaderCommand::ProvideRemote(
                crate::ui_image::UIImageType::Logo(slug.clone()),
                logo,
            ))?
        } else {
            channel.send(UIImageCacheLoaderCommand::Stub(
                crate::ui_image::UIImageType::Logo(slug.clone()),
            ))?
        }
    }

    // I'm doing it down here because this call has a tendency to fail at the time of writing.
    // If it's down here it only takes down the background image, and not the logo/hero.
    let images_1 = if let Some(images) = images_1 {
        images.await?
    } else {
        None
    };

    if !has_background {
        if let Some(background_image) = get_preferred_bg_hero(&images_1) {
            channel.send(UIImageCacheLoaderCommand::ProvideRemote(
                crate::ui_image::UIImageType::Background(slug),
                background_image,
            ))?
        }
    }

    Ok(())
}

pub async fn get_games_request(
    aeapp_arc: LockedAeApp,
    channel: Sender<AeAppLibResponse>,
    channel1: Sender<UIImageCacheLoaderCommand>,
    ctx: &Context,
) -> Result<(), BackendError> {
    debug!("received request to load games");
    let mut aeapp = aeapp_arc.lock().await;
    let service_layer = aeapp.service_layer().clone();
    let locale = aeapp.locale().short_str().to_owned();
    let logged_in = aeapp.auth_storage().lock().await.current().is_some();
    if !logged_in {
        return Err(BackendError::LoggedOut);
    }

    let owned_games = aeapp.mut_library().games().await?;

    for game in owned_games {
        let slug = game.base_offer().slug().clone();
        info!("processing {}", &slug);

        let downloads = game.base_offer().offer().downloads();
        let download = downloads
            .iter()
            .find(|item| item.download_type() == "LIVE")
            .or_else(|| downloads.first());

        let version = if let Ok(version) = game.base_offer().installed_version().await {
            version
        } else {
            "Unknown".to_owned()
        };

        let game_info = GameInfo {
            slug: slug.clone(),
            offer: game.base_offer().offer().offer_id().to_string(),
            name: game.name(),
            details: GameDetailsWrapper::Unloaded,
            version: GameVersionInfo {
                installed: version.clone(),
                latest: download.map(|item| item.version().to_owned()).unwrap_or(version),
                mandatory: download.is_some_and(|item| *item.treat_updates_as_mandatory()),
            },
            dlc: game.extra_offers().clone(),
            installed: game.base_offer().is_installed().await,
            has_cloud_saves: game.base_offer().offer().has_cloud_save(),
        };
        let slug = game_info.slug.clone();
        let settings = crate::GameSettings::new();
        let res = AeAppLibResponse::GameInfoResponse(InteractThreadGameListResponse {
            game: game_info,
            settings,
        });
        channel.send(res)?;

        let bg = aeapp_dir()?.join("cache/ui/images/").join(&slug).join("background.jpg");
        let game_hero = aeapp_dir()?.join("cache/ui/images/").join(&slug).join("hero.jpg");
        let game_logo = aeapp_dir()?.join("cache/ui/images/").join(&slug).join("logo.png");
        let has_hero = fs::metadata(&game_hero).is_ok();
        let has_logo = fs::metadata(&game_logo).is_ok();
        let has_background = fs::metadata(&bg).is_ok();

        if !has_hero || !has_logo || !has_background {
            //we're like 20 tasks deep i swear but this shit's gonna be real fast, trust
            let slug_send = slug.clone();
            let locale_send = locale.clone();
            let channel_send = channel1.clone();
            let service_layer_send = service_layer.clone();
            tokio::task::spawn(async move {
                handle_images(
                    slug_send,
                    locale_send,
                    has_hero,
                    has_logo,
                    has_background,
                    channel_send,
                    service_layer_send,
                )
                .await
            });
            tokio::task::yield_now().await;
        }

        egui::Context::request_repaint(&ctx);
    }
    Ok(())
}
