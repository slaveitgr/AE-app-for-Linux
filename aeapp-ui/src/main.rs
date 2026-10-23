use aeapp::{core::library::OwnedOffer, util::log::init_logger};
use clap::{arg, command, Parser};
use desktop::check_desktop_icon;
use egui::{
    pos2,
    style::{ScrollStyle, Spacing},
    style::{WidgetVisuals, Widgets},
    vec2, Align2, Color32, FontData, FontDefinitions, FontFamily, FontId, Layout, Margin, Rect,
    Response, Rounding, Stroke, Style, TextureId, Ui, Vec2, ViewportBuilder, Visuals, Widget,
};
use log::error;
use std::{collections::HashMap, default::Default, ops::RangeInclusive, path::PathBuf};
use strum_macros::EnumIter;
use ui_image::{UIImageCache, UIImageType};
use views::{
    debug_view::debug_view,
    downloads_view::{downloads_view, QueuedDownload},
    friends_view::{
        friends_view, FriendsViewBar, FriendsViewBarPage, FriendsViewBarStatusFilter, UIFriend,
    },
    game_view::{games_view, GameViewBar, GameViewBarGenre, GameViewBarPlatform},
    settings_view::settings_view,
    undefined_view::{coming_soon_view, undefined_view},
};

use eframe::egui_glow;
use egui_extras::{Size, StripBuilder};
use egui_glow::glow;

use app_bg_renderer::AppBgRenderer;
use bridge_thread::{BackendError, BridgeThread};
use game_view_bg_renderer::GameViewBgRenderer;
use renderers::{app_bg_renderer, game_view_bg_renderer};
use translation_manager::{positional_replace, TranslationManager};

pub mod bridge;
pub mod util;
mod views;
pub mod widgets;

mod bridge_processor;
mod bridge_thread;
mod desktop;
mod download_options;
mod enum_locale_map;
mod event_processor;
mod event_thread;
mod membership;
mod preview;
mod renderers;
mod shell;
mod translation_manager;
mod tray;
mod ui_image;

const ACCENT_COLOR: Color32 = Color32::from_rgb(39, 106, 252);
const APP_MARGIN: Vec2 = vec2(12.0, 12.0); //TODO: user setting
const FRIEND_INGAME_COLOR: Color32 = Color32::from_rgb(39, 106, 252); // temp

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    debug: bool,
    #[arg(short, long)]
    profile: bool,
    /// Explore the interface with local demo data and no EA connection.
    #[arg(long, alias = "no-login")]
    preview: bool,
    /// Save the rendered interface as a PNG and exit.
    #[arg(long, value_name = "PNG_PATH")]
    screenshot: Option<PathBuf>,
    /// Initial local preview screen.
    #[arg(long, default_value = "library", value_parser = ["library", "home", "settings", "game", "proton", "downloads", "profile", "browse", "ea-play", "install"])]
    preview_page: String,
    /// Local screenshot viewport, e.g. 1920x1032 (preview only).
    #[arg(long)]
    preview_size: Option<String>,
    /// Initial settings tab in local preview (0 through 5).
    #[arg(long, default_value_t = 0, value_parser = clap::value_parser!(u8).range(0..=5))]
    preview_settings_tab: u8,
    /// Initial Game Hub tab in local preview (Overview/Currency/Add-ons).
    #[arg(long, default_value_t = 0, value_parser = clap::value_parser!(u8).range(0..=2))]
    preview_game_tab: u8,
    /// Download options preview state (local fixtures only).
    #[arg(long, default_value = "ready", value_parser = ["ready", "calculating", "insufficient", "error", "enqueuing", "enqueue-error"])]
    preview_install_state: String,
    /// Game Hub details preview state (local fixtures only).
    #[arg(long, default_value = "ready", value_parser = ["ready", "loading", "failed"])]
    preview_details_state: String,
    /// Active download preview state (local fixtures only).
    #[arg(long, default_value = "downloading", value_parser = ["downloading", "unknown-size", "paused", "failed", "finalizing"])]
    preview_download_state: String,
    /// Show a local sample transfer in the sidebar on any preview page.
    #[arg(long)]
    preview_active_download: bool,
    #[arg(short, long)]
    allow_bf3: bool,
}

fn local_asset_directory() -> Option<PathBuf> {
    std::env::var_os("AEAPP_ASSET_DIR")
        .or_else(|| std::env::var_os("AEAPP_FONT_DIR"))
        .map(PathBuf::from)
        .or_else(|| {
            std::env::current_exe()
                .ok()
                .and_then(|exe| exe.parent().map(|path| path.join("ea-assets")))
        })
}

fn application_icon() -> egui::IconData {
    if let Some(directory) = local_asset_directory() {
        if let Ok(bytes) = std::fs::read(directory.join("icon.png")) {
            match eframe::icon_data::from_png_bytes(&bytes) {
                Ok(icon) => return icon,
                Err(error) => log::warn!("Could not load local app icon: {error}"),
            }
        }
    }
    eframe::icon_data::from_png_bytes(include_bytes!("../../aeapp-resources/assets/logo.png"))
        .expect("The bundled application icon is valid")
}

