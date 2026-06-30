//! aspid — a cross-platform Hollow Knight mod manager (Iced front-end).

mod style;
mod theme;

use aspid_core::config::Config;
use aspid_core::game::{self, ApiState, Install};
use aspid_core::modlinks::{ApiManifest, Catalog, Mod};
use aspid_core::mods::{self, InstalledMod};
use aspid_core::share::PackShare;
use aspid_core::skins::{self, HkSkin, SkinStore};
use aspid_core::{launch, modapi, modlinks, modpack};

use std::time::Duration;

use iced::widget::{
    button, column, container, image, mouse_area, pick_list, row, scrollable, stack, svg, text,
    text_input, tooltip, Space,
};

/// The GitHub mark, rendered (and tinted) next to a mod's actions to open its homepage.
const GITHUB_MARK: &[u8] = br##"<svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg"><path d="M12 .297c-6.63 0-12 5.373-12 12 0 5.303 3.438 9.8 8.205 11.385.6.113.82-.258.82-.577 0-.285-.01-1.04-.015-2.04-3.338.724-4.042-1.61-4.042-1.61C4.422 18.07 3.633 17.7 3.633 17.7c-1.087-.744.084-.729.084-.729 1.205.084 1.838 1.236 1.838 1.236 1.07 1.835 2.809 1.305 3.495.998.108-.776.417-1.305.76-1.605-2.665-.3-5.466-1.332-5.466-5.93 0-1.31.465-2.38 1.235-3.22-.135-.303-.54-1.523.105-3.176 0 0 1.005-.322 3.3 1.23.96-.267 1.98-.399 3-.405 1.02.006 2.04.138 3 .405 2.28-1.552 3.285-1.23 3.285-1.23.645 1.653.24 2.873.12 3.176.765.84 1.23 1.91 1.23 3.22 0 4.61-2.805 5.625-5.475 5.92.42.36.81 1.096.81 2.22 0 1.606-.015 2.896-.015 3.286 0 .315.21.69.825.57C20.565 22.092 24 17.592 24 12.297c0-6.627-5.373-12-12-12"/></svg>"##;

/// A generic "external link" icon, shown top-right on externally-hosted skin cards.
const LINK_MARK: &[u8] = br##"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" xmlns="http://www.w3.org/2000/svg"><path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"/><path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"/></svg>"##;

/// A download icon, shown on skin cards aspid can fetch automatically.
const DOWNLOAD_MARK: &[u8] = br##"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" xmlns="http://www.w3.org/2000/svg"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>"##;

/// A check/tick icon, shown on skins already in the library.
const CHECK_MARK: &[u8] = br##"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round" xmlns="http://www.w3.org/2000/svg"><polyline points="20 6 9 17 4 12"/></svg>"##;

// Feather-style nav + action icons (stroke=currentColor; tinted via style::icon).
macro_rules! feather {
    ($body:expr) => {
        concat!(
            r##"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" xmlns="http://www.w3.org/2000/svg">"##,
            $body,
            "</svg>"
        ).as_bytes()
    };
}

