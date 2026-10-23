use base64::{engine::general_purpose, Engine};
use derive_getters::Getters;
use log::{error, info};
use std::{
    env,
    fmt::Display,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::{
    process::{Child, Command},
    sync::Mutex,
};
use uuid::Uuid;

use crate::{
    core::{
        auth::{
            context::AuthContext,
            nucleus_auth_exchange,
            storage::{AuthError, TokenError},
        },
        clients::JUNO_PC_CLIENT_ID,
        cloudsync::{CloudSyncError, CloudSyncLockMode},
        library::{LibraryError, OwnedOffer},
        service_layer::ServiceLayerError,
        AeApp,
    },
    ooa::{needs_license_update, request_and_save_license, LicenseAuth, LicenseError},
    util::{
        native::{NativeError, SafeParent, SafeStr},
        registry::bootstrap_path,
        simple_crypto,
    },
};
use thiserror::Error;

#[cfg(unix)]
use crate::unix::fs::case_insensitive_path;

use serde::{Deserialize, Serialize};

#[derive(Error, Debug)]
pub enum LaunchError {
    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Auth(#[from] AuthError),
    #[error(transparent)]
    CloudSync(#[from] CloudSyncError),
    #[error(transparent)]
    Library(#[from] LibraryError),
    #[error(transparent)]
    License(#[from] LicenseError),
    #[error(transparent)]
    Native(#[from] NativeError),
    #[error(transparent)]
    ServiceLayer(#[from] ServiceLayerError),
    #[error(transparent)]
    Token(#[from] TokenError),

    #[error("no offer was found for id `{0}`")]
    NoOfferFound(String),
    #[error("offline mode is not yet supported")]
    Offline,
    #[error("game path must be specified when launching in OnlineOffline mode")]
    GamePathOffline,
    #[error("game path not found")]
    GamePath,
    #[error("`{0}` is not installed")]
    NotInstalled(String),
    #[error("bootstrap was not found! Please re-install aeapp")]
    BootstrapMissing,
    #[error("a game is already running ({0}); stop it before launching another")]
    AlreadyRunning(String),
    #[error("there is no active game for offer `{0}`")]
    NoActiveGame(String),
    #[error("could not safely identify this game's process; it may still be starting. Retry Stop or close the game from its own menu")]
    StopProcessNotFound,
    #[error("the game process could not be stopped; close it from the game's own menu")]
    StopSignalFailed,
    #[error("stopping a game from this client is currently supported only on Linux")]
    StopUnsupported,
    #[error(
        "content ID (`{0}`) was specified as an offer ID when launching in OnlineOffline mode"
    )]
    ContentIdAsOfferId(String),
}

pub enum StartupStage {
    Launch,
    ConnectionEstablished,
}

pub struct LibraryInjection {
    pub path: PathBuf,
    pub stage: StartupStage,
}

pub struct LaunchOptions {
    pub path_override: Option<String>,
    pub arguments: Vec<String>,
    pub cloud_saves: bool,
    pub proton: ProtonLaunchOptions,
}

/// Per-game settings passed only to the Proton child process.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProtonLaunchOptions {
    /// A local Proton installation directory; `None` uses managed Proton.
    pub proton_path: Option<PathBuf>,
    pub proton_log: bool,
    pub disable_esync: bool,
    pub disable_fsync: bool,
}

pub enum LaunchMode {
    /// Completely offline, relies on cached license files and user IDs
    Offline(String), // Offer ID
    /// Online, makes requests about the user and licensing
    Online(String), // Offer ID
    /// Online, but only for license requests; everything else uses dummy offer and user IDs
    /// Content ID, Game executable path, and username/password must be specified
    OnlineOffline(String, String, String), // Content ID, Persona, Password
}

impl LaunchMode {
    // What an awful name
    pub fn is_online_offline(&self) -> bool {
        match self {
            LaunchMode::OnlineOffline(_, _, _) => true,
            _ => false,
        }
    }
}

#[derive(Getters)]
pub struct ActiveGameContext {
    launch_id: String,
    game_path: String,
    content_id: String,
    offer: Option<OwnedOffer>,
    mode: LaunchMode,
    injections: Vec<LibraryInjection>,
    cloud_saves: bool,
    proton: ProtonLaunchOptions,
    process: Child,
    started: bool,
}

impl ActiveGameContext {
    pub fn new(
        launch_id: &str,
        game_path: &str,
        cloud_saves: bool,
        content_id: &str,
        offer: Option<OwnedOffer>,
        mode: LaunchMode,
        process: Child,
    ) -> Self {
        Self {
            launch_id: launch_id.to_owned(),
            game_path: game_path.to_owned(),
            content_id: content_id.to_owned(),
            offer,
            mode,
            injections: Vec::new(),
            cloud_saves,
            proton: ProtonLaunchOptions::default(),
            process,
            started: false,
        }
    }

    pub fn set_started(&mut self) {
        self.started = true;
    }

    pub fn with_proton_options(mut self, options: ProtonLaunchOptions) -> Self {
        self.proton = options;
        self
    }

    pub fn process_mut(&mut self) -> &mut Child {
        &mut self.process
    }
}

#[derive(Default, Serialize, Deserialize)]
pub struct BootstrapLaunchArgs {
    pub path: String,
    pub args: Vec<String>,
    #[serde(default)]
    pub proton: ProtonLaunchOptions,
}

impl Display for LaunchMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LaunchMode::Offline(offer_id) => write!(f, "{}", offer_id),
            LaunchMode::Online(offer_id) => write!(f, "{}", offer_id),
            LaunchMode::OnlineOffline(content_id, _, _) => write!(f, "{}", content_id),
        }
    }
}

pub async fn start_game(
    aeapp_arc: Arc<Mutex<AeApp>>,
    mode: LaunchMode,
    options: LaunchOptions,
) -> Result<(), LaunchError> {
    let mut aeapp = aeapp_arc.lock().await;
    ensure_launch_available(aeapp.playing.as_ref().map(|game| game.mode()))?;

    // Reject an invalid custom runner before authentication or runtime setup.
    #[cfg(unix)]
    crate::unix::wine::resolve_proton_path(&options.proton)?;

    info!("Initiating game launch with {}...", mode);

    if let LaunchMode::OnlineOffline(ref content_id, _, _) = mode {
        if options.path_override.is_none() {
            return Err(LaunchError::GamePathOffline);
        }

        if content_id.starts_with("Origin.OFR") {
            return Err(LaunchError::ContentIdAsOfferId(content_id.clone()));
        }
    }

    let (content_id, online_offline, offer, access_token) =
        if let LaunchMode::Online(ref offer_id) = mode {
            let access_token = &aeapp.access_token().await?;
            let offer = match aeapp.mut_library().game_by_base_offer(offer_id).await? {
                Some(offer) => offer,
                None => return Err(LaunchError::NoOfferFound(offer_id.clone())),
            };

            if !offer.is_installed().await {
                return Err(LaunchError::NotInstalled(offer.offer_id().clone()));
            }

            let content_id = offer.offer().content_id().to_owned();

            (
                content_id,
                false,
                Some(offer.clone()),
                access_token.to_owned(),
            )
        } else if let LaunchMode::OnlineOffline(ref content_id, _, _) = mode {
            (content_id.to_owned(), true, None, String::new())
        } else {
            return Err(LaunchError::Offline);
        };

    // Need to move this into AeApp and have a "current game" system
    let path = if let Some(game_path_override) = options.path_override {
        PathBuf::from(&game_path_override)
    } else if !online_offline {
        match offer {
            Some(ref offer) => offer.execute_path(false).await?.clone(),
            None => return Err(LaunchError::NoOfferFound("Unknown".to_string())),
        }
    } else {
        return Err(LaunchError::GamePath);
    };

    #[cfg(unix)]
    let path = case_insensitive_path(path);
    let dir = path.safe_parent()?.safe_str()?;
    let path = path.safe_str()?;
    info!("Game path: {}", path);

    #[cfg(unix)]
    aeapp_linux_setup_with_options(&options.proton).await?;

    match mode {
        LaunchMode::Offline(_) => {}
        LaunchMode::Online(_) => {
            let auth = LicenseAuth::AccessToken(aeapp.access_token().await?);

            let offer = offer.as_ref().unwrap();
            if needs_license_update(&content_id).await? {
                info!(
                    "Requesting new game license for {}...",
                    offer.offer().display_name()
                );

                request_and_save_license(&auth, &content_id, path.to_owned().into()).await?;
            } else {
                info!("Existing game license is still valid, not updating");
            }

            if options.cloud_saves && offer.offer().has_cloud_save() {
                info!("Syncing with cloud save...");

                let result = aeapp
                    .cloud_sync()
                    .obtain_lock(offer, CloudSyncLockMode::Read)
                    .await;
                if let Err(err) = result {
                    error!("Failed to obtain CloudSync read lock: {}", err);
                } else {
                    let lock = result?;

                    let result = lock.sync_files().await;
                    if let Err(err) = result {
                        error!("Failed to sync cloud save: {}", err);
                    } else {
                        info!("Cloud save synced");
                    }

                    lock.release().await?;
                }
            }
        }
        LaunchMode::OnlineOffline(_, ref persona, ref password) => {
            let auth = LicenseAuth::Direct(persona.to_owned(), password.to_owned());

            if needs_license_update(&content_id).await? {
                request_and_save_license(&auth, &content_id, path.to_owned().into()).await?;
            } else {
                info!("Existing game license is still valid, not updating");
            }
        }
    }

    let mut game_args = options.arguments.clone();

    // Append args from env
    if let Ok(args) = env::var("AEAPP_LAUNCH_ARGS") {
        game_args.append(&mut parse_arguments(args.as_str()));
    }

    if !bootstrap_path()?.exists() {
        return Err(LaunchError::BootstrapMissing);
    }

    let mut child = Command::new(bootstrap_path()?);
    child.arg("launch");

    let bootstrap_args = BootstrapLaunchArgs {
        path: path.to_string(),
        args: game_args,
        proton: options.proton.clone(),
    };

    let b64 = general_purpose::STANDARD.encode(serde_json::to_string(&bootstrap_args)?);
    child.arg(b64);

    let user = aeapp.local_user().await?;
    let launch_id = Uuid::new_v4().to_string();

    child
        .current_dir(PathBuf::from(path).safe_parent()?)
        .env("MXLaunchId", launch_id.to_owned())
        .env("EAAuthCode", "unavailable")
        .env("EAEgsProxyIpcPort", "0")
        .env("EAEntitlementSource", "EA")
        .env("EAExternalSource", "EA")
        .env("EAFreeTrialGame", "false")
        .env("EAGameLocale", aeapp.locale.full_str())
        .env("EAGenericAuthToken", access_token.to_owned())
        .env("EALaunchCode", "unavailable")
        .env("EALaunchOwner", "EA")
        .env(
            "EALaunchEAID",
            user.player()
                .as_ref()
                .ok_or(ServiceLayerError::MissingField)?
                .display_name(),
        )
        .env("EALaunchEnv", "production")
        .env("EALaunchOfflineMode", "false")
        .env("EALsxPort", aeapp.lsx_port.to_string())
        .env(
            "EARtPLaunchCode",
            simple_crypto::rtp_handshake().to_string(),
        )
        .env("EASecureLaunchTokenTemp", user.id())
        .env("EASteamProxyIpcPort", "0")
        .env("OriginSessionKey", launch_id.clone())
        .env("ContentId", content_id.clone())
        .env("EAOnErrorExitRetCode", "1");

    match mode {
        LaunchMode::Offline(_) => todo!(),
        LaunchMode::Online(ref offer_id) => {
            let short_token = request_opaque_ooa_token(&access_token).await?;

            child
                .env("EAConnectionId", offer_id.clone())
                .env("EALicenseToken", offer_id.clone())
                .env("EALaunchUserAuthToken", short_token)
                .env("EAAccessTokenJWS", access_token);
        }
        LaunchMode::OnlineOffline(_, ref persona, ref password) => {
            child
                .env("EALaunchOOAUserEmail", persona)
                .env("EALaunchOOAUserPass", password)
                // Given this is probably running headlessly, don't show a UI on error
                .env("EAOnErrorExitRetCode", "1");
        }
    };

    let child = child.spawn().map_err(NativeError::from)?;

    aeapp.playing = Some(
        ActiveGameContext::new(
            &launch_id,
            dir,
            options.cloud_saves,
            &content_id,
            offer,
            mode,
            child,
        )
        .with_proton_options(options.proton),
    );

    Ok(())
}

fn ensure_launch_available(active_mode: Option<&LaunchMode>) -> Result<(), LaunchError> {
    if let Some(mode) = active_mode {
        return Err(LaunchError::AlreadyRunning(mode.to_string()));
    }
    Ok(())
}

/// Request termination of only the Windows game processes belonging to this launch.
/// The normal game-exit path still owns process reaping, presence and cloud-save sync.
pub async fn stop_game(aeapp_arc: Arc<Mutex<AeApp>>, offer_id: &str) -> Result<(), LaunchError> {
    let (launch_id, game_dir) = {
        let aeapp = aeapp_arc.lock().await;
        let active = aeapp
            .playing
            .as_ref()
            .filter(|game| {
                game.offer()
                    .as_ref()
                    .is_some_and(|offer| offer.offer_id() == offer_id)
            })
            .ok_or_else(|| LaunchError::NoActiveGame(offer_id.to_owned()))?;
        (
            active.launch_id().clone(),
            PathBuf::from(active.game_path().as_str()),
        )
    };

    #[cfg(target_os = "linux")]
    {
        use sysinfo::{ProcessExt, Signal, System, SystemExt};

        let system = System::new_all();
        let mut found = false;
        let mut requested = false;
        for process in system.processes().values() {
            if !matches_launched_game_process(
                &launch_id,
                &game_dir,
                process.cmd().first().map(String::as_str),
                process.cwd(),
                process.environ(),
            ) {
                continue;
            }
            found = true;
            requested |= process.kill_with(Signal::Term) == Some(true);
        }
        if !found {
            return Err(LaunchError::StopProcessNotFound);
        }
        if !requested {
            return Err(LaunchError::StopSignalFailed);
        }
        info!("Requested stop for active game {}", offer_id);
        Ok(())
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (launch_id, game_dir);
        Err(LaunchError::StopUnsupported)
    }
}

#[cfg(target_os = "linux")]
fn matches_launched_game_process(
    launch_id: &str,
    game_dir: &Path,
    command: Option<&str>,
    working_dir: &Path,
    environment: &[String],
) -> bool {
    if launch_id.is_empty() || !game_dir.is_absolute() || game_dir == Path::new("/") {
        return false;
    }
    let launch_marker = format!("MXLaunchId={launch_id}");
    if !environment.iter().any(|value| value == &launch_marker) {
        return false;
    }
    let Some(command) = command else {
        return false;
    };
    let normalized = command.replace('\\', "/");
    let normalized = normalized
        .strip_prefix("Z:")
        .or_else(|| normalized.strip_prefix("z:"))
        .unwrap_or(&normalized);
    let executable = Path::new(normalized);
    if !executable
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
    {
        return false;
    }
    let basename = executable
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    if [
        "services.exe",
        "explorer.exe",
        "winedevice.exe",
        "rpcss.exe",
        "plugplay.exe",
        "conhost.exe",
        "winedbg.exe",
    ]
    .iter()
    .any(|helper| basename.eq_ignore_ascii_case(helper))
    {
        return false;
    }
    // Require both the unique launch marker and a component-aware installation identity.
    // In particular, never target umu, the bootstrap, or a shared Wine server.
    executable.starts_with(game_dir)
        || (!normalized.contains('/') && working_dir.starts_with(game_dir))
}

async fn request_opaque_ooa_token(access_token: &str) -> Result<String, AuthError> {
    let mut context = AuthContext::new()?;
    context.set_access_token(&access_token);
    context.set_token_format("OPAQUE");
    context.set_expires_in(550);

    // These scopes match the token EA Desktop requests for this
    context.add_scope("basic.commerce.cartv2");
    context.add_scope("service.atom");
    context.add_scope("dp.client.default");
    context.add_scope("signin");
    context.add_scope("social_recommendation_user");
    context.add_scope("basic.optin.write");
    context.add_scope("basic.commerce.cartv2.write");
    context.add_scope("basic.billing");
    context.add_scope("external.social_information_ups_admin");

    nucleus_auth_exchange(&context, JUNO_PC_CLIENT_ID, "token").await
}

#[cfg(unix)]
pub async fn aeapp_linux_setup() -> Result<(), NativeError> {
    aeapp_linux_setup_with_options(&ProtonLaunchOptions::default()).await
}

#[cfg(unix)]
async fn aeapp_linux_setup_with_options(options: &ProtonLaunchOptions) -> Result<(), NativeError> {
    use crate::unix::wine::{
        check_runtime_validity, check_wine_validity, get_lutris_runtimes, install_runtime,
        install_wine, setup_wine_registry_with_options,
    };

    info!("Verifying wine dependencies...");

    let skip = std::env::var("AEAPP_DISABLE_WINE_VERIFICATION").is_ok();
    if !skip {
        if options.proton_path.is_none() && !check_wine_validity().await? {
            install_wine().await?;
        }
        let runtimes = get_lutris_runtimes().await?;
        if !check_runtime_validity("eac_runtime", &runtimes).await? {
            install_runtime("eac_runtime", &runtimes).await?;
        }
        if !check_runtime_validity("umu", &runtimes).await? {
            install_runtime("umu", &runtimes).await?;
        }
    }

    setup_wine_registry_with_options(options).await?;

    Ok(())
}

pub fn parse_arguments(input: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current_arg = String::new();
    let mut in_quotes = false;

    for c in input.chars() {
        match c {
            ' ' if !in_quotes => {
                if !current_arg.is_empty() {
                    args.push(current_arg.clone());
                    current_arg.clear();
                }
            }
            '"' => {
                in_quotes = !in_quotes;
            }
            _ => {
                current_arg.push(c);
            }
        }
    }

    if !current_arg.is_empty() {
        args.push(current_arg);
    }

    args
}

#[cfg(test)]
mod tests {
    use super::{
        ensure_launch_available, BootstrapLaunchArgs, LaunchError, LaunchMode, ProtonLaunchOptions,
    };

    #[test]
    fn second_launch_is_rejected_for_any_active_mode() {
        assert!(ensure_launch_available(None).is_ok());
        for mode in [
            LaunchMode::Online("owned-offer".into()),
            LaunchMode::OnlineOffline("content-id".into(), "persona".into(), "password".into()),
        ] {
            let error = ensure_launch_available(Some(&mode)).unwrap_err();
            assert!(matches!(error, LaunchError::AlreadyRunning(_)));
            assert!(!error.to_string().contains("password"));
        }
    }

    #[cfg(target_os = "linux")]
    fn matches_process(command: Option<&str>, cwd: &str, marker: &str) -> bool {
        super::matches_launched_game_process(
            "launch-owned",
            std::path::Path::new("/games/EA Game"),
            command,
            std::path::Path::new(cwd),
            &[marker.to_owned()],
        )
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn stop_matches_wine_paths_and_child_executables_with_spaces() {
        assert!(matches_process(
            Some("Z:\\games\\EA Game\\Game.EXE"),
            "/",
            "MXLaunchId=launch-owned"
        ));
        assert!(matches_process(
            Some("child.exe"),
            "/games/EA Game/bin",
            "MXLaunchId=launch-owned"
        ));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn stop_requires_exact_launch_marker_and_directory_components() {
        assert!(!matches_process(
            Some("/games/EA Game/game.exe"),
            "/",
            "MXLaunchId=launch-other"
        ));
        assert!(!matches_process(
            Some("/games/EA Game/game.exe"),
            "/",
            "MXLaunchId=launch-owned-extra"
        ));
        assert!(!matches_process(
            Some("/games/EA Game-other/game.exe"),
            "/",
            "MXLaunchId=launch-owned"
        ));
        assert!(!matches_process(
            Some("game.exe"),
            "/games/EA Game-other",
            "MXLaunchId=launch-owned"
        ));
        assert!(!matches_process(
            Some("/other/game.exe"),
            "/games/EA Game",
            "MXLaunchId=launch-owned"
        ));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn stop_never_targets_bootstrap_runners_or_shared_wine_helpers() {
        for command in [
            "aeapp-bootstrap",
            "umu-run",
            "wine64-preloader",
            "wineserver",
            "services.exe",
            "explorer.exe",
            "winedevice.exe",
            "conhost.exe",
        ] {
            assert!(
                !matches_process(Some(command), "/games/EA Game", "MXLaunchId=launch-owned"),
                "{command}"
            );
        }
        assert!(!matches_process(
            None,
            "/games/EA Game",
            "MXLaunchId=launch-owned"
        ));
    }

    #[test]
    fn bootstrap_accepts_legacy_launch_arguments() {
        let args: BootstrapLaunchArgs =
            serde_json::from_str(r#"{"path":"game.exe","args":["-windowed"]}"#).unwrap();

        assert_eq!(args.path, "game.exe");
        assert_eq!(args.proton, ProtonLaunchOptions::default());
    }

    #[test]
    fn bootstrap_preserves_per_game_proton_options() {
        let options = ProtonLaunchOptions {
            proton_path: Some("/games/GE Proton".into()),
            proton_log: true,
            disable_esync: true,
            disable_fsync: false,
        };
        let args = BootstrapLaunchArgs {
            path: "game.exe".into(),
            args: vec!["an argument with spaces".into()],
            proton: options.clone(),
        };
        let decoded: BootstrapLaunchArgs =
            serde_json::from_str(&serde_json::to_string(&args).unwrap()).unwrap();

        assert_eq!(decoded.proton, options);
        assert_eq!(decoded.args, args.args);
    }
}