#[tokio::main]
async fn main() {
    init_logger();
    let mut args = Args::parse();

    if !args.preview
        && !std::env::var("AEAPP_INSTALLED").is_ok_and(|var| var == "1")
        && !std::env::var("AEAPP_PACKAGED").is_ok_and(|var| var == "1")
    {
        if let Err(err) = check_desktop_icon() {
            error!("Failed to register desktop icon! {}", err);
        }
    }

    if !cfg!(debug_assertions) {
        args.debug = false;
    }

    if args.profile && !args.preview {
        puffin::set_scopes_on(true);

        match puffin_http::Server::new("127.0.0.1:8585") {
            Ok(puffin_server) => {
                std::process::Command::new("~/.cargo/bin/puffin_viewer")
                    .arg("--url")
                    .arg("127.0.0.1:8585")
                    .spawn()
                    .ok();

                // We can store the server if we want, but in this case we just want
                // it to keep running. Dropping it closes the server, so let's not drop it!
                #[allow(clippy::mem_forget)]
                std::mem::forget(puffin_server);
            }
            Err(err) => {
                eprintln!("Failed to start puffin server: {err}");
            }
        };
    }

    let preview_size = args
        .preview_size
        .as_deref()
        .filter(|_| args.preview)
        .and_then(|value| value.split_once('x'))
        .and_then(|(width, height)| Some([width.parse::<f32>().ok()?, height.parse::<f32>().ok()?]))
        .filter(|size| {
            size[0] >= 1080.0 && size[0] <= 3840.0 && size[1] >= 700.0 && size[1] <= 2160.0
        })
        .unwrap_or([1440.0, 900.0]);
    let mut native_options = eframe::NativeOptions {
        persist_window: !args.preview,
        persistence_path: if args.preview {
            Some(PathBuf::from("/dev/null"))
        } else {
            None
        },
        viewport: ViewportBuilder::default()
            .with_inner_size(preview_size)
            .with_min_inner_size([1080.0, 700.0])
            .with_app_id("io.github.slaveitgr.aeapp")
            .with_decorations(false)
            .with_icon(application_icon()),
        ..Default::default()
    };
    #[cfg(target_os = "linux")]
    if !args.preview && std::env::var_os("DISPLAY").is_some() {
        // Winit 0.29 cannot hide a native Wayland window. XWayland preserves tray-close behavior.
        use winit::platform::x11::EventLoopBuilderExtX11;
        native_options.event_loop_builder = Some(Box::new(|builder| {
            builder.with_x11();
        }));
    }
    eframe::run_native(
        "aeapp",
        native_options,
        Box::new(move |cc| Ok(Box::new(AeAppEguiApp::new(cc, args)))),
    )
    .expect("Failed i guess?")
}

#[derive(Debug, PartialEq, Default, Clone)]
enum PageType {
    #[default]
    Games,
    Home,
    Store,
    EaPlay,
    Settings,
    Downloads,
    Debug,
}

#[derive(Debug, PartialEq, Clone)]
// ATM Machine
enum PopupModal {
    GameSettings(String),
    GameInstall(String),
    GameLaunchOOD(String),
}

/// Which tab is selected in the game list info panel
#[derive(PartialEq, Default)]
pub enum GameInfoTab {
    #[default]
    Achievements,
    Dlc,
    Mods,
}

/// TBD
pub struct GameInstalledModsInfo {}

#[derive(PartialEq, Clone)]
pub struct GameDetails {
    /// Playtime in tenths of an hour. None means no verified statistic was supplied.
    time: Option<u32>,
    /// None is unavailable; Some(0) is a verified zero.
    achievements_unlocked: Option<u16>,
    /// None is unavailable; Some(0) means the game has no achievements.
    achievements_total: Option<u16>,
    /// Path the game is installed to
    path: String,
    /// Minimum specs to run the game, in EasyMark spec
    system_requirements_min: Option<String>,
    /// Recommended specs to run the game, in EasyMark spec
    system_requirements_rec: Option<String>,
}

#[derive(Clone)]
pub enum GameDetailsWrapper {
    Unloaded,
    Loading,
    Available(GameDetails),
    Failed(String),
}