const ICON_HOME: &[u8] = feather!(
    r##"<path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/><polyline points="9 22 9 12 15 12 15 22"/>"##
);
const ICON_COMPASS: &[u8] = feather!(
    r##"<circle cx="12" cy="12" r="10"/><polygon points="16.24 7.76 14.12 14.12 7.76 16.24 9.88 9.88"/>"##
);
const ICON_PACKAGE: &[u8] = feather!(
    r##"<path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"/><polyline points="3.27 6.96 12 12.01 20.73 6.96"/><line x1="12" y1="22.08" x2="12" y2="12"/>"##
);
const ICON_LAYERS: &[u8] = feather!(
    r##"<polygon points="12 2 2 7 12 12 22 7 12 2"/><polyline points="2 17 12 22 22 17"/><polyline points="2 12 12 17 22 12"/>"##
);
const ICON_SHIRT: &[u8] =
    feather!(r##"<path d="M8 2 4 5l2 3 2-1v13h8V7l2 1 2-3-4-3c0 0-2 2-4 2s-4-2-4-2Z"/>"##);
const ICON_GEAR: &[u8] = feather!(
    r##"<circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/>"##
);
const ICON_PLAY: &[u8] = br##"<svg viewBox="0 0 24 24" fill="currentColor" xmlns="http://www.w3.org/2000/svg"><polygon points="6 4 20 12 6 20 6 4"/></svg>"##;
const ICON_REFRESH: &[u8] = feather!(
    r##"<polyline points="23 4 23 10 17 10"/><polyline points="1 20 1 14 7 14"/><path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15"/>"##
);
const ICON_COPY: &[u8] = feather!(
    r##"<rect x="9" y="9" width="13" height="13" rx="2" ry="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/>"##
);
const ICON_TRASH: &[u8] = feather!(
    r##"<polyline points="3 6 5 6 21 6"/><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/>"##
);
const ICON_PLUS: &[u8] =
    feather!(r##"<line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/>"##);
const ICON_SHARE: &[u8] = feather!(
    r##"<circle cx="18" cy="5" r="3"/><circle cx="6" cy="12" r="3"/><circle cx="18" cy="19" r="3"/><line x1="8.59" y1="13.51" x2="15.42" y2="17.49"/><line x1="15.41" y1="6.51" x2="8.59" y2="10.49"/>"##
);
const ICON_FOLDER: &[u8] = feather!(
    r##"<path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/>"##
);
const ICON_SEARCH: &[u8] =
    feather!(r##"<circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/>"##);
const ICON_CHEVRON_LEFT: &[u8] = feather!(r##"<polyline points="15 18 9 12 15 6"/>"##);
const ICON_CHEVRON_RIGHT: &[u8] = feather!(r##"<polyline points="9 18 15 12 9 6"/>"##);

/// The application/window icon.
const APP_ICON: &[u8] = include_bytes!("../../../icons/aspid.png");

use iced::{Element, Length, Subscription, Task, Theme};

fn main() -> iced::Result {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "aspid=info,aspid_core=info".into()),
        )
        .init();

    iced::application(App::new, App::update, App::view)
        .title("aspid")
        .theme(App::theme)
        .subscription(App::subscription)
        .default_font(style::REGULAR)
        .font(include_bytes!("../fonts/Inter.ttf").as_slice())
        .window(iced::window::Settings {
            size: iced::Size::new(1040.0, 720.0),
            position: iced::window::Position::Centered,
            min_size: Some(iced::Size::new(820.0, 560.0)),
            icon: iced::window::icon::from_file_data(APP_ICON, None).ok(),
            ..Default::default()
        })
        .run()
}

/// Identifies the card/row currently hovered, for hover styling.
#[derive(Debug, Clone, PartialEq, Eq)]
enum HoverKey {
    ModCard(usize),
    InstalledRow(String),
    PackRow(String),
    SkinCard(String),
}

/// Which top-level screen is currently shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    Dashboard,
    Browse,
    Installed,
    Modpacks,
    Skins,
    Settings,
    /// A mod's detail page (the selected mod is held in `App::detail_mod`).
    ModDetail,
}

impl Screen {
    const ALL: [Screen; 6] = [
        Screen::Dashboard,
        Screen::Browse,
        Screen::Installed,
        Screen::Modpacks,
        Screen::Skins,
        Screen::Settings,
    ];

    fn label(self) -> &'static str {
        match self {
            Screen::Dashboard => "Dashboard",
            Screen::Browse => "Browse",
            Screen::Installed => "Installed",
            Screen::Modpacks => "Modpacks",
            Screen::Skins => "Skins",
            Screen::Settings => "Settings",
            Screen::ModDetail => "Mod",
        }
    }
}

/// Top-level application state.
struct App {
    config: Config,
    theme: Theme,
    screen: Screen,
    install: Option<Install>,
    catalog: Option<Catalog>,
    api_manifest: Option<ApiManifest>,
    installed: Vec<InstalledMod>,
    modpacks: Option<modpack::Manager>,
    new_pack_name: String,
    import_code: String,
    share_code: Option<String>,
    /// Cached brand image handle (rebuilding it each frame re-uploads it → flicker).
    brand: image::Handle,
    skin_store: Option<SkinStore>,
    skin_catalog: Option<Vec<HkSkin>>,
    skin_search: String,
    /// Active Browse filters.
    browse_tags: std::collections::BTreeSet<String>,
    browse_installed_only: bool,
    skins_installed_only: bool,
    /// Catalog index of the external skin whose "how to install" popup is open.
    skin_modal: Option<usize>,
    /// Which card/row the pointer is over (for hover styling).
    hovered: Option<HoverKey>,
    /// The mod shown on the detail screen, and the screen to return to.
    detail_mod: Option<String>,
    detail_return: Screen,
    manual_path: String,
    search: String,
    status: String,
    busy: bool,
    /// Remaining redraw frames to emit via the subscription after a state change.
    redraws_remaining: u8,
}

#[derive(Debug, Clone)]
enum Message {
    Navigate(Screen),
    DetectSteam,
    SteamDetected(Result<Install, String>),
    ManualPathChanged(String),
    SetManualPath,
    RefreshCatalog,
    CatalogLoaded(Result<Catalog, String>),
    ApiManifestLoaded(Result<ApiManifest, String>),
    SearchChanged(String),
    ToggleBrowseTag(String),
    ToggleBrowseInstalled,
    ClearBrowseFilters,
    ToggleSkinsInstalled,
    OpenUrl(String),
    InstallMod(String),
    RemoveMod(String),
    OpenModDetail(String),
    SetModEnabled(String, bool),
    InstallOrUpdateApi,
    LaunchModded,
    LaunchVanilla,
    EnableModpacks,
    NewPackNameChanged(String),
    CreatePack,
    ClonePack(String),
    ActivatePack(String),
    DeletePack(String),
    ExportPack(String),
    SharePackUploaded(Result<String, String>),
    ShareCodeChanged(String),
    ImportCodeChanged(String),
    ImportPack,
    PackResolved(Result<PackShare, String>),
    ThemePresetChanged(String),
    AccentChanged(String),
    SetActiveSkin(&'static str, String),
    RemoveSkin(&'static str, String),
    SyncSkins(&'static str),
    LoadSkinCatalog,
    SkinCatalogLoaded(Result<Vec<HkSkin>, String>),
    SkinSearchChanged(String),
    DownloadSkin(usize),
    ShowExternalSkin(usize),
    CloseSkinModal,
    ImportSkinFile,
    Hover(HoverKey),
    Unhover,
    CopyToClipboard(String),
    /// A background action finished with a human-readable status (or error).
    ActionDone(Result<String, String>),
    /// Drives a few redraw frames after a state change (Wayland repaint workaround).
    Tick,
}

impl App {
    fn new() -> (Self, Task<Message>) {
        let config = Config::load().unwrap_or_default();
        let theme = theme::from_config(&config.theme);
        let install = config
            .game_path
            .as_ref()
            .and_then(|p| game::validate(p).ok());
        let screen = if install.is_some() {
            Screen::Dashboard
        } else {
            Screen::Settings
        };

        let modpacks = install
            .as_ref()
            .and_then(|i| modpack::Manager::for_install(i).ok());
        let manual_path = config
            .game_path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default();

        let mut app = App {
            config,
            theme,
            screen,
            install,
            catalog: None,
            api_manifest: None,
            installed: Vec::new(),
            modpacks,
            new_pack_name: String::new(),
            import_code: String::new(),
            share_code: None,
            brand: image::Handle::from_bytes(APP_ICON),
            skin_store: SkinStore::open().ok(),
            skin_catalog: None,
            skin_search: String::new(),
            browse_tags: std::collections::BTreeSet::new(),
            browse_installed_only: false,
            skins_installed_only: false,
            skin_modal: None,
            hovered: None,
            detail_mod: None,
            detail_return: Screen::Browse,
            manual_path,
            search: String::new(),
            status: String::new(),
            busy: false,
            redraws_remaining: 0,
        };
        let boot = app.refresh_all(false);
        (app, boot)
    }

    fn theme(&self) -> Theme {
        self.theme.clone()
    }

    /// While redraw frames are pending, tick at ~60fps to force the window to repaint;
    /// otherwise stay idle. This delivers async-task results promptly on Wayland.
    fn subscription(&self) -> Subscription<Message> {
        if self.redraws_remaining > 0 {
            iced::time::every(Duration::from_millis(16)).map(|_| Message::Tick)
        } else {
            Subscription::none()
        }
    }

    /// Re-scan installed mods from disk (cheap, synchronous).
    fn refresh_installed(&mut self) {
        if let Some(install) = &self.install {
            self.installed = mods::list_installed(install).unwrap_or_default();
        }
    }

    /// Adopt a validated install: persist its path, (re)build the modpack manager, and
    /// kick off catalog/installed refreshes. Shared by Steam detection and manual entry.
    fn adopt_install(&mut self, install: Install) -> Task<Message> {
        self.config.game_path = Some(install.root.clone());
        let _ = self.config.save();
        self.manual_path = install.root.display().to_string();
        self.modpacks = modpack::Manager::for_install(&install).ok();
        self.install = Some(install);
        self.screen = Screen::Dashboard;
        self.refresh_all(false)
    }

    /// Kick off catalog + API-manifest loads and refresh the installed list.
    fn refresh_all(&mut self, force: bool) -> Task<Message> {
        self.refresh_installed();
        if self.install.is_none() {
            return Task::none();
        }
        let cfg1 = self.config.clone();
        let cfg2 = self.config.clone();
        Task::batch([
            Task::perform(load_catalog(cfg1, force), Message::CatalogLoaded),
            Task::perform(load_api(cfg2, force), Message::ApiManifestLoaded),
        ])
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        // A bare redraw frame: just count down, don't re-arm (avoid an infinite loop).
        if let Message::Tick = message {
            self.redraws_remaining = self.redraws_remaining.saturating_sub(1);
            return Task::none();
        }

        // Only results delivered from a completed async Task need the repaint workaround;
        // input-driven messages are already painted in response to the input event. Arming
        // it for everything made navigation rebuild the (potentially large) view several
        // extra times, which was visible as a hitch when opening Browse.
        let is_async = matches!(
            message,
            Message::SteamDetected(_)
                | Message::CatalogLoaded(_)
                | Message::ApiManifestLoaded(_)
                | Message::SkinCatalogLoaded(_)
                | Message::SharePackUploaded(_)
                | Message::PackResolved(_)
                | Message::ActionDone(_)
        );
        let task = self.handle(message);
        if is_async {
            self.redraws_remaining = 3;
        }
        task
    }

    fn handle(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Tick => Task::none(),
            Message::Navigate(screen) => {
                self.screen = screen;
                Task::none()
            }
            Message::DetectSteam => {
                self.status = "Searching for a Steam installation…".into();
                Task::perform(
                    async { game::locate_steam().map_err(|e| e.to_string()) },
                    Message::SteamDetected,
                )
            }
            Message::SteamDetected(Ok(install)) => {
                self.status = format!("Found install at {}", install.root.display());
                self.adopt_install(install)
            }
            Message::SteamDetected(Err(e)) => {
                self.status = format!("Detection failed: {e}");
                Task::none()
            }
            Message::ManualPathChanged(p) => {
                self.manual_path = p;
                Task::none()
            }
            Message::SetManualPath => match game::validate(self.manual_path.trim()) {
                Ok(install) => {
                    self.status = format!("Using install at {}", install.root.display());
                    self.adopt_install(install)
                }
                Err(e) => {
                    self.status = format!("Invalid game path: {e}");
                    Task::none()
                }
            },
            Message::RefreshCatalog => {
                self.status = "Refreshing catalog…".into();
                self.refresh_all(true)
            }
            Message::CatalogLoaded(Ok(catalog)) => {
                self.status = format!("Loaded {} mods", catalog.len());
                self.catalog = Some(catalog);
                Task::none()
            }
            Message::CatalogLoaded(Err(e)) => {
                self.status = format!("Failed to load catalog: {e}");
                Task::none()
            }
            Message::ApiManifestLoaded(result) => {
                self.api_manifest = result.ok();
                // Honour the auto-update-API preference: if the API is installed and the
                // catalog offers a newer version, update it in the background.
                let should_update = matches!(
                    (&self.install, &self.api_manifest),
                    (Some(install), Some(manifest))
                        if install.api_state().is_installed()
                            && modapi::update_available(install, manifest)
                );
                if should_update && !self.busy {
                    let install = self.install.clone().unwrap();
                    let manifest = self.api_manifest.clone().unwrap();
                    self.busy = true;
                    self.status = "Updating modding API…".into();
                    return Task::perform(do_install_api(install, manifest), Message::ActionDone);
                }
                Task::none()
            }
            Message::SearchChanged(q) => {
                self.search = q;
                Task::none()
            }
            Message::ToggleBrowseTag(t) => {
                if !self.browse_tags.remove(&t) {
                    self.browse_tags.insert(t);
                }
                Task::none()
            }
            Message::ToggleBrowseInstalled => {
                self.browse_installed_only = !self.browse_installed_only;
                Task::none()
            }
            Message::ClearBrowseFilters => {
                self.browse_tags.clear();
                self.browse_installed_only = false;
                Task::none()
            }
            Message::ToggleSkinsInstalled => {
                self.skins_installed_only = !self.skins_installed_only;
                Task::none()
            }
            Message::OpenUrl(url) => {
                if let Err(e) = launch::open_url(&url) {
                    self.status = format!("Couldn't open link: {e}");
                }
                Task::none()
            }
            Message::InstallMod(name) => {
                let (Some(install), Some(catalog)) = (&self.install, &self.catalog) else {
                    return Task::none();
                };
                self.busy = true;
                self.status = format!("Installing {name}…");
                let (install, catalog) = (install.clone(), catalog.clone());
                Task::perform(do_install(install, catalog, name), Message::ActionDone)
            }
            Message::RemoveMod(name) => {
                if let Some(install) = &self.install {
                    let result = remove_with_warning(install, self.catalog.as_ref(), &name);
                    self.apply_sync_result(result);
                }
                Task::none()
            }
            Message::SetModEnabled(name, enabled) => {
                if let Some(install) = &self.install {
                    let result = mods::set_enabled(install, &name, enabled)
                        .map(|()| {
                            format!("{} {name}", if enabled { "Enabled" } else { "Disabled" })
                        })
                        .map_err(|e| e.to_string());
                    self.apply_sync_result(result);
                }
                Task::none()
            }
            Message::InstallOrUpdateApi => {
                let (Some(install), Some(manifest)) = (&self.install, &self.api_manifest) else {
                    return Task::none();
                };
                self.busy = true;
                self.status = "Installing modding API…".into();
                let (install, manifest) = (install.clone(), manifest.clone());
                Task::perform(do_install_api(install, manifest), Message::ActionDone)
            }
            Message::LaunchModded => {
                if let Some(install) = &self.install {
                    let result = (|| {
                        modapi::enable_modded(install)?;
                        launch::launch_via_steam()
                    })()
                    .map(|()| "Launching modded…".to_string())
                    .map_err(|e| e.to_string());
                    self.apply_sync_result(result);
                }
                Task::none()
            }
            Message::LaunchVanilla => {
                if let Some(install) = &self.install {
                    let result = (|| {
                        modapi::disable_for_vanilla(install)?;
                        launch::launch_via_steam()
                    })()
                    .map(|()| "Launching vanilla…".to_string())
                    .map_err(|e| e.to_string());
                    self.apply_sync_result(result);
                }
                Task::none()
            }
            Message::EnableModpacks => {
                let result = self.with_modpacks(|m| {
                    m.ensure_initialized()?;
                    Ok("Modpacks enabled — captured current setup as “Default”".to_string())
                });
                self.apply_sync_result(result);
                Task::none()
            }
            Message::NewPackNameChanged(name) => {
                self.new_pack_name = name;
                Task::none()
            }
            Message::CreatePack => {
                let name = self.new_pack_name.trim().to_string();
                if name.is_empty() {
                    self.status = "Enter a name for the new pack".into();
                    return Task::none();
                }
                let result = self.with_modpacks(move |m| {
                    m.create(&name)?;
                    Ok(format!("Created pack “{name}”"))
                });
                if result.is_ok() {
                    self.new_pack_name.clear();
                }
                self.apply_sync_result(result);
                Task::none()
            }
            Message::ClonePack(id) => {
                let result = self.with_modpacks(move |m| {
                    let base = m
                        .packs()
                        .iter()
                        .find(|p| p.id == id)
                        .map(|p| p.name.clone())
                        .unwrap_or_else(|| id.clone());
                    let name = format!("{base} copy");
                    m.clone_pack(&id, &name)?;
                    Ok(format!("Cloned to “{name}”"))
                });
                self.apply_sync_result(result);
                Task::none()
            }
            Message::ActivatePack(id) => {
                let result = self.with_modpacks(move |m| {
                    m.activate(&id)?;
                    Ok(format!("Activated “{id}”"))
                });
                self.apply_sync_result(result);
                Task::none()
            }
            Message::DeletePack(id) => {
                let result = self.with_modpacks(move |m| {
                    m.delete(&id)?;
                    Ok(format!("Deleted “{id}”"))
                });
                self.apply_sync_result(result);
                Task::none()
            }
            Message::ExportPack(id) => match self.modpacks.as_ref().map(|m| m.export(&id)) {
                Some(Ok(share)) => {
                    self.busy = true;
                    self.status = "Creating share code…".into();
                    Task::perform(upload_share(share), Message::SharePackUploaded)
                }
                Some(Err(e)) => {
                    self.status = format!("Export failed: {e}");
                    Task::none()
                }
                None => Task::none(),
            },
            Message::SharePackUploaded(Ok(code)) => {
                self.busy = false;
                self.status = format!("Share code “{code}” copied to clipboard");
                self.share_code = Some(code.clone());
                iced::clipboard::write(code)
            }
            Message::SharePackUploaded(Err(e)) => {
                self.busy = false;
                self.status = format!("Couldn't create share code: {e}");
                Task::none()
            }
            Message::ShareCodeChanged(s) => {
                // Keep the exported code field selectable without losing it.
                self.share_code = Some(s);
                Task::none()
            }
            Message::ImportCodeChanged(s) => {
                self.import_code = s;
                Task::none()
            }
            Message::ImportPack => {
                let code = self.import_code.trim().to_string();
                if code.is_empty() {
                    self.status = "Paste a modpack code first".into();
                    return Task::none();
                }
                self.busy = true;
                self.status = "Fetching modpack…".into();
                Task::perform(resolve_share(code), Message::PackResolved)
            }
            Message::PackResolved(Ok(share)) => {
                let name = share.name.clone();
                // Create + activate the new pack synchronously, then install its mods.
                let new_id = self.with_modpacks(move |m| {
                    let id = m.create(&name)?;
                    m.activate(&id)?;
                    Ok(id)
                });
                if let Err(e) = new_id {
                    self.busy = false;
                    self.status = format!("Import failed: {e}");
                    return Task::none();
                }
                self.refresh_installed();
                let (Some(install), Some(catalog)) = (&self.install, &self.catalog) else {
                    self.busy = false;
                    self.status = "Catalog not loaded — can't install the pack's mods".into();
                    return Task::none();
                };
                self.status = format!("Importing “{}” ({} mods)…", share.name, share.mods.len());
                self.import_code.clear();
                let names: Vec<String> = share.mods.into_iter().map(|m| m.name).collect();
                let (install, catalog) = (install.clone(), catalog.clone());
                Task::perform(do_import(install, catalog, names), Message::ActionDone)
            }
            Message::PackResolved(Err(e)) => {
                self.busy = false;
                self.status = format!("Invalid or expired code: {e}");
                Task::none()
            }
            Message::ThemePresetChanged(preset) => {
                self.config.theme.preset = preset;
                self.apply_theme();
                Task::none()
            }
            Message::AccentChanged(accent) => {
                let trimmed = accent.trim();
                self.config.theme.accent = if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                };
                self.apply_theme();
                Task::none()
            }
            Message::SetActiveSkin(kind_id, name) => {
                self.config
                    .active_skins
                    .insert(kind_id.to_string(), name.clone());
                let _ = self.config.save();
                self.status = format!("Active skin set to “{name}”");
                Task::none()
            }
            Message::RemoveSkin(kind_id, name) => {
                if let (Some(store), Some(kind)) = (&self.skin_store, kind_by_id(kind_id)) {
                    let result = store
                        .remove(kind, &name)
                        .map(|()| format!("Removed skin “{name}”"))
                        .map_err(|e| e.to_string());
                    if self.config.active_skins.get(kind_id) == Some(&name) {
                        self.config.active_skins.remove(kind_id);
                        let _ = self.config.save();
                    }
                    self.apply_sync_result(result);
                }
                Task::none()
            }
            Message::SyncSkins(kind_id) => {
                if let (Some(store), Some(install), Some(kind)) =
                    (&self.skin_store, &self.install, kind_by_id(kind_id))
                {
                    let result = store
                        .sync_to_game(install, kind)
                        .map(|n| format!("Synced {n} skin(s) to the game"))
                        .map_err(|e| e.to_string());
                    self.apply_sync_result(result);
                }
                Task::none()
            }
            Message::LoadSkinCatalog => {
                self.status = "Loading skins from hkskins.art…".into();
                self.busy = true;
                let url = self.config.skin_catalog_url().to_string();
                let force = self.skin_catalog.is_some();
                Task::perform(load_skin_catalog(url, force), Message::SkinCatalogLoaded)
            }
            Message::SkinCatalogLoaded(Ok(skins)) => {
                self.busy = false;
                self.status = format!("Loaded {} skins from hkskins.art", skins.len());
                self.skin_catalog = Some(skins);
                Task::none()
            }
            Message::SkinCatalogLoaded(Err(e)) => {
                self.busy = false;
                self.status = format!("Failed to load skins: {e}");
                Task::none()
            }
            Message::SkinSearchChanged(q) => {
                self.skin_search = q;
                Task::none()
            }
            Message::DownloadSkin(index) => {
                let skin = self
                    .skin_catalog
                    .as_ref()
                    .and_then(|c| c.get(index))
                    .cloned();
                let (Some(skin), Some(store)) = (skin, self.skin_store.clone()) else {
                    return Task::none();
                };
                self.busy = true;
                self.status = format!("Downloading skin “{}”…", skin.name);
                Task::perform(download_skin(store, skin), Message::ActionDone)
            }
            Message::OpenModDetail(name) => {
                self.detail_mod = Some(name);
                self.detail_return = self.screen;
                self.hovered = None;
                self.screen = Screen::ModDetail;
                Task::none()
            }
            Message::Hover(key) => {
                self.hovered = Some(key);
                Task::none()
            }
            Message::Unhover => {
                self.hovered = None;
                Task::none()
            }
            Message::CopyToClipboard(s) => {
                self.status = "Copied to clipboard".into();
                iced::clipboard::write(s)
            }
            Message::ShowExternalSkin(index) => {
                self.skin_modal = Some(index);
                Task::none()
            }
            Message::CloseSkinModal => {
                self.skin_modal = None;
                Task::none()
            }
            Message::ImportSkinFile => {
                let Some(store) = self.skin_store.clone() else {
                    return Task::none();
                };
                self.busy = true;
                self.skin_modal = None;
                self.status = "Choose the downloaded skin file…".into();
                Task::perform(import_skin_file(store), Message::ActionDone)
            }
            Message::ActionDone(result) => {
                self.busy = false;
                self.apply_sync_result(result);
                Task::none()
            }
        }
    }

    /// Rebuild the live theme from config and persist the change.
    fn apply_theme(&mut self) {
        self.theme = theme::from_config(&self.config.theme);
        let _ = self.config.save();
    }

    /// Run an operation against the modpack manager, mapping core errors to strings.
    fn with_modpacks<F>(&mut self, f: F) -> Result<String, String>
    where
        F: FnOnce(&mut modpack::Manager) -> aspid_core::Result<String>,
    {
        match self.modpacks.as_mut() {
            Some(m) => f(m).map_err(|e| e.to_string()),
            None => Err("No game configured".to_string()),
        }
    }

    /// Apply a finished action's result: set status and refresh installed state.
    fn apply_sync_result(&mut self, result: Result<String, String>) {
        match result {
            Ok(msg) => self.status = msg,
            Err(e) => self.status = format!("Error: {e}"),
        }
        self.refresh_installed();
    }

    fn is_installed(&self, name: &str) -> bool {
        self.installed.iter().any(|m| m.name == name)
    }

    // ---- Views ---------------------------------------------------------------

    fn view(&self) -> Element<'_, Message> {
        let content = match self.screen {
            Screen::Dashboard => self.view_dashboard(),
            Screen::Browse => self.view_browse(),
            Screen::Installed => self.view_installed(),
            Screen::Modpacks => self.view_modpacks(),
            Screen::Skins => self.view_skins(),
            Screen::Settings => self.view_settings(),
            Screen::ModDetail => self.view_mod_detail(),
        };

        // Centre the screen content at a comfortable max width, with breathing room.
        let framed = container(content)
            .max_width(style::CONTENT_MAX)
            .width(Length::Fill)
            .height(Length::Fill);
        let area = container(framed)
            .center_x(Length::Fill)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(style::XL);

        let body = column![area, self.status_bar()].width(Length::Fill);

        let layout = row![self.sidebar(), body].height(Length::Fill);
        let base = container(layout)
            .style(style::root)
            .width(Length::Fill)
            .height(Length::Fill);

        // Overlay the "external skin" popup, if open.
        if let Some(skin) = self
            .skin_modal
            .and_then(|i| self.skin_catalog.as_ref().and_then(|c| c.get(i)))
        {
            let backdrop = mouse_area(
                container(Space::new())
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .style(style::backdrop),
            )
            .on_press(Message::CloseSkinModal);

            let dialog = container(self.external_skin_dialog(skin))
                .center_x(Length::Fill)
                .center_y(Length::Fill);

            stack![base, backdrop, dialog].into()
        } else {
            base.into()
        }
    }

    /// The "this skin is hosted externally" popup body.
    fn external_skin_dialog<'a>(&self, skin: &'a HkSkin) -> Element<'a, Message> {
        let open = button(text("Open page"))
            .style(style::secondary)
            .on_press_maybe(
                (!skin.source.is_empty()).then(|| Message::OpenUrl(skin.source.clone())),
            );
        let import = button(text("Import downloaded file…"))
            .style(style::primary)
            .on_press_maybe((!self.busy).then_some(Message::ImportSkinFile));

        let body = column![
            text(skin.name.clone()).size(18).font(style::SEMIBOLD),
            text(
                "This skin is hosted on another site. Open its page to download the \
                 skin, then import the downloaded file into your library."
            )
            .size(13)
            .style(style::muted),
            row![open, import].spacing(style::SM),
            container(
                button(text("Close"))
                    .style(style::ghost)
                    .on_press(Message::CloseSkinModal),
            )
            .center_x(Length::Fill),
        ]
        .spacing(style::MD);

        container(body)
            .width(Length::Fixed(400.0))
            .padding(style::LG)
            .style(style::card)
            .into()
    }

    fn nav_item(&self, screen: Screen) -> Element<'_, Message> {
        let active = screen == self.screen;
        let istyle: fn(&Theme, svg::Status) -> svg::Style = if active {
            style::icon_accent
        } else {
            style::icon
        };
        button(
            row![
                svg_icon(screen_icon(screen), 17.0, istyle),
                text(screen.label()).size(14).font(if active {
                    style::SEMIBOLD
                } else {
                    style::MEDIUM
                }),
            ]
            .spacing(style::SM)
            .align_y(iced::Alignment::Center),
        )
        .width(Length::Fill)
        .padding(style::pad(style::SM, style::MD))
        .style(style::nav(active))
        .on_press(Message::Navigate(screen))
        .into()
    }

    fn sidebar(&self) -> Element<'_, Message> {
        let brand = container(
            row![
                image(self.brand.clone())
                    .width(Length::Fixed(26.0))
                    .height(Length::Fixed(26.0))
                    .content_fit(iced::ContentFit::Contain),
                text("aspid")
                    .size(22)
                    .font(style::SEMIBOLD)
                    .style(style::accent),
            ]
            .spacing(style::SM)
            .align_y(iced::Alignment::Center),
        )
        .padding(style::pad(style::SM, style::MD));

        let mut nav = column![]
            .spacing(style::XS)
            .padding(style::pad(0.0, style::SM));
        for screen in Screen::ALL {
            if screen == Screen::Settings {
                continue; // pinned to the footer
            }
            nav = nav.push(self.nav_item(screen));
        }

        // Footer: a compact active-pack pill plus Settings.
        let summary = match &self.install {
            None => "No game set".to_string(),
            Some(_) => match self.modpacks.as_ref().and_then(|m| m.active()) {
                Some(active) => self
                    .modpacks
                    .as_ref()
                    .and_then(|m| m.packs().iter().find(|p| p.id == active))
                    .map(|p| p.name.clone())
                    .unwrap_or_else(|| active.to_string()),
                None => "Game ready".to_string(),
            },
        };
        let pack_pill = container(
            row![
                svg_icon(ICON_LAYERS, 13.0, style::icon),
                text(summary).size(12).style(style::muted),
            ]
            .spacing(style::XS)
            .align_y(iced::Alignment::Center),
        )
        .padding(style::pad(style::XS, style::MD));

        let footer = column![pack_pill, self.nav_item(Screen::Settings)]
            .spacing(style::XS)
            .padding(style::pad(0.0, style::SM));

        let col = column![brand, nav, Space::new().height(Length::Fill), footer,]
            .spacing(style::SM)
            .height(Length::Fill);

        container(col)
            .style(style::sidebar)
            .width(Length::Fixed(style::SIDEBAR_W))
            .height(Length::Fill)
            .padding(style::pad(style::MD, 0.0))
            .into()
    }

    fn status_bar(&self) -> Element<'_, Message> {
        let label = if self.status.is_empty() {
            "Ready".to_string()
        } else {
            self.status.clone()
        };
        let mut bar = row![].spacing(style::SM).align_y(iced::Alignment::Center);
        if self.busy {
            bar = bar.push(
                container(Space::new())
                    .width(Length::Fixed(8.0))
                    .height(Length::Fixed(8.0))
                    .style(style::dot),
            );
        }
        bar = bar.push(text(label).size(13).style(if self.busy {
            style::accent
        } else {
            style::muted
        }));

        container(bar)
            .width(Length::Fill)
            .padding(style::pad(style::SM, style::XL))
            .style(style::status_bar)
            .into()
    }

    fn view_dashboard(&self) -> Element<'_, Message> {
        let Some(install) = &self.install else {
            return column![
                header(
                    "Dashboard",
                    Some("Connect your game to get started.".into()),
                    None
                ),
                card(
                    text(
                        "No Hollow Knight install is configured yet. Open Settings to detect it \
                         via Steam or enter the folder manually."
                    )
                    .size(14)
                ),
            ]
            .spacing(style::LG)
            .into();
        };

        let state = install.api_state();
        let api_update = matches!(
            (&self.api_manifest, state.is_installed()),
            (Some(m), true) if modapi::update_available(install, m)
        );
        let launch_enabled = !self.busy && state.is_installed();

        let status_chip = match state {
            ApiState::Installed => chip("Modded".into(), style::chip_success),
            ApiState::DisabledForVanilla => chip("Running vanilla".into(), style::chip_warn),
            ApiState::NotInstalled => chip("Vanilla".into(), style::chip_neutral),
            ApiState::Missing => chip("Broken install".into(), style::chip_warn),
        };

        let full_path = install.root.display().to_string();
        let path_el = tooltip(
            text(truncate_middle(&full_path, 56))
                .size(12)
                .style(style::muted),
            container(text(full_path).size(12))
                .padding(style::pad(style::XXS, style::SM))
                .style(style::card),
            tooltip::Position::Bottom,
        )
        .gap(6.0);

        let hero = container(
            column![
                row![
                    column![
                        text("Hollow Knight").size(19).font(style::SEMIBOLD),
                        path_el,
                    ]
                    .spacing(style::XS)
                    .width(Length::Fill),
                    status_chip,
                ]
                .align_y(iced::Alignment::Center)
                .spacing(style::MD),
                row![
                    labeled_button(
                        ICON_PLAY,
                        style::icon_on_accent,
                        "Launch modded",
                        style::primary,
                        launch_enabled.then_some(Message::LaunchModded),
                    )
                    .padding(style::pad(style::SM, style::LG)),
                    labeled_button(
                        ICON_PLAY,
                        style::icon,
                        "Launch vanilla",
                        style::secondary,
                        launch_enabled.then_some(Message::LaunchVanilla),
                    )
                    .padding(style::pad(style::SM, style::LG)),
                ]
                .spacing(style::SM),
            ]
            .spacing(style::LG),
        )
        .padding(style::XL)
        .width(Length::Fill)
        .style(style::hero);

        let api_button_label = if !state.is_installed() {
            "Install API"
        } else if api_update {
            "Update API"
        } else {
            "Reinstall API"
        };
        let api_version = modapi::installed_version(install)
            .map(|v| format!("Installed · v{v}"))
            .unwrap_or_else(|| "Not installed".to_string());
        let mut api_meta = row![text(api_version).size(12).style(style::muted)].spacing(style::SM);
        if api_update {
            api_meta = api_meta.push(chip("Update available".into(), style::chip_warn));
        }
        let api_card = card(
            row![
                column![style::section("Modding API"), api_meta]
                    .spacing(style::XS)
                    .width(Length::Fill),
                labeled_button(
                    if api_update {
                        ICON_REFRESH
                    } else {
                        DOWNLOAD_MARK
                    },
                    if api_update {
                        style::icon_on_accent
                    } else {
                        style::icon
                    },
                    api_button_label,
                    if api_update {
                        style::primary
                    } else {
                        style::secondary
                    },
                    (!self.busy && self.api_manifest.is_some())
                        .then_some(Message::InstallOrUpdateApi),
                ),
            ]
            .align_y(iced::Alignment::Center)
            .spacing(style::MD),
        );

        let mut stats =
            row![stat(ICON_PACKAGE, format!("{} mods", self.installed.len()))].spacing(style::SM);
        if let Some(pack) = self.modpacks.as_ref().and_then(|m| m.active()) {
            stats = stats.push(stat(ICON_LAYERS, format!("Pack · {pack}")));
        }
        if let Some(v) = modapi::installed_version(install) {
            stats = stats.push(stat(ICON_GEAR, format!("API v{v}")));
        }

        column![
            header(
                "Dashboard",
                Some("Launch and manage your game.".into()),
                None
            ),
            hero,
            api_card,
            stats.wrap(),
        ]
        .spacing(style::LG)
        .into()
    }

    fn view_browse(&self) -> Element<'_, Message> {
        let search: Element<'_, Message> = text_input("Search mods…", &self.search)
            .on_input(Message::SearchChanged)
            .padding(style::pad(style::SM, style::MD))
            .style(style::input)
            .width(Length::Fixed(280.0))
            .into();

        let Some(catalog) = &self.catalog else {
            return column![
                header("Browse", None, Some(search)),
                card(
                    text("Catalog not loaded yet — it loads automatically once a game is set.")
                        .size(14)
                ),
            ]
            .spacing(style::LG)
            .into();
        };

        const CAP: usize = 150;
        let query = self.search.to_lowercase();
        let matches: Vec<(usize, &Mod)> = catalog
            .mods()
            .iter()
            .enumerate()
            .filter(|(_, m)| {
                let search_ok = query.is_empty()
                    || m.name.to_lowercase().contains(&query)
                    || m.description.to_lowercase().contains(&query);
                // Tags: a mod matches if it shares ANY selected tag (union).
                let tags_ok = self.browse_tags.is_empty()
                    || m.tags.iter().any(|t| self.browse_tags.contains(t));
                let installed_ok = !self.browse_installed_only || self.is_installed(&m.name);
                search_ok && tags_ok && installed_ok
            })
            .collect();

        let mut col = column![].spacing(style::SM);
        for (i, m) in matches.iter().take(CAP) {
            col = col.push(self.mod_row(*i, m));
        }

        let subtitle = if matches.len() > CAP {
            format!("{} mods · showing {CAP} — narrow further", catalog.len())
        } else {
            format!("{} mods · {} match", catalog.len(), matches.len())
        };

        column![
            header("Browse", Some(subtitle), Some(search)),
            self.browse_filters(catalog),
            screen_scroll(col),
        ]
        .spacing(style::MD)
        .into()
    }

    /// The wrapping filter bar (category toggle-chips + Installed + Clear).
    fn browse_filters<'a>(&'a self, catalog: &'a Catalog) -> Element<'a, Message> {
        // Distinct tags across the catalog, sorted.
        let mut tags: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
        for m in catalog.mods() {
            for t in &m.tags {
                tags.insert(t.as_str());
            }
        }

        let toggle = |label: String, selected: bool, msg: Message| -> Element<'a, Message> {
            button(text(label).size(11).font(style::MEDIUM))
                .padding(style::pad(2.0, style::SM))
                .style(style::toggle_chip(selected))
                .on_press(msg)
                .into()
        };

        let mut bar = row![].spacing(style::XS).align_y(iced::Alignment::Center);
        bar = bar.push(toggle(
            "Installed".to_string(),
            self.browse_installed_only,
            Message::ToggleBrowseInstalled,
        ));
        for t in tags {
            bar = bar.push(toggle(
                t.to_string(),
                self.browse_tags.contains(t),
                Message::ToggleBrowseTag(t.to_string()),
            ));
        }
        if self.browse_installed_only || !self.browse_tags.is_empty() {
            bar = bar.push(
                button(text("Clear").size(11).font(style::MEDIUM))
                    .padding(style::pad(2.0, style::SM))
                    .style(style::ghost)
                    .on_press(Message::ClearBrowseFilters),
            );
        }
        bar.wrap().into()
    }

    /// A clickable mod card on Browse (opens the detail page; highlights on hover).
    fn mod_row<'a>(&'a self, index: usize, m: &'a Mod) -> Element<'a, Message> {
        let installed = self.is_installed(&m.name);
        let hovered = self.hovered == Some(HoverKey::ModCard(index));

        let mut chips =
            row![chip(format!("v{}", m.version), style::chip_neutral)].spacing(style::XS);
        for tag in m.tags.iter().take(3) {
            chips = chips.push(tag_chip(tag));
        }

        let info = column![
            style::strong(m.name.clone()),
            text(truncate(&m.description, 116))
                .size(12)
                .style(style::muted),
            chips,
        ]
        .spacing(style::XS)
        .width(Length::Fill);

        let indicator: Element<'a, Message> = if installed {
            chip("Installed".into(), style::chip_success)
        } else {
            svg_icon(ICON_CHEVRON_RIGHT, 18.0, style::icon)
        };

        let body = row![info, indicator]
            .spacing(style::MD)
            .align_y(iced::Alignment::Center);

        mouse_area(card_h(body, hovered))
            .on_enter(Message::Hover(HoverKey::ModCard(index)))
            .on_exit(Message::Unhover)
            .on_press(Message::OpenModDetail(m.name.clone()))
            .into()
    }

    fn view_installed(&self) -> Element<'_, Message> {
        if self.installed.is_empty() {
            return column![
                header("Installed", None, None),
                card(text("Nothing installed yet — find mods in Browse.").size(14)),
            ]
            .spacing(style::LG)
            .into();
        }

        let mut list = column![].spacing(style::SM);
        for m in &self.installed {
            let update = self
                .catalog
                .as_ref()
                .and_then(|c| c.get(&m.name))
                .map(|cm| m.update_available(cm))
                .unwrap_or(false);
            let hovered = self.hovered == Some(HoverKey::InstalledRow(m.name.clone()));

            let version = m.version.clone().unwrap_or_else(|| "?".into());
            let mut chips =
                row![chip(format!("v{version}"), style::chip_neutral)].spacing(style::XS);
            if m.enabled {
                chips = chips.push(chip("Enabled".into(), style::chip_success));
            } else {
                chips = chips.push(chip("Disabled".into(), style::chip_neutral));
            }
            if update {
                chips = chips.push(chip("Update available".into(), style::chip_warn));
            }

            let enabled = m.enabled;
            let name = m.name.clone();

            let info = mouse_area(
                column![style::strong(m.name.clone()), chips]
                    .spacing(style::XS)
                    .width(Length::Fill),
            )
            .on_press(Message::OpenModDetail(m.name.clone()));

            let actions = row![
                button(text(if enabled { "Disable" } else { "Enable" }))
                    .style(style::secondary)
                    .padding(style::pad(style::SM, style::MD))
                    .on_press_maybe(
                        (!self.busy).then(|| Message::SetModEnabled(name.clone(), !enabled))
                    ),
                labeled_button(
                    ICON_TRASH,
                    style::icon,
                    "Remove",
                    style::danger,
                    (!self.busy).then(|| Message::RemoveMod(name.clone())),
                ),
            ]
            .spacing(style::SM);

            let body = row![info, actions]
                .spacing(style::MD)
                .align_y(iced::Alignment::Center);

            list = list.push(
                mouse_area(card_h(body, hovered))
                    .on_enter(Message::Hover(HoverKey::InstalledRow(m.name.clone())))
                    .on_exit(Message::Unhover),
            );
        }

        let subtitle = format!("{} installed", self.installed.len());
        column![
            header("Installed", Some(subtitle), None),
            screen_scroll(list)
        ]
        .spacing(style::LG)
        .into()
    }

    fn view_modpacks(&self) -> Element<'_, Message> {
        let Some(manager) = &self.modpacks else {
            return column![
                header("Modpacks", None, None),
                card(text("No game configured yet — head to Settings.").size(14)),
            ]
            .spacing(style::LG)
            .into();
        };

        // Not yet initialised: explain the one-time capture and offer to enable.
        if manager.active().is_none() {
            return column![
                header(
                    "Modpacks",
                    Some("Separate mods and saves per setup.".into()),
                    None
                ),
                card(
                    column![
                        text("Enable modpacks").size(16),
                        text(
                            "Each pack gets its own mods and save files. Enabling captures your \
                             current setup as a “Default” pack (your data is moved, never \
                             deleted) and adds an empty “Vanilla” pack."
                        )
                        .size(13)
                        .style(style::muted),
                        button(text("Enable modpacks"))
                            .style(style::primary)
                            .padding(style::pad(style::SM, style::LG))
                            .on_press_maybe((!self.busy).then_some(Message::EnableModpacks)),
                    ]
                    .spacing(style::MD)
                ),
            ]
            .spacing(style::LG)
            .into();
        }

        let active = manager.active().map(str::to_string);
        let mut list = column![].spacing(style::SM);
        for pack in manager.packs() {
            let is_active = active.as_deref() == Some(pack.id.as_str());
            let id = pack.id.clone();
            let deletable = !is_active && pack.id != modpack::VANILLA_ID;
            let hovered = self.hovered == Some(HoverKey::PackRow(pack.id.clone()));

            let mut title_row = row![style::strong(pack.name.clone())]
                .spacing(style::SM)
                .align_y(iced::Alignment::Center);
            if is_active {
                title_row = title_row.push(chip("Active".into(), style::chip_success));
            }

            let actions = row![
                labeled_button(
                    CHECK_MARK,
                    style::icon_on_accent,
                    "Activate",
                    style::primary,
                    (!self.busy && !is_active).then(|| Message::ActivatePack(id.clone())),
                ),
                icon_button(
                    ICON_SHARE,
                    (!self.busy).then(|| Message::ExportPack(id.clone())),
                    "Share",
                ),
                icon_button(
                    ICON_COPY,
                    (!self.busy).then(|| Message::ClonePack(id.clone())),
                    "Clone",
                ),
                icon_button(
                    ICON_TRASH,
                    (!self.busy && deletable).then(|| Message::DeletePack(id.clone())),
                    "Delete",
                ),
            ]
            .spacing(style::XS)
            .align_y(iced::Alignment::Center);

            let body = row![title_row.width(Length::Fill), actions]
                .spacing(style::MD)
                .align_y(iced::Alignment::Center);

            let row_card: Element<'_, Message> = if is_active {
                container(body)
                    .padding(style::LG)
                    .width(Length::Fill)
                    .style(style::hero)
                    .into()
            } else {
                mouse_area(card_h(body, hovered))
                    .on_enter(Message::Hover(HoverKey::PackRow(pack.id.clone())))
                    .on_exit(Message::Unhover)
                    .into()
            };
            list = list.push(row_card);
        }

        let create: Element<'_, Message> = row![
            text_input("New pack name…", &self.new_pack_name)
                .on_input(Message::NewPackNameChanged)
                .on_submit(Message::CreatePack)
                .padding(style::pad(style::SM, style::MD))
                .style(style::input)
                .width(Length::Fixed(190.0)),
            labeled_button(
                ICON_PLUS,
                style::icon_on_accent,
                "Create",
                style::primary,
                (!self.busy).then_some(Message::CreatePack),
            ),
        ]
        .spacing(style::SM)
        .align_y(iced::Alignment::Center)
        .into();

        // Import / share card.
        let mut share_card = column![
            style::section("Share & import"),
            text("Share a pack to copy its mod list as a code. Paste one here to recreate it.")
                .size(12)
                .style(style::muted),
            row![
                text_input("Paste a modpack code…", &self.import_code)
                    .on_input(Message::ImportCodeChanged)
                    .on_submit(Message::ImportPack)
                    .padding(style::pad(style::SM, style::MD))
                    .style(style::input)
                    .width(Length::Fill),
                labeled_button(
                    DOWNLOAD_MARK,
                    style::icon_on_accent,
                    "Import",
                    style::primary,
                    (!self.busy).then_some(Message::ImportPack),
                ),
            ]
            .spacing(style::SM)
            .align_y(iced::Alignment::Center),
        ]
        .spacing(style::SM);

        if let Some(code) = &self.share_code {
            share_card = share_card.push(
                row![
                    text_input("", code)
                        .on_input(Message::ShareCodeChanged)
                        .padding(style::pad(style::SM, style::MD))
                        .style(style::input)
                        .width(Length::Fill),
                    icon_button(
                        ICON_COPY,
                        Some(Message::CopyToClipboard(code.clone())),
                        "Copy code",
                    ),
                ]
                .spacing(style::SM)
                .align_y(iced::Alignment::Center),
            );
        }

        column![
            header("Modpacks", None, Some(create)),
            screen_scroll(list),
            card(share_card),
        ]
        .spacing(style::LG)
        .into()
    }

    fn view_skins(&self) -> Element<'_, Message> {
        let Some(store) = &self.skin_store else {
            return column![
                header("Skins", None, None),
                card(text("Skin storage unavailable.").size(14)),
            ]
            .spacing(style::LG)
            .into();
        };

        let mut col = column![header(
            "Skins",
            Some("Cosmetics that persist across modpacks.".into()),
            None
        )]
        .spacing(style::LG);

        // Boss Bar keeps a small library list (it has no online catalog).
        col = col.push(self.boss_bar_section(store));

        // Custom Knight: controls + a card grid that is the management surface.
        let ck = skins::CUSTOM_KNIGHT;
        let ck_installed = self
            .install
            .as_ref()
            .map(|i| skins::is_mod_installed(i, ck))
            .unwrap_or(false);
        let ck_status = if ck_installed {
            chip("Installed".into(), style::chip_success)
        } else {
            chip("Mod not installed".into(), style::chip_neutral)
        };

        let controls_head = row![
            style::section("Custom Knight skins"),
            ck_status,
            container(Space::new()).width(Length::Fill),
            labeled_button(
                ICON_REFRESH,
                style::icon,
                "Sync to game",
                style::secondary,
                (!self.busy && ck_installed).then_some(Message::SyncSkins(ck.id)),
            ),
            labeled_button(
                ICON_FOLDER,
                style::icon,
                "Import file…",
                style::secondary,
                (!self.busy).then_some(Message::ImportSkinFile),
            ),
            labeled_button(
                ICON_REFRESH,
                if self.skin_catalog.is_some() {
                    style::icon
                } else {
                    style::icon_on_accent
                },
                if self.skin_catalog.is_some() {
                    "Reload"
                } else {
                    "Load catalog"
                },
                if self.skin_catalog.is_some() {
                    style::secondary
                } else {
                    style::primary
                },
                (!self.busy).then_some(Message::LoadSkinCatalog),
            ),
        ]
        .spacing(style::SM)
        .align_y(iced::Alignment::Center);

        let installed_toggle: Element<'_, Message> =
            button(text("Installed only").size(11).font(style::MEDIUM))
                .padding(style::pad(2.0, style::SM))
                .style(style::toggle_chip(self.skins_installed_only))
                .on_press(Message::ToggleSkinsInstalled)
                .into();

        let mut controls = column![
            controls_head,
            row![
                text_input("Search skins…", &self.skin_search)
                    .on_input(Message::SkinSearchChanged)
                    .padding(style::pad(style::SM, style::MD))
                    .style(style::input)
                    .width(Length::Fill),
                installed_toggle,
            ]
            .spacing(style::SM)
            .align_y(iced::Alignment::Center),
        ]
        .spacing(style::SM);

        if self.skin_catalog.is_none() {
            controls = controls.push(
                text(
                    "Browse 600+ community skins from hkskins.art. Click a skin to set it \
                     active; install via the corner action, or “Import file…” for ones \
                     hosted elsewhere.",
                )
                .size(12)
                .style(style::muted),
            );
        }
        col = col.push(card(controls));

        // Build the grid: library-only skins (imported) first, then the catalog.
        const CAP: usize = 90;
        const COLS: usize = 3;
        let q = self.skin_search.to_lowercase();
        let pass = |name: &str, author: &str| {
            q.is_empty() || name.to_lowercase().contains(&q) || author.to_lowercase().contains(&q)
        };

        let library: Vec<String> = store.list(ck).unwrap_or_default();
        let lib_lower: std::collections::HashSet<String> =
            library.iter().map(|n| n.to_lowercase()).collect();
        let catalog: &[HkSkin] = self.skin_catalog.as_deref().unwrap_or(&[]);
        let catalog_lower: std::collections::HashSet<String> =
            catalog.iter().map(|s| s.name.to_lowercase()).collect();

        // Imported skins not present in the catalog become simple cards.
        let lib_only: Vec<HkSkin> = library
            .iter()
            .filter(|n| !catalog_lower.contains(&n.to_lowercase()))
            .map(|n| HkSkin {
                name: n.clone(),
                author: String::new(),
                desc: String::new(),
                source: String::new(),
                date_added: String::new(),
                preview: store.skin_preview(ck, n),
            })
            .collect();

        let mut items: Vec<(Option<usize>, &HkSkin)> = Vec::new();
        for s in &lib_only {
            if pass(&s.name, "") {
                items.push((None, s));
            }
        }
        for (i, s) in catalog.iter().enumerate() {
            let inst = lib_lower.contains(&s.name.to_lowercase());
            if pass(&s.name, &s.author) && (!self.skins_installed_only || inst) {
                items.push((Some(i), s));
            }
        }
        let shown = items.len();
        items.truncate(CAP);

        col = col.push(
            text(if shown > CAP {
                format!("showing {CAP} of {shown} — search to narrow")
            } else {
                format!("{shown} skins")
            })
            .size(12)
            .style(style::muted),
        );

        let mut grid = column![].spacing(style::MD);
        for chunk in items.chunks(COLS) {
            let mut r = row![]
                .spacing(style::MD)
                .height(Length::Shrink)
                .align_y(iced::Alignment::Start);
            for (idx, skin) in chunk {
                let installed = lib_lower.contains(&skin.name.to_lowercase());
                r = r.push(self.skin_card(*idx, skin, installed));
            }
            // Pad the final row so cards keep equal width.
            for _ in chunk.len()..COLS {
                r = r.push(container(Space::new()).width(Length::Fill));
            }
            grid = grid.push(r);
        }
        col = col.push(grid);

        screen_scroll(col)
    }

    /// Boss Bar's compact library list (no online catalog to browse).
    fn boss_bar_section<'a>(&'a self, store: &'a SkinStore) -> Element<'a, Message> {
        let kind = skins::BOSS_BAR;
        let installed = self
            .install
            .as_ref()
            .map(|i| skins::is_mod_installed(i, kind))
            .unwrap_or(false);
        let status = if installed {
            chip("Installed".into(), style::chip_success)
        } else {
            chip("Mod not installed".into(), style::chip_neutral)
        };
        let head = row![
            style::section("Boss Bar"),
            status,
            container(Space::new()).width(Length::Fill),
            labeled_button(
                ICON_REFRESH,
                style::icon,
                "Sync to game",
                style::secondary,
                (!self.busy && installed).then_some(Message::SyncSkins(kind.id)),
            ),
        ]
        .spacing(style::SM)
        .align_y(iced::Alignment::Center);

        let mut section = column![head].spacing(style::SM);
        let library = store.list(kind).unwrap_or_default();
        if library.is_empty() {
            section = section.push(
                text("No boss-bar skins in your library yet.")
                    .size(12)
                    .style(style::muted),
            );
        } else {
            let active = self.config.active_skins.get(kind.id);
            for name in library {
                let is_active = active == Some(&name);
                let mut label_row = row![text(name.clone()).size(14)]
                    .spacing(style::SM)
                    .align_y(iced::Alignment::Center);
                if is_active {
                    label_row = label_row.push(chip("Active".into(), style::chip_success));
                }
                section = section.push(
                    row![
                        label_row.width(Length::Fill),
                        button(text("Set active"))
                            .style(style::secondary)
                            .padding(style::pad(style::SM, style::MD))
                            .on_press_maybe(
                                (!is_active).then(|| Message::SetActiveSkin(kind.id, name.clone()))
                            ),
                        icon_button(
                            ICON_TRASH,
                            (!self.busy).then(|| Message::RemoveSkin(kind.id, name.clone())),
                            "Remove",
                        ),
                    ]
                    .spacing(style::SM)
                    .align_y(iced::Alignment::Center),
                );
            }
        }
        card(section)
    }

    /// A skin card on the Custom Knight grid. Click sets it active (if installed) or
    /// downloads/opens it; the corner action removes (installed) or installs.
    /// `index` is the catalog index (None for imported, library-only skins).
    fn skin_card<'a>(
        &'a self,
        index: Option<usize>,
        skin: &HkSkin,
        installed: bool,
    ) -> Element<'a, Message> {
        let ck = skins::CUSTOM_KNIGHT;
        fn icon(
            mark: &'static [u8],
            sty: fn(&Theme, svg::Status) -> svg::Style,
        ) -> svg::Svg<'static, Theme> {
            svg(svg::Handle::from_memory(mark))
                .width(Length::Fixed(16.0))
                .height(Length::Fixed(16.0))
                .style(sty)
        }

        let active = self
            .config
            .active_skins
            .get(ck.id)
            .map(|a| a.eq_ignore_ascii_case(&skin.name))
            .unwrap_or(false);
        let hovered = self.hovered == Some(HoverKey::SkinCard(skin.name.clone()));

        // Corner action.
        let corner: Element<'a, Message> = if installed {
            button(icon(ICON_TRASH, style::icon))
                .style(style::ghost)
                .padding(style::XS)
                .on_press_maybe((!self.busy).then(|| Message::RemoveSkin(ck.id, skin.name.clone())))
                .into()
        } else if skin.is_auto_downloadable() {
            button(icon(DOWNLOAD_MARK, style::icon))
                .style(style::ghost)
                .padding(style::XS)
                .on_press_maybe(
                    index.and_then(|i| (!self.busy).then_some(Message::DownloadSkin(i))),
                )
                .into()
        } else {
            button(icon(LINK_MARK, style::icon))
                .style(style::ghost)
                .padding(style::XS)
                .on_press_maybe(index.map(Message::ShowExternalSkin))
                .into()
        };

        // What clicking the card body does.
        let primary = if installed {
            Message::SetActiveSkin(ck.id, skin.name.clone())
        } else if skin.is_auto_downloadable() {
            index.map(Message::DownloadSkin).unwrap_or(Message::Unhover)
        } else {
            index
                .map(Message::ShowExternalSkin)
                .unwrap_or(Message::Unhover)
        };

        let preview: Element<'a, Message> = match &skin.preview {
            Some(p) => image(image::Handle::from_path(p.clone()))
                .width(Length::Fixed(96.0))
                .height(Length::Fixed(96.0))
                .content_fit(iced::ContentFit::Contain)
                .into(),
            None => Space::new()
                .width(Length::Fixed(96.0))
                .height(Length::Fixed(96.0))
                .into(),
        };

        let mut name_row = row![text(skin.name.clone())
            .size(14)
            .font(style::SEMIBOLD)
            .style(style::accent)]
        .spacing(style::XS)
        .align_y(iced::Alignment::Center);
        if active {
            name_row = name_row.push(chip("Active".into(), style::chip_success));
        }

        let mut clickable = column![
            container(preview).center_x(Length::Fill),
            container(name_row).center_x(Length::Fill),
        ]
        .spacing(style::XS)
        .width(Length::Fill);
        if !skin.author.is_empty() {
            clickable = clickable.push(
                container(
                    text(format!("by {}", skin.author))
                        .size(12)
                        .style(style::muted),
                )
                .center_x(Length::Fill),
            );
        }
        if !skin.desc.is_empty() {
            clickable = clickable.push(
                container(text(truncate(&skin.desc, 70)).size(11).style(style::muted))
                    .center_x(Length::Fill),
            );
        }
        if !skin.date_added.is_empty() {
            clickable = clickable.push(
                container(
                    text(format!("Added {}", skin.date_added))
                        .size(10)
                        .style(style::muted),
                )
                .center_x(Length::Fill),
            );
        }

        let body = column![
            row![Space::new().width(Length::Fill), corner],
            mouse_area(clickable).on_press(primary),
        ]
        .spacing(style::XS)
        .width(Length::Fill);

        // Fixed height (not `Fill`, which collapses to 0 inside a vertical scrollable)
        // gives every card the same footprint; `Fill` width keeps columns even.
        let card = container(body)
            .width(Length::Fill)
            .height(Length::Fixed(240.0))
            .padding(style::MD)
            .style(if active {
                style::card_active
            } else if hovered {
                style::card_hover
            } else {
                style::card
            });

        mouse_area(card)
            .on_enter(Message::Hover(HoverKey::SkinCard(skin.name.clone())))
            .on_exit(Message::Unhover)
            .into()
    }

    fn view_mod_detail(&self) -> Element<'_, Message> {
        let back = labeled_button(
            ICON_CHEVRON_LEFT,
            style::icon,
            "Back",
            style::secondary,
            Some(Message::Navigate(self.detail_return)),
        );

        let resolved = self
            .detail_mod
            .as_deref()
            .and_then(|n| self.catalog.as_ref().and_then(|c| c.get(n)));
        let Some(m) = resolved else {
            return column![
                back,
                card(text("This mod isn't in the loaded catalog.").size(14))
            ]
            .spacing(style::LG)
            .into();
        };

        let installed = self.is_installed(&m.name);

        // Primary action.
        let action: Element<'_, Message> = if installed {
            labeled_button(
                ICON_TRASH,
                style::icon,
                "Remove",
                style::danger,
                (!self.busy).then(|| Message::RemoveMod(m.name.clone())),
            )
            .into()
        } else {
            labeled_button(
                DOWNLOAD_MARK,
                style::icon_on_accent,
                "Install",
                style::primary,
                (!self.busy).then(|| Message::InstallMod(m.name.clone())),
            )
            .into()
        };
        let mut actions = row![action]
            .spacing(style::SM)
            .align_y(iced::Alignment::Center);
        if let Some(url) = &m.repository {
            actions = actions.push(icon_button(
                GITHUB_MARK,
                Some(Message::OpenUrl(url.clone())),
                "Open repository",
            ));
        }

        // Title block: name + meta + tags.
        let authors = if m.authors.is_empty() {
            String::new()
        } else {
            format!("  ·  by {}", m.authors.join(", "))
        };
        let mut chips =
            row![chip(format!("v{}", m.version), style::chip_neutral)].spacing(style::XS);
        if installed {
            chips = chips.push(chip("Installed".into(), style::chip_success));
        }
        for t in &m.tags {
            chips = chips.push(tag_chip(t));
        }
        let title_block = column![
            text(m.name.clone()).size(22).font(style::SEMIBOLD),
            text(format!("v{}{}", m.version, authors))
                .size(12)
                .style(style::muted),
            chips.wrap(),
        ]
        .spacing(style::SM)
        .width(Length::Fill);

        let head = card(
            column![
                row![title_block, actions]
                    .spacing(style::MD)
                    .align_y(iced::Alignment::Center),
                style::body(if m.description.is_empty() {
                    "No description provided.".to_string()
                } else {
                    m.description.clone()
                })
                .style(style::muted),
            ]
            .spacing(style::MD),
        );

        let mut body = column![back, head].spacing(style::LG);

        // Dependencies.
        if !m.dependencies.is_empty() {
            let mut list = column![style::section("Dependencies")].spacing(style::SM);
            for dep in &m.dependencies {
                let dep_installed = self.is_installed(dep);
                let badge = if dep_installed {
                    chip("Installed".into(), style::chip_success)
                } else {
                    chip("Not installed".into(), style::chip_neutral)
                };
                list = list.push(
                    row![text(dep.clone()).size(13).width(Length::Fill), badge]
                        .align_y(iced::Alignment::Center)
                        .spacing(style::SM),
                );
            }
            body = body.push(card(list));
        }

        // Integrations.
        if !m.integrations.is_empty() {
            let mut chips = row![].spacing(style::XS);
            for i in &m.integrations {
                chips = chips.push(tag_chip(i));
            }
            body = body.push(card(
                column![style::section("Integrates with"), chips.wrap()].spacing(style::SM),
            ));
        }

        screen_scroll(body)
    }

    fn view_settings(&self) -> Element<'_, Message> {
        let detected = match &self.install {
            Some(i) => i.root.display().to_string(),
            None => "Not set".into(),
        };
        let catalog_line = match &self.catalog {
            Some(c) => format!("{} mods loaded", c.len()),
            None => "Not loaded".into(),
        };

        let game_card = card(
            column![
                row![
                    svg_icon(ICON_HOME, 15.0, style::icon),
                    style::section("Game"),
                ]
                .spacing(style::SM)
                .align_y(iced::Alignment::Center),
                text(truncate_middle(&detected, 64))
                    .size(12)
                    .style(style::muted),
                row![labeled_button(
                    ICON_SEARCH,
                    style::icon,
                    "Detect via Steam",
                    style::secondary,
                    Some(Message::DetectSteam),
                ),]
                .spacing(style::SM),
                row![
                    text_input("Or enter the Hollow Knight folder…", &self.manual_path)
                        .on_input(Message::ManualPathChanged)
                        .on_submit(Message::SetManualPath)
                        .padding(style::pad(style::SM, style::MD))
                        .style(style::input)
                        .width(Length::Fill),
                    labeled_button(
                        ICON_FOLDER,
                        style::icon_on_accent,
                        "Set path",
                        style::primary,
                        Some(Message::SetManualPath),
                    ),
                ]
                .spacing(style::SM)
                .align_y(iced::Alignment::Center),
            ]
            .spacing(style::MD),
        );

        let catalog_card = card(
            row![
                column![
                    row![
                        svg_icon(ICON_COMPASS, 15.0, style::icon),
                        style::section("Mod catalog"),
                    ]
                    .spacing(style::SM)
                    .align_y(iced::Alignment::Center),
                    text(catalog_line).size(12).style(style::muted),
                ]
                .spacing(style::XS)
                .width(Length::Fill),
                labeled_button(
                    ICON_REFRESH,
                    style::icon,
                    "Refresh",
                    style::secondary,
                    (!self.busy).then_some(Message::RefreshCatalog),
                ),
            ]
            .align_y(iced::Alignment::Center)
            .spacing(style::MD),
        );

        let presets = theme::preset_names();
        let selected = Some(self.config.theme.preset.clone());
        let accent = self.config.theme.accent.clone().unwrap_or_default();

        // Preset accent swatches.
        let mut swatches = row![].spacing(style::SM).align_y(iced::Alignment::Center);
        for (hex, r, g, b) in ACCENTS {
            swatches = swatches.push(
                button(
                    container(Space::new())
                        .width(Length::Fixed(18.0))
                        .height(Length::Fixed(18.0))
                        .style(style::swatch(iced::Color::from_rgb8(r, g, b))),
                )
                .style(style::ghost)
                .padding(2.0)
                .on_press(Message::AccentChanged(hex.to_string())),
            );
        }
        swatches = swatches.push(
            button(text("Default").size(12))
                .style(style::ghost)
                .on_press(Message::AccentChanged(String::new())),
        );

        let appearance_card = card(
            column![
                row![
                    svg_icon(ICON_GEAR, 15.0, style::icon),
                    style::section("Appearance"),
                ]
                .spacing(style::SM)
                .align_y(iced::Alignment::Center),
                row![
                    text("Theme").width(Length::Fixed(110.0)),
                    pick_list(presets, selected, Message::ThemePresetChanged),
                ]
                .spacing(style::MD)
                .align_y(iced::Alignment::Center),
                row![
                    text("Accent").width(Length::Fixed(110.0)),
                    container(Space::new())
                        .width(Length::Fixed(18.0))
                        .height(Length::Fixed(18.0))
                        .style(style::accent_swatch),
                    text_input("#RRGGBB", &accent)
                        .on_input(Message::AccentChanged)
                        .padding(style::pad(style::SM, style::MD))
                        .style(style::input)
                        .width(Length::Fixed(160.0)),
                ]
                .spacing(style::SM)
                .align_y(iced::Alignment::Center),
                row![text("").width(Length::Fixed(110.0)), swatches]
                    .spacing(style::MD)
                    .align_y(iced::Alignment::Center),
            ]
            .spacing(style::MD),
        );

        let about = row![
            text(format!("aspid v{}", env!("CARGO_PKG_VERSION")))
                .size(12)
                .style(style::muted)
                .width(Length::Fill),
            icon_button(
                GITHUB_MARK,
                Some(Message::OpenUrl("https://github.com/marlstar/aspid".into())),
                "Project on GitHub",
            ),
        ]
        .align_y(iced::Alignment::Center);

        column![
            header("Settings", None, None),
            screen_scroll(
                column![game_card, catalog_card, appearance_card, about].spacing(style::LG)
            ),
        ]
        .spacing(style::LG)
        .into()
    }
}

/// Preset accent colours offered in Settings (hex string + RGB).
const ACCENTS: [(&str, u8, u8, u8); 7] = [
    ("#E0652E", 0xE0, 0x65, 0x2E),
    ("#4D9DFF", 0x4D, 0x9D, 0xFF),
    ("#1BD96A", 0x1B, 0xD9, 0x6A),
    ("#36C5A8", 0x36, 0xC5, 0xA8),
    ("#A06CFF", 0xA0, 0x6C, 0xFF),
    ("#F5A623", 0xF5, 0xA6, 0x23),
    ("#E5534B", 0xE5, 0x53, 0x4B),
];

/// A standard page header: large title, optional subtitle, optional right-aligned actions.
fn header<'a>(
    title: &'a str,
    subtitle: Option<String>,
    actions: Option<Element<'a, Message>>,
) -> Element<'a, Message> {
    let mut titles = column![style::title(title)].spacing(2.0);
    if let Some(s) = subtitle {
        titles = titles.push(text(s).size(13).style(style::muted));
    }
    let mut bar = row![container(titles).width(Length::Fill)]
        .align_y(iced::Alignment::Center)
        .spacing(style::MD);
    if let Some(a) = actions {
        bar = bar.push(a);
    }
    bar.into()
}

/// Wrap content in a standard card surface.
fn card<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(content)
        .padding(style::LG)
        .width(Length::Fill)
        .style(style::card)
        .into()
}