impl From<Result<GameDetails, String>> for GameDetailsWrapper {
    fn from(result: Result<GameDetails, String>) -> Self {
        match result {
            Ok(details) => Self::Available(details),
            Err(message) => Self::Failed(message),
        }
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct GameSettings {
    cloud_saves: bool,
    launch_args: String,
    exe_override: String,
    #[serde(default)]
    proton_path: String,
    #[serde(default)]
    proton_log: bool,
    #[serde(default)]
    disable_esync: bool,
    #[serde(default)]
    disable_fsync: bool,
}

impl GameSettings {
    pub fn new() -> Self {
        Self {
            cloud_saves: true,
            launch_args: String::new(),
            exe_override: String::new(),
            proton_path: String::new(),
            proton_log: false,
            disable_esync: false,
            disable_fsync: false,
        }
    }
}

#[derive(Clone)]
pub struct GameVersionInfo {
    installed: String,
    latest: String,
    mandatory: bool,
}

#[derive(Clone)]
pub struct GameInfo {
    /// Origin slug of the game
    slug: String,
    /// Origin offer ID of the game
    offer: String,
    /// Display name of the game
    name: String,
    /// Game info
    details: GameDetailsWrapper,
    version: GameVersionInfo,
    dlc: Vec<OwnedOffer>,
    installed: bool,
    has_cloud_saves: bool,
}

#[derive(PartialEq, Eq)]
pub enum BackendStallState {
    Starting,
    UserNeedsToInstallService,
    UserNeedsToLogIn,
    LoggingIn,
    BingChilling,
}

pub use download_options::InstallModalState;

pub struct AeAppEguiApp {
    /// extra not-already-handled commandline args
    args: Args,
    /// general toggle for showing debug info
    debug: bool,
    /// stuff for the bar on the top of the Games view
    game_view_bar: GameViewBar,
    /// stuff for the bar on the top of the Friends view
    friends_view_bar: FriendsViewBar,
    /// Logged in user's display name
    user_name: String,
    /// Logged in user's ID
    user_id: String,
    membership: membership::Membership,
    /// games
    games: HashMap<String, GameInfo>,
    /// selected game
    game_sel: String,
    /// friends
    friends: Vec<UIFriend>,
    /// width of the friends sidebar
    friends_width: f32,
    /// force visibility of friends sidebar
    force_friends: bool,
    /// what page you're on (games, friends, etc)
    page_view: PageType,
    /// Modal
    modal: Option<PopupModal>,
    /// Renderer for the blur effect in the game view
    game_view_bg_renderer: Option<GameViewBgRenderer>,
    /// Renderer for the app's background
    app_bg_renderer: Option<AppBgRenderer>,
    /// Image cache
    img_cache: UIImageCache,
    /// Translations
    locale: TranslationManager,
    /// If a core thread has crashed and made the UI unstable
    critical_error: Option<BackendError>,
    /// If a backend function has failed that the user should be aware of, but UI is still functional
    nonfatal_errors: Vec<BackendError>,
    /// Backend
    backend: BridgeThread,
    /// what the backend doin?
    backend_state: BackendStallState,
    /// what type of login we're using
    /// Slug of the game currently running, may not be fully accurate but it's good enough to let the user know the button was clicked
    playing_game: Option<String>,
    /// Currently downloading game
    installing_now: Option<QueuedDownload>,
    /// Queue of game installs, indexed by offer ID
    install_queue: HashMap<String, QueuedDownload>,
    /// State for installer modal
    installer_state: InstallModalState,
    /// User Settings for the frontend
    settings: FrontendSettings,
    library_details: bool,
    installed_only: bool,
    rendered_frames: u32,
    screenshot_requested: bool,
    screenshot_done: bool,
    native_tray: Option<tray::NativeTray>,
    tray_hidden: bool,
    force_quit: bool,
}

#[derive(serde::Serialize, serde::Deserialize, PartialEq, EnumIter)]
pub enum FrontendLanguage {
    SystemDefault,
    EnUS,
}

#[derive(serde::Serialize, serde::Deserialize, Copy, Clone)]
pub struct FrontendPerformanceSettings {
    disable_blur: bool,
}

impl FrontendPerformanceSettings {
    pub fn new() -> Self {
        Self {
            disable_blur: false,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct FrontendSettings {
    default_install_folder: String,
    language: FrontendLanguage,
    ignore_ood_games: bool,
    game_settings: HashMap<String, GameSettings>,
    performance_settings: FrontendPerformanceSettings,
}

impl FrontendSettings {
    pub fn new() -> Self {
        Self {
            default_install_folder: String::new(),
            language: FrontendLanguage::SystemDefault,
            ignore_ood_games: false,
            game_settings: HashMap::new(),
            performance_settings: FrontendPerformanceSettings::new(),
        }
    }
}

const F9B233: Color32 = ACCENT_COLOR;

const WIDGET_HOVER: Color32 = Color32::from_rgb(61, 121, 252);

impl AeAppEguiApp {
    pub(crate) fn is_preview(&self) -> bool {
        self.args.preview
    }

    fn new(cc: &eframe::CreationContext<'_>, args: Args) -> Self {
        let mut style = Style::default();
        if args.preview {
            style.animation_time = 0.0;
        }
        style.spacing.item_spacing = vec2(10.0, 10.0);
        style.spacing.interact_size = vec2(40.0, 36.0);
        style.spacing.button_padding = vec2(16.0, 9.0);
        style.spacing.scroll.bar_width = 6.0;
        style.visuals = Visuals::dark();
        style.visuals.override_text_color = Some(shell::TEXT);
        style.visuals.panel_fill = shell::BACKGROUND;
        style.visuals.window_fill = shell::SURFACE;
        style.visuals.faint_bg_color = shell::SURFACE;
        style.visuals.extreme_bg_color = shell::SIDEBAR;
        style.visuals.hyperlink_color = ACCENT_COLOR;
        style.visuals.selection.bg_fill = ACCENT_COLOR.linear_multiply(0.45);
        style.visuals.widgets.inactive.bg_fill = shell::SURFACE;
        style.visuals.widgets.inactive.weak_bg_fill = shell::SURFACE;
        style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, shell::BORDER);
        style.visuals.widgets.inactive.rounding = Rounding::same(7.0);
        style.visuals.widgets.hovered.bg_fill = shell::SURFACE_HOVER;
        style.visuals.widgets.hovered.weak_bg_fill = shell::SURFACE_HOVER;
        style.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, shell::MUTED);
        style.visuals.widgets.hovered.rounding = Rounding::same(7.0);
        style.visuals.widgets.active.bg_fill = ACCENT_COLOR;
        style.visuals.widgets.active.weak_bg_fill = ACCENT_COLOR;
        style.visuals.widgets.active.rounding = Rounding::same(7.0);
        style.visuals.window_rounding = Rounding::same(12.0);
        style.visuals.window_stroke = Stroke::new(1.0, shell::BORDER);
        style.text_styles.insert(egui::TextStyle::Body, FontId::proportional(14.0));
        style.text_styles.insert(egui::TextStyle::Button, FontId::proportional(14.0));
        style.text_styles.insert(egui::TextStyle::Heading, FontId::proportional(26.0));

        let mut fonts = FontDefinitions::default();

        fonts.font_data.insert(
            "ibm_plex".to_owned(),
            FontData::from_static(include_bytes!("../fonts/IBMPlexSans-Regular.ttf")),
        );

        fonts
            .families
            .get_mut(&FontFamily::Proportional)
            .unwrap()
            .insert(0, "ibm_plex".to_owned());

        fonts.families.get_mut(&FontFamily::Monospace).unwrap().push("ibm_plex".to_owned());

        // Optional user-owned EA fonts are loaded locally, never embedded in this binary.
        let font_directory = local_asset_directory();
        if let Some(directory) = font_directory {
            if let Ok(bytes) = std::fs::read(directory.join("ea-text.otf")) {
                fonts.font_data.insert("ea_text_local".to_owned(), FontData::from_owned(bytes));
                fonts
                    .families
                    .get_mut(&FontFamily::Proportional)
                    .unwrap()
                    .insert(0, "ea_text_local".to_owned());
            }
            if let Ok(bytes) = std::fs::read(directory.join("ea-display.otf")) {
                fonts.font_data.insert("ea_display_local".to_owned(), FontData::from_owned(bytes));
                let mut family = fonts.families[&FontFamily::Proportional].clone();
                family.insert(0, "ea_display_local".to_owned());
                fonts.families.insert(FontFamily::Name("ea_display".into()), family);
                style.text_styles.insert(
                    egui::TextStyle::Heading,
                    FontId::new(28.0, FontFamily::Name("ea_display".into())),
                );
            }
        }
        for (family, filename) in [
            ("ea_bold", "ea-text-bold.otf"),
            ("ea_display_bold", "ea-display-bold.otf"),
        ] {
            let mut fallback = fonts.families[&FontFamily::Proportional].clone();
            if let Some(bytes) = local_asset_directory()
                .and_then(|directory| std::fs::read(directory.join(filename)).ok())
            {
                fonts.font_data.insert(family.to_owned(), FontData::from_owned(bytes));
                fallback.insert(0, family.to_owned());
            }
            fonts.families.insert(FontFamily::Name(family.into()), fallback);
        }
        style.text_styles.insert(egui::TextStyle::Heading, shell::display(32.0));
        egui_extras::install_image_loaders(&cc.egui_ctx);
        cc.egui_ctx.set_style(style);
        cc.egui_ctx.set_fonts(fonts);

        #[cfg(debug_assertions)]
        cc.egui_ctx.set_debug_on_hover(args.debug);

        let mut settings: FrontendSettings = if args.preview {
            FrontendSettings::new()
        } else if let Some(storage) = cc.storage {
            eframe::get_value(storage, "settings").unwrap_or(FrontendSettings::new())
        } else {
            FrontendSettings::new()
        };

        let (img_cache, backend) = if args.preview {
            (
                UIImageCache::preview(cc.egui_ctx.clone()),
                preview::disconnected_backend(),
            )
        } else {
            let (cache, remote) = UIImageCache::new(cc.egui_ctx.clone());
            (cache, BridgeThread::new(&cc.egui_ctx, remote))
        };
        let is_preview = args.preview;
        if is_preview {
            cc.egui_ctx.data_mut(|data| {
                data.insert_temp(
                    egui::Id::new("ea_settings_tab"),
                    args.preview_settings_tab as usize,
                )
            });
        }
        let debug = args.debug;
        let preview_page = args.preview_page.clone();
        if is_preview {
            cc.egui_ctx.data_mut(|data| {
                data.insert_temp(
                    egui::Id::new(("aeapp-game-hub-tab", "star-wars-jedi-survivor")),
                    args.preview_game_tab as usize,
                )
            });
        }
        if is_preview && preview_page == "profile" {
            cc.egui_ctx
                .data_mut(|data| data.insert_temp(egui::Id::new("ea-own-profile-summary"), true));
        }
        if is_preview && preview_page == "proton" {
            settings
                .game_settings
                .insert("star-wars-jedi-survivor".to_owned(), GameSettings::new());
        }
        let preview_downloads =
            is_preview && (preview_page == "downloads" || args.preview_active_download);
        let preview_current = preview_downloads
            .then(|| preview::seed_download(&cc.egui_ctx, &args.preview_download_state));
        let mut preview_queue = HashMap::new();
        if preview_downloads {
            let queued = QueuedDownload {
                slug: "mass-effect-legendary".to_owned(),
                offer: "preview:mass-effect-legendary".to_owned(),
                downloaded_bytes: 0,
                total_bytes: 0,
            };
            cc.egui_ctx.data_mut(|data| {
                data.insert_temp(
                    views::downloads_view::order_key(),
                    vec![queued.offer.clone()],
                );
            });
            preview_queue.insert(queued.offer.clone(), queued);
        }
        let native_tray = tray::NativeTray::start(&cc.egui_ctx, is_preview);
        #[cfg(target_os = "linux")]
        if let Some(tray) = native_tray.as_ref() {
            use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
            if let Ok(handle) = cc.window_handle() {
                let id = match handle.as_raw() {
                    RawWindowHandle::Xlib(window) => u32::try_from(window.window).ok(),
                    RawWindowHandle::Xcb(window) => Some(window.window.get()),
                    _ => None,
                };
                if let Some(id) = id {
                    tray.set_x11_window(id);
                }
            }
        }

        let mut installer_state = InstallModalState::new(&settings);
        if is_preview && preview_page == "install" {
            installer_state = InstallModalState::for_game(&settings, "star-wars-jedi-survivor");
            installer_state.seed_preview(&args.preview_install_state);
        }
        let games = if is_preview {
            preview::games(&args.preview_details_state)
        } else {
            HashMap::new()
        };
        Self {
            args,
            debug,
            game_view_bar: GameViewBar {
                genre_filter: GameViewBarGenre::AllGames,
                platform_filter: GameViewBarPlatform::AllPlatforms,
                game_size: 2.0,
                search_buffer: String::new(),
            },
            friends_view_bar: FriendsViewBar {
                page: FriendsViewBarPage::All,
                status_filter: FriendsViewBarStatusFilter::Name,
                search_buffer: String::new(),
                friend_sel: String::new(),
            },
            user_name: if is_preview { "Demo player" } else { "Player" }.to_owned(),
            user_id: String::new(),
            membership: if is_preview {
                membership::Membership::EaPlay
            } else {
                membership::Membership::Unknown
            },
            games,
            game_sel: if is_preview
                && matches!(preview_page.as_str(), "game" | "proton" | "install")
            {
                "star-wars-jedi-survivor".to_owned()
            } else {
                String::new()
            },
            friends: Vec::new(),
            friends_width: 0.0,
            force_friends: false,
            //game_view_rows: false,
            page_view: if is_preview && preview_page == "settings" {
                PageType::Settings
            } else if is_preview && matches!(preview_page.as_str(), "home" | "profile") {
                PageType::Home
            } else if is_preview && preview_page == "browse" {
                PageType::Store
            } else if is_preview && preview_page == "ea-play" {
                PageType::EaPlay
            } else if is_preview && preview_page == "downloads" {
                PageType::Downloads
            } else if !is_preview {
                PageType::Home
            } else {
                PageType::Games
            },
            modal: if is_preview && preview_page == "proton" {
                Some(PopupModal::GameSettings(
                    "star-wars-jedi-survivor".to_owned(),
                ))
            } else if is_preview && preview_page == "install" {
                Some(PopupModal::GameInstall(
                    "star-wars-jedi-survivor".to_owned(),
                ))
            } else {
                None
            },
            game_view_bg_renderer: None,
            app_bg_renderer: None,
            img_cache,
            locale: TranslationManager::new(&settings.language),
            critical_error: None,
            nonfatal_errors: Vec::new(),
            backend,
            backend_state: if is_preview {
                BackendStallState::BingChilling
            } else {
                BackendStallState::Starting
            },
            playing_game: None,
            installing_now: preview_current,
            install_queue: preview_queue,
            installer_state,
            settings,
            library_details: is_preview
                && matches!(preview_page.as_str(), "game" | "proton" | "install"),
            installed_only: false,
            rendered_frames: 0,
            screenshot_requested: false,
            screenshot_done: false,
            native_tray,
            tray_hidden: false,
            force_quit: false,
        }
    }
}

// modified from https://github.com/emilk/egui/blob/master/examples/custom_window_frame/src/main.rs

pub fn tab_bar_button(ui: &mut Ui, res: Response) {
    puffin::profile_function!();
    let mut res2 = Rect::clone(&res.rect);
    res2.min.y = res2.max.y - 4.;
    ui.painter().vline(
        res2.min.x + 2.0,
        RangeInclusive::new(res2.min.y, res2.max.y),
        Stroke::new(2.0, ACCENT_COLOR),
    );
    ui.painter().rect_filled(
        res2,
        Rounding::ZERO,
        if res.hovered() {
            ACCENT_COLOR
        } else {
            ACCENT_COLOR.linear_multiply(0.9)
        },
    );
}

/// We used to have a semi-functional implementation that only worked on Mac, but, as i would say, we do not care 🗣️🗣️🗣️
fn custom_window_frame(
    enabled: bool, // disables the entire app, used for if the bg thread crashes
    crash_text: String,
    ctx: &egui::Context,
    _: &mut eframe::Frame,
    _title: &str,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    puffin::profile_function!();
    use egui::*;

    let panel_frame = egui::Frame {
        fill: Color32::RED,
        rounding: 0.0.into(),
        stroke: Stroke::NONE,
        outer_margin: Margin {
            left: APP_MARGIN.x,
            right: APP_MARGIN.x,
            top: APP_MARGIN.y + if !enabled { APP_MARGIN.y * 3.0 } else { 0.0 },
            bottom: APP_MARGIN.y,
        },
        ..Default::default()
    };

    CentralPanel::default().frame(panel_frame).show(ctx, |ui| {
        if !enabled {
            let warning_rect = Rect {
                min: Pos2::ZERO,
                max: pos2(
                    ui.available_width() + APP_MARGIN.x * 2.0,
                    APP_MARGIN.y * 3.0,
                ),
            };
            ui.painter().rect_filled(warning_rect, Rounding::same(0.0), Color32::RED);
            ui.painter().text(
                warning_rect.center(),
                Align2::CENTER_CENTER,
                crash_text,
                FontId::proportional(16.0),
                Color32::BLACK,
            );
        }
        ui.add_enabled_ui(enabled, add_contents);
    });
}

/// Wrapper/helper for the tab buttons in the top left of the app
fn tab_button(ui: &mut Ui, edit_var: &mut PageType, page: PageType, label: &str) {
    puffin::profile_function!();
    ui.style_mut().visuals.widgets.inactive.rounding = Rounding::ZERO;
    ui.style_mut().visuals.widgets.active.rounding = Rounding::ZERO;
    ui.style_mut().visuals.widgets.hovered.rounding = Rounding::ZERO;
    ui.style_mut().visuals.widgets.inactive.expansion = -1.0;
    ui.style_mut().visuals.widgets.active.expansion = -1.0;
    ui.style_mut().visuals.widgets.hovered.expansion = -1.0;

    if edit_var == &page {
        ui.style_mut().visuals.widgets.inactive.weak_bg_fill = Color32::WHITE;
        ui.style_mut().visuals.widgets.inactive.fg_stroke = Stroke::new(2.0, Color32::BLACK);
        ui.style_mut().visuals.widgets.inactive.bg_stroke = Stroke::NONE;
        ui.style_mut().visuals.widgets.active.weak_bg_fill = Color32::WHITE;
        ui.style_mut().visuals.widgets.active.fg_stroke = Stroke::new(2.0, Color32::BLACK);
        ui.style_mut().visuals.widgets.active.bg_stroke = Stroke::NONE;
        ui.style_mut().visuals.widgets.hovered.weak_bg_fill = Color32::WHITE;
        ui.style_mut().visuals.widgets.hovered.fg_stroke = Stroke::new(2.0, Color32::BLACK);
        ui.style_mut().visuals.widgets.hovered.bg_stroke = Stroke::NONE;
    } else {
        ui.style_mut().visuals.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
        ui.style_mut().visuals.widgets.inactive.fg_stroke = Stroke::new(2.0, Color32::WHITE);
        ui.style_mut().visuals.widgets.inactive.bg_stroke = Stroke::NONE;
        ui.style_mut().visuals.widgets.active.weak_bg_fill = Color32::TRANSPARENT;
        ui.style_mut().visuals.widgets.active.fg_stroke = Stroke::new(2.0, F9B233);
        ui.style_mut().visuals.widgets.active.bg_stroke = Stroke::NONE;
        ui.style_mut().visuals.widgets.hovered.weak_bg_fill = Color32::TRANSPARENT;
        ui.style_mut().visuals.widgets.hovered.fg_stroke = Stroke::new(2.0, F9B233);
        ui.style_mut().visuals.widgets.hovered.bg_stroke = Stroke::NONE;
    }
    let text = egui::RichText::new(label.to_uppercase()).size(16.0);

    let test = ui.add_sized([120.0, 28.0], egui::Button::new(text));
    if test.clicked() {
        *edit_var = page.clone();
    }
}

// god-awful macro to do something incredibly simple because apparently wrapping it in a function has rustc fucking implode
// say what you want about C++ footguns but rust is the polar fucking opposite, shooting you in the head for doing literally anything
macro_rules! set_app_modal {
    ($arg1:expr, $arg2:expr) => {
        if let Some(modal) = $arg2 {
            match modal {
                PopupModal::GameSettings(slug) => {
                    if $arg1.settings.game_settings.get(&slug).is_none() {
                        $arg1
                            .settings
                            .game_settings
                            .insert(slug.clone(), crate::GameSettings::new());
                    }
                }
                PopupModal::GameInstall(slug) => {
                    $arg1.installer_state =
                        crate::InstallModalState::for_game(&$arg1.settings, &slug);
                }
                PopupModal::GameLaunchOOD(_) => {}
            }
            $arg1.modal = $arg2;
        } else {
            $arg1.modal = None;
        }
    };
}

pub(crate) use set_app_modal;

impl AeAppEguiApp {
    fn tab_bar(&mut self, header: &mut Ui) {
        puffin::profile_function!();
        let navbar = egui::Frame::default()
            .stroke(Stroke::new(2.0, Color32::WHITE))
            .inner_margin(Margin::same(0.0))
            .outer_margin(Margin::same(2.0))
            .rounding(Rounding::same(4.0));
        navbar.show(header, |ui| {
            ui.horizontal(|ui| {
                let loc = &self.locale.localization.menubar;
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.style_mut().visuals.widgets.inactive.rounding = Rounding::ZERO;
                tab_button(ui, &mut self.page_view, PageType::Games, &loc.games);
                tab_button(ui, &mut self.page_view, PageType::Store, &loc.store);
                tab_button(ui, &mut self.page_view, PageType::Settings, &loc.settings);
                tab_button(ui, &mut self.page_view, PageType::Downloads, &loc.downloads);
                #[cfg(debug_assertions)]
                if self.debug {
                    tab_button(ui, &mut self.page_view, PageType::Debug, "Debug");
                }
            });
        });
    }

    fn you(&mut self, profile: &mut Ui) {
        puffin::profile_function!();
        profile.with_layout(egui::Layout::right_to_left(egui::Align::Center), |rtl| {
            rtl.style_mut().spacing.item_spacing.x = 0.0;
            rtl.allocate_space(vec2(2.0, 2.0));
            rtl.style_mut().spacing.item_spacing.x = APP_MARGIN.x;

            let uid = self.user_id.clone();
            let img_response = if let Some(av) = self.img_cache.get(UIImageType::Avatar(uid)) {
                rtl.image((av.id(), vec2(36.0, 36.0)))
            } else {
                rtl.image((self.img_cache.placeholder_avatar.id(), vec2(36.0, 36.0)))
            };
            let stroke = Stroke::new(2.0, {
                if self.playing_game.is_some() {
                    FRIEND_INGAME_COLOR
                } else {
                    Color32::GREEN
                }
            });
            rtl.painter().rect(
                img_response.rect.expand(1.0),
                Rounding::same(4.0),
                Color32::TRANSPARENT,
                stroke,
            );
            let point = img_response.rect.left_center() + vec2(-rtl.spacing().item_spacing.x, 2.0);

            if let Some(game_slug) = &self.playing_game {
                if let Some(game) = &self.games.get(game_slug) {
                    let offset = vec2(0.0, 0.5);
                    rtl.painter().text(
                        point - offset,
                        Align2::RIGHT_BOTTOM,
                        &self.user_name,
                        FontId::proportional(15.0),
                        Color32::WHITE,
                    );
                    rtl.painter().text(
                        point + offset,
                        Align2::RIGHT_TOP,
                        positional_replace!(
                            &self.locale.localization.friends_view.status.presence_basic,
                            "game",
                            &game.name
                        ),
                        FontId::proportional(10.0),
                        Color32::WHITE,
                    );
                }
            } else {
                rtl.painter().text(
                    point,
                    Align2::RIGHT_CENTER,
                    &self.user_name,
                    FontId::proportional(15.0),
                    Color32::WHITE,
                );
            }
        });
    }

    fn main(&mut self, app_rect: Rect, ui: &mut Ui) {
        ui.add_enabled_ui(self.modal.is_none(), |ui| shell::show(self, ui));
        if let Some(PopupModal::GameInstall(slug)) = self.modal.clone() {
            download_options::show(self, ui.ctx(), &slug);
            return;
        }
        let mut clear = false;
        if let Some(modal) = &self.modal {
            egui::Window::new("Manage game")
                .id(egui::Id::new("game_options_modal"))
                .title_bar(false)
                .collapsible(false)
                .resizable(false)
                .fixed_size(vec2(720.0, (app_rect.height() - 100.0).clamp(400.0, 650.0)))
                .max_height((app_rect.height() - 60.0).max(300.0))
                .vscroll(true)
                .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
                .frame(egui::Frame::window(ui.style()).fill(shell::SURFACE).inner_margin(24.0))
                .show(ui.ctx(), |ui| {
                        ui.style_mut().spacing.interact_size = vec2(100.0, 30.0);
                        ui.spacing_mut().icon_width = 30.0;
                        match modal {
                            PopupModal::GameSettings(slug) => 'outer: {
                                let game = if let Some(game) = self.games.get_mut(slug) { game } else { break 'outer; };
                                //let game_settings = game.settings.borrow_mut();
                                ui.horizontal(|header| {
                                    header.heading(positional_replace!(self.locale.localization.modals.game_settings.header, "game", &game.name));
                                    header.with_layout(Layout::right_to_left(egui::Align::Center), |close_button| {
                                        if close_button.add_sized(vec2(80.0, 30.0), egui::Button::new(&self.locale.localization.modals.close.to_ascii_uppercase())).clicked() {
                                            clear = true
                                        }
                                    });
                                });
                                ui.separator();
                                if game.installed {
                                    if let Some(settings) = self.settings.game_settings.get_mut(&game.slug) {
                                        ui.add_enabled(game.has_cloud_saves, egui::Checkbox::new(&mut settings.cloud_saves, &self.locale.localization.modals.game_settings.cloud_saves));

                                        ui.label(&self.locale.localization.modals.game_settings.launch_arguments);
                                        ui.add_sized(vec2(ui.available_width(), ui.style().spacing.interact_size.y), egui::TextEdit::singleline(&mut settings.launch_args).vertical_align(egui::Align::Center));

                                        ui.separator();


                                        let button_size = vec2(100.0, 30.0);

                                        ui.label(&self.locale.localization.modals.game_settings.executable_override);
                                        ui.horizontal(|ui| {
                                            let size = vec2(500.0 - (24.0 + ui.style().spacing.item_spacing.x), 30.0);
                                            ui.add_sized(size, egui::TextEdit::singleline(&mut settings.exe_override).vertical_align(egui::Align::Center));
                                            ui.add_enabled(false, egui::Button::new("Browse · soon"));
                                        });

                                        #[cfg(target_os = "linux")]
                                        {
                                            ui.heading("Proton compatibility");
                                            ui.label("Proton directory override");
                                            ui.add_sized(vec2(ui.available_width(), 34.0), egui::TextEdit::singleline(&mut settings.proton_path).hint_text("Leave empty to use managed GE-Proton"));
                                            ui.small("Choose a local Proton directory containing the proton script.");
                                            ui.checkbox(&mut settings.proton_log, "Write Proton diagnostic log");
                                            ui.horizontal(|ui| {
                                                ui.checkbox(&mut settings.disable_esync, "Disable Esync");
                                                ui.checkbox(&mut settings.disable_fsync, "Disable Fsync");
                                            });
                                        }
                                        ui.separator();
                                    }
                                    ui.add_space(10.0);

                                    ui.horizontal(|ui| {
                                        ui.label(positional_replace!(self.locale.localization.modals.game_settings.version, "version", &game.version.installed));
                                        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                                            ui.add_enabled(false, egui::Button::new(format!("  {}  ", &self.locale.localization.modals.game_settings.uninstall.to_ascii_uppercase())));
                                        });
                                    });
                                } else {
                                    ui.label(&self.locale.localization.modals.game_settings.not_installed);
                                }
                            },
                            PopupModal::GameInstall(_) => {}
                            PopupModal::GameLaunchOOD(slug) => 'outer: {
                                let game = if let Some(game) = self.games.get_mut(slug) { game } else { break 'outer; };
                                ui.horizontal(|header| {
                                    header.heading(&self.locale.localization.modals.game_launch_out_of_date.header);
                                    header.with_layout(Layout::right_to_left(egui::Align::Center), |close_button| {
                                        close_button.add_enabled_ui(!self.installer_state.locating, |close_button| {
                                            if close_button.add_sized(vec2(80.0, 30.0), egui::Button::new(&self.locale.localization.modals.close.to_ascii_uppercase())).clicked() {
                                                clear = true
                                            }
                                        });
                                    });
                                });

                                ui.separator();

                                ui.label(positional_replace!(&self.locale.localization.modals.game_launch_out_of_date.warning, "gamename", &game.name));
                                if game.version.mandatory {
                                    egui::Label::new(egui::RichText::new(positional_replace!(&self.locale.localization.modals.game_launch_out_of_date.really_warning, "gamename", &game.name)).color(Color32::RED)).ui(ui);
                                }

                                ui.label(positional_replace!(&self.locale.localization.modals.game_launch_out_of_date.comparison, "local", &game.version.installed, "online", &game.version.latest));

                                ui.with_layout(Layout::bottom_up(egui::Align::Min), |ui| {
                                    if ui.add_sized([ui.available_size_before_wrap().x, ui.spacing().interact_size.y], egui::Button::new(&self.locale.localization.modals.game_launch_out_of_date.launch)).clicked() {
                                        self.playing_game = Some(game.slug.clone());
                                        let settings = self.settings.game_settings.get(&game.slug);
                                        let settings = if let Some(settings) = settings {
                                            Some(settings.to_owned())
                                        } else {
                                            None
                                        };
                                        let _ = self.backend.backend_commander.send(
                                            crate::bridge_thread::AeAppLibRequest::StartGameRequest(
                                                game.clone(),
                                                settings,
                                            ),
                                        );
                                        clear = true
                                    }
                                    ui.separator();
                                    ui.checkbox(&mut self.settings.ignore_ood_games, &self.locale.localization.modals.game_launch_out_of_date.ok_i_get_it);
                                });
                            }
                        }
                });
        }
        if clear || ui.ctx().input(|input| input.key_pressed(egui::Key::Escape)) {
            self.modal = None;
        }
    }

    fn login(&mut self, app_rect: Rect, ui: &mut Ui) {
        let main_block_rect = ui.painter().text(
            app_rect.center(),
            Align2::CENTER_BOTTOM,
            &self.locale.localization.startup_flow.login_header,
            egui::FontId::proportional(30.0),
            Color32::WHITE,
        );
        let button_rect = Rect {
            min: main_block_rect.center_bottom() + vec2(-60.0, 4.0),
            max: main_block_rect.center_bottom() + vec2(60.0, 34.0),
        };
        if ui
            .put(
                button_rect,
                egui::Button::new(
                    &self.locale.localization.startup_flow.login_button.to_ascii_uppercase(),
                ),
            )
            .clicked()
        {
            self.backend
                .backend_commander
                .send(bridge_thread::AeAppLibRequest::LoginRequestOauth)
                .unwrap();
            self.backend_state = BackendStallState::LoggingIn;
        }
    }
}

impl eframe::App for AeAppEguiApp {
    fn raw_input_hook(&mut self, _ctx: &egui::Context, input: &mut egui::RawInput) {
        if self.args.preview && self.args.screenshot.is_some() {
            // Frozen fixture screenshots must not depend on the desktop cursor.
            input.events.retain(|event| {
                !matches!(
                    event,
                    egui::Event::PointerMoved(_)
                        | egui::Event::PointerButton { .. }
                        | egui::Event::PointerGone
                        | egui::Event::Touch { .. }
                )
            });
            input.events.push(egui::Event::PointerGone);
        }
    }