/// A card whose surface lifts on hover.
fn card_h<'a>(content: impl Into<Element<'a, Message>>, hovered: bool) -> Element<'a, Message> {
    container(content)
        .padding(style::LG)
        .width(Length::Fill)
        .style(if hovered {
            style::card_hover
        } else {
            style::card
        })
        .into()
}

/// A small rounded chip/badge.
fn chip<'a>(
    label: String,
    sty: fn(&Theme) -> iced::widget::container::Style,
) -> Element<'a, Message> {
    container(text(label).size(11).font(style::MEDIUM))
        .padding(style::pad(2.0, 8.0))
        .style(sty)
        .into()
}

/// A tag chip coloured by its label.
fn tag_chip<'a>(label: &str) -> Element<'a, Message> {
    container(text(label.to_string()).size(11).font(style::MEDIUM))
        .padding(style::pad(2.0, 8.0))
        .style(style::tag(label))
        .into()
}

/// A monochrome SVG icon, tinted via `sty`.
fn svg_icon<'a>(
    mark: &'static [u8],
    size: f32,
    sty: fn(&Theme, svg::Status) -> svg::Style,
) -> Element<'a, Message> {
    svg(svg::Handle::from_memory(mark))
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .style(sty)
        .into()
}

/// An icon-only ghost button with a tooltip.
fn icon_button<'a>(
    mark: &'static [u8],
    msg: Option<Message>,
    tip: &'a str,
) -> Element<'a, Message> {
    let btn = button(svg_icon(mark, 16.0, style::icon))
        .style(style::ghost)
        .padding(style::XS)
        .on_press_maybe(msg);
    tooltip(
        btn,
        container(text(tip).size(12))
            .padding(style::pad(style::XXS, style::SM))
            .style(style::card),
        tooltip::Position::Top,
    )
    .gap(6.0)
    .into()
}

/// A button with a leading icon and a label.
fn labeled_button<'a>(
    mark: &'static [u8],
    icon_style: fn(&Theme, svg::Status) -> svg::Style,
    label: &'a str,
    btn_style: fn(&Theme, button::Status) -> button::Style,
    msg: Option<Message>,
) -> iced::widget::Button<'a, Message> {
    button(
        row![svg_icon(mark, 15.0, icon_style), text(label)]
            .spacing(style::XS)
            .align_y(iced::Alignment::Center),
    )
    .style(btn_style)
    .padding(style::pad(style::SM, style::MD))
    .on_press_maybe(msg)
}

/// Wrap a screen body in a styled, gutter-padded vertical scrollable.
fn screen_scroll<'a>(body: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    scrollable(container(body).padding(iced::Padding {
        right: style::SCROLL_GUTTER,
        ..iced::Padding::ZERO
    }))
    .direction(scrollable::Direction::Vertical(
        scrollable::Scrollbar::new()
            .width(8.0)
            .margin(2.0)
            .scroller_width(6.0),
    ))
    .style(style::scrollbar)
    .height(Length::Fill)
    .into()
}

/// A small stat pill: an icon and a label on a neutral surface.
fn stat<'a>(mark: &'static [u8], label: String) -> Element<'a, Message> {
    container(
        row![
            svg_icon(mark, 14.0, style::icon),
            text(label).size(12).style(style::muted),
        ]
        .spacing(style::XS)
        .align_y(iced::Alignment::Center),
    )
    .padding(style::pad(style::XS, style::MD))
    .style(style::surface)
    .into()
}