    fn persist_egui_memory(&self) -> bool {
        !self.is_preview()
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        if !self.args.preview {
            eframe::set_value(storage, "settings", &self.settings);
        }
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Some(event) = self.native_tray.as_ref().and_then(tray::NativeTray::try_recv) {
            match event {
                tray::TrayEvent::Open
                | tray::TrayEvent::Settings
                | tray::TrayEvent::Unavailable => {
                    if event == tray::TrayEvent::Settings {
                        self.page_view = PageType::Settings;
                        self.library_details = false;
                    }
                    if event != tray::TrayEvent::Unavailable || self.tray_hidden {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                    }
                    self.tray_hidden = false;
                }
                tray::TrayEvent::Exit => {
                    self.force_quit = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        }
        // A failed worker must never leave a hidden, inaccessible window.
        if self.tray_hidden
            && !self.native_tray.as_ref().is_some_and(tray::NativeTray::is_available)
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            self.tray_hidden = false;
        }
        if self.tray_hidden {
            ctx.request_repaint_after(std::time::Duration::from_secs(1));
        }
        let close_requested = ctx.input(|input| input.viewport().close_requested());
        if close_requested
            && self.force_quit
            && self.installer_state.is_enqueuing()
            && self.critical_error.is_none()
            && !self.args.preview
        {
            // A sent request is not yet a durable queue entry. Keep the worker alive
            // until its terminal acknowledgment rather than silently losing that job.
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.force_quit = false;
            self.nonfatal_errors.push(BackendError::InstallPreparation(
                "The game is still being added to the download queue. Wait for this request to finish before choosing Exit.".into(),
            ));
        } else if close_requested && !self.force_quit && !self.args.preview {
            // X keeps downloads running. Without a registered, restorable tray,
            // leave a taskbar entry so the existing window remains recoverable.
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            if self.native_tray.as_ref().is_some_and(tray::NativeTray::is_available) {
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                self.tray_hidden = true;
            } else {
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                self.tray_hidden = false;
            }
        }

        if !self.args.preview {
            bridge_processor::frontend_processor(self, ctx);
            event_processor::frontend_processor(self, ctx);
        }
        egui::CentralPanel::default().frame(egui::Frame::none().fill(shell::BACKGROUND)).show(
            ctx,
            |ui| {
                if self.critical_error.is_some() {
                    egui::Frame::none()
                        .fill(Color32::from_rgb(99, 39, 48))
                        .inner_margin(14.0)
                        .show(ui, |ui| {
                            ui.label("The EA connection stopped. Restart the client to reconnect.");
                            if let Some(err) = &self.critical_error {
                                ui.small(err.to_string());
                            }
                            if ui.button("Close client").clicked() {
                                self.force_quit = true;
                                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                        });
                }
                let rect = ui.available_rect_before_wrap();
                self.main(rect, ui);
            },
        );
        if let Some(message) = self.nonfatal_errors.last().map(ToString::to_string) {
            let mut dismiss = false;
            egui::Window::new("Action couldn't finish")
                .collapsible(false)
                .resizable(false)
                .default_width(460.0)
                .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
                .show(ctx, |ui| {
                    ui.label(message);
                    ui.add_space(12.0);
                    dismiss = ui.button("Dismiss").clicked();
                });
            if dismiss {
                self.nonfatal_errors.pop();
            }
        }
        self.rendered_frames += 1;
        if self.args.screenshot.is_some() && !self.screenshot_requested && self.rendered_frames >= 4
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot);
            self.screenshot_requested = true;
        }
        if let Some(path) = &self.args.screenshot {
            for event in ctx.input(|i| i.events.clone()) {
                if let egui::Event::Screenshot { image, .. } = event {
                    let pixels: Vec<u8> = image.pixels.iter().flat_map(|p| p.to_array()).collect();
                    let result = image::save_buffer(
                        path,
                        &pixels,
                        image.width() as u32,
                        image.height() as u32,
                        image::ColorType::Rgba8,
                    );
                    match result {
                        Ok(()) => {
                            self.screenshot_done = true;
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        Err(err) => {
                            eprintln!("Could not save screenshot: {err}");
                            self.screenshot_done = true;
                        }
                    }
                }
            }
            if !self.screenshot_done {
                ctx.request_repaint();
            }
        }
        puffin::GlobalProfiler::lock().new_frame();
    }

    fn on_exit(&mut self, _gl: Option<&glow::Context>) {
        if !self.args.preview {
            let _ = self
                .backend
                .backend_commander
                .send(bridge_thread::AeAppLibRequest::ShutdownRequest);
        }
    }
}