/// Truncate a long string in the middle (keeps the start and end), for paths.
fn truncate_middle(s: &str, max: usize) -> String {
    let n = s.chars().count();
    if n <= max {
        return s.to_string();
    }
    let keep = max.saturating_sub(1) / 2;
    let start: String = s.chars().take(keep).collect();
    let end: String = s.chars().skip(n - keep).collect();
    format!("{start}…{end}")
}

/// The sidebar icon for a screen.
fn screen_icon(screen: Screen) -> &'static [u8] {
    match screen {
        Screen::Dashboard => ICON_HOME,
        Screen::Browse => ICON_COMPASS,
        Screen::Installed => ICON_PACKAGE,
        Screen::Modpacks => ICON_LAYERS,
        Screen::Skins => ICON_SHIRT,
        Screen::Settings => ICON_GEAR,
        Screen::ModDetail => ICON_PACKAGE,
    }
}

/// Resolve a skin-kind id back to its [`SkinKind`].
fn kind_by_id(id: &str) -> Option<skins::SkinKind> {
    skins::ALL_KINDS.iter().copied().find(|k| k.id == id)
}

fn truncate(s: &str, max: usize) -> String {
    let one_line = s.replace('\n', " ");
    if one_line.chars().count() <= max {
        one_line
    } else {
        let cut: String = one_line.chars().take(max).collect();
        format!("{cut}…")
    }
}

// ---- Async bridges to aspid-core ---------------------------------------------

async fn load_catalog(config: Config, force: bool) -> Result<Catalog, String> {
    modlinks::fetch_catalog(&config, force)
        .await
        .map_err(|e| e.to_string())
}

async fn load_api(config: Config, force: bool) -> Result<ApiManifest, String> {
    modlinks::fetch_api_manifest(&config, force)
        .await
        .map_err(|e| e.to_string())
}

async fn do_install(install: Install, catalog: Catalog, name: String) -> Result<String, String> {
    mods::install_with_dependencies(&install, &catalog, &name)
        .await
        .map(|installed| match installed.len() {
            0 => format!("{name} is already up to date"),
            1 => format!("Installed {name}"),
            n => format!("Installed {name} (+{} dependencies)", n - 1),
        })
        .map_err(|e| e.to_string())
}

async fn load_skin_catalog(url: String, force: bool) -> Result<Vec<HkSkin>, String> {
    skins::fetch_catalog(&url, force)
        .await
        .map_err(|e| e.to_string())
}

async fn download_skin(store: SkinStore, skin: HkSkin) -> Result<String, String> {
    skins::download_into(&store, &skin)
        .await
        .map(|name| format!("Downloaded skin “{name}” to your library"))
        .map_err(|e| e.to_string())
}

/// Open a native file picker for a downloaded skin archive and import it into the library.
async fn import_skin_file(store: SkinStore) -> Result<String, String> {
    let Some(file) = rfd::AsyncFileDialog::new()
        .set_title("Select a downloaded skin (.zip)")
        .add_filter("Skin archive", &["zip"])
        .pick_file()
        .await
    else {
        return Ok("Skin import cancelled".to_string());
    };
    let bytes = file.read().await;
    let raw_name = file.file_name();
    let fallback = raw_name.strip_suffix(".zip").unwrap_or(&raw_name);
    skins::SkinStore::import_zip(&store, skins::CUSTOM_KNIGHT, &bytes, fallback)
        .map(|name| format!("Imported skin “{name}” to your library"))
        .map_err(|e| e.to_string())
}

async fn upload_share(share: PackShare) -> Result<String, String> {
    aspid_core::share::upload(&share)
        .await
        .map_err(|e| e.to_string())
}

async fn resolve_share(code: String) -> Result<PackShare, String> {
    aspid_core::share::resolve(&code)
        .await
        .map_err(|e| e.to_string())
}

async fn do_import(
    install: Install,
    catalog: Catalog,
    names: Vec<String>,
) -> Result<String, String> {
    let mut installed = 0usize;
    let mut missing = Vec::new();
    for name in names {
        if catalog.get(&name).is_none() {
            missing.push(name);
            continue;
        }
        mods::install_with_dependencies(&install, &catalog, &name)
            .await
            .map_err(|e| format!("Failed installing {name}: {e}"))?;
        installed += 1;
    }
    let mut msg = format!("Imported {installed} mod(s)");
    if !missing.is_empty() {
        msg += &format!("; {} not in catalog: {}", missing.len(), missing.join(", "));
    }
    Ok(msg)
}

async fn do_install_api(install: Install, manifest: ApiManifest) -> Result<String, String> {
    modapi::install(&install, &manifest)
        .await
        .map(|v| format!("Modding API v{v} installed"))
        .map_err(|e| e.to_string())
}

/// Remove a mod, warning (in the status message) if other installed mods depend on it.
fn remove_with_warning(
    install: &Install,
    catalog: Option<&Catalog>,
    name: &str,
) -> Result<String, String> {
    let dependents = catalog
        .and_then(|c| mods::installed_dependents(install, c, name).ok())
        .unwrap_or_default();
    mods::remove(install, name).map_err(|e| e.to_string())?;
    if dependents.is_empty() {
        Ok(format!("Removed {name}"))
    } else {
        Ok(format!(
            "Removed {name} — warning: still required by {}",
            dependents.join(", ")
        ))
    }
}
