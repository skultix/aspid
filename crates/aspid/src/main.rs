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
    text_input, Space,
};

/// The GitHub mark, rendered (and tinted) next to a mod's actions to open its homepage.
const GITHUB_MARK: &[u8] = br##"<svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg"><path d="M12 .297c-6.63 0-12 5.373-12 12 0 5.303 3.438 9.8 8.205 11.385.6.113.82-.258.82-.577 0-.285-.01-1.04-.015-2.04-3.338.724-4.042-1.61-4.042-1.61C4.422 18.07 3.633 17.7 3.633 17.7c-1.087-.744.084-.729.084-.729 1.205.084 1.838 1.236 1.838 1.236 1.07 1.835 2.809 1.305 3.495.998.108-.776.417-1.305.76-1.605-2.665-.3-5.466-1.332-5.466-5.93 0-1.31.465-2.38 1.235-3.22-.135-.303-.54-1.523.105-3.176 0 0 1.005-.322 3.3 1.23.96-.267 1.98-.399 3-.405 1.02.006 2.04.138 3 .405 2.28-1.552 3.285-1.23 3.285-1.23.645 1.653.24 2.873.12 3.176.765.84 1.23 1.91 1.23 3.22 0 4.61-2.805 5.625-5.475 5.92.42.36.81 1.096.81 2.22 0 1.606-.015 2.896-.015 3.286 0 .315.21.69.825.57C20.565 22.092 24 17.592 24 12.297c0-6.627-5.373-12-12-12"/></svg>"##;

/// A generic "external link" icon, shown top-right on externally-hosted skin cards.
const LINK_MARK: &[u8] = br##"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" xmlns="http://www.w3.org/2000/svg"><path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"/><path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"/></svg>"##;

/// A download icon, shown on skin cards aspid can fetch automatically.
const DOWNLOAD_MARK: &[u8] = br##"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" xmlns="http://www.w3.org/2000/svg"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>"##;

/// A check/tick icon, shown on skins already in the library.
const CHECK_MARK: &[u8] = br##"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round" xmlns="http://www.w3.org/2000/svg"><polyline points="20 6 9 17 4 12"/></svg>"##;
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
        .window_size(iced::Size::new(1040.0, 720.0))
        .centered()
        .run()
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
    skin_store: Option<SkinStore>,
    skin_catalog: Option<Vec<HkSkin>>,
    skin_search: String,
    /// Catalog index of the external skin whose "how to install" popup is open.
    skin_modal: Option<usize>,
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
    OpenUrl(String),
    InstallMod(String),
    RemoveMod(String),
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
            skin_store: SkinStore::open().ok(),
            skin_catalog: None,
            skin_search: String::new(),
            skin_modal: None,
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
        };

        let body = column![
            container(content)
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(style::XL),
            self.status_bar(),
        ]
        .width(Length::Fill);

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
            text("Hosted externally").size(18),
            text(format!(
                "“{}” is hosted on another site. Open its page to download the skin, \
                 then import the downloaded file into your library.",
                skin.name
            ))
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
        button(text(screen.label()).size(14))
            .width(Length::Fill)
            .padding(style::pad(style::SM, style::MD))
            .style(style::nav(active))
            .on_press(Message::Navigate(screen))
            .into()
    }

    fn sidebar(&self) -> Element<'_, Message> {
        let brand = container(text("aspid").size(26).style(style::accent))
            .padding(style::pad(style::MD, style::MD));

        let mut nav = column![]
            .spacing(style::XS)
            .padding(style::pad(0.0, style::SM));
        for screen in Screen::ALL {
            if screen == Screen::Settings {
                continue; // pinned to the footer
            }
            nav = nav.push(self.nav_item(screen));
        }

        // Footer: a compact status summary plus Settings.
        let summary = match &self.install {
            None => "No game set".to_string(),
            Some(_) => match self.modpacks.as_ref().and_then(|m| m.active()) {
                Some(active) => {
                    let name = self
                        .modpacks
                        .as_ref()
                        .and_then(|m| m.packs().iter().find(|p| p.id == active))
                        .map(|p| p.name.clone())
                        .unwrap_or_else(|| active.to_string());
                    format!("Pack: {name}")
                }
                None => "Game ready".to_string(),
            },
        };
        let footer = column![
            container(text(summary).size(12).style(style::muted))
                .padding(style::pad(style::SM, style::MD)),
            self.nav_item(Screen::Settings),
        ]
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
        let dot = if self.busy { "●  " } else { "" };
        let styled = text(format!("{dot}{label}")).size(13).style(if self.busy {
            style::accent
        } else {
            style::muted
        });
        container(styled)
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
            ApiState::DisabledForVanilla => chip("Modded · running vanilla".into(), style::chip),
            ApiState::NotInstalled => chip("Vanilla".into(), style::chip_neutral),
            ApiState::Missing => chip("Broken install".into(), style::chip_neutral),
        };

        let hero = container(
            column![
                row![
                    column![
                        text("Hollow Knight").size(20),
                        text(install.root.display().to_string())
                            .size(12)
                            .style(style::muted),
                    ]
                    .spacing(style::XS)
                    .width(Length::Fill),
                    status_chip,
                ]
                .align_y(iced::Alignment::Center)
                .spacing(style::MD),
                row![
                    button(text("▶  Launch modded"))
                        .style(style::primary)
                        .padding(style::pad(style::SM, style::LG))
                        .on_press_maybe(launch_enabled.then_some(Message::LaunchModded)),
                    button(text("Launch vanilla"))
                        .style(style::secondary)
                        .padding(style::pad(style::SM, style::LG))
                        .on_press_maybe(launch_enabled.then_some(Message::LaunchVanilla)),
                ]
                .spacing(style::SM),
            ]
            .spacing(style::LG),
        )
        .padding(style::XL)
        .width(Length::Fill)
        .style(style::hero);

        let api_button_label = if !state.is_installed() {
            "Install modding API"
        } else if api_update {
            "Update modding API"
        } else {
            "Reinstall API"
        };
        let api_version = modapi::installed_version(install)
            .map(|v| format!("Installed: v{v}"))
            .unwrap_or_else(|| "Not installed".to_string());
        let api_card = card(
            row![
                column![
                    text("Modding API").size(16),
                    text(api_version).size(12).style(style::muted),
                ]
                .spacing(style::XS)
                .width(Length::Fill),
                button(text(api_button_label))
                    .style(if api_update {
                        style::primary
                    } else {
                        style::secondary
                    })
                    .on_press_maybe(
                        (!self.busy && self.api_manifest.is_some())
                            .then_some(Message::InstallOrUpdateApi)
                    ),
            ]
            .align_y(iced::Alignment::Center)
            .spacing(style::MD),
        );

        let pack_name = self
            .modpacks
            .as_ref()
            .and_then(|m| m.active())
            .map(|a| a.to_string());
        let mut stats = row![chip(
            format!("{} mods installed", self.installed.len()),
            style::chip_neutral
        )]
        .spacing(style::SM);
        if let Some(pack) = pack_name {
            stats = stats.push(chip(format!("Pack: {pack}"), style::chip_neutral));
        }

        column![
            header(
                "Dashboard",
                Some("Launch and manage your game.".into()),
                None
            ),
            hero,
            api_card,
            stats,
        ]
        .spacing(style::LG)
        .into()
    }

    fn view_browse(&self) -> Element<'_, Message> {
        let search: Element<'_, Message> = text_input("Search mods…", &self.search)
            .on_input(Message::SearchChanged)
            .padding(style::SM)
            .width(Length::Fixed(260.0))
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

        // Cap how many cards we build at once: constructing hundreds of cards per frame is
        // what caused the hitch. With a search this is rarely hit; without one, the cap
        // keeps navigation snappy and the user narrows down via the search box.
        const CAP: usize = 150;
        let query = self.search.to_lowercase();
        let matches: Vec<&Mod> = catalog
            .mods()
            .iter()
            .filter(|m| {
                query.is_empty()
                    || m.name.to_lowercase().contains(&query)
                    || m.description.to_lowercase().contains(&query)
            })
            .collect();

        let mut col = column![].spacing(style::SM);
        for m in matches.iter().take(CAP) {
            col = col.push(self.mod_row(m));
        }

        let subtitle = if matches.len() > CAP {
            format!(
                "showing {CAP} of {} matches — search to narrow",
                matches.len()
            )
        } else {
            format!("{} of {} mods", matches.len(), catalog.len())
        };

        column![
            header("Browse", Some(subtitle), Some(search)),
            scrollable(col).height(Length::Fill),
        ]
        .spacing(style::LG)
        .into()
    }

    fn mod_row<'a>(&'a self, m: &'a Mod) -> Element<'a, Message> {
        let installed = self.is_installed(&m.name);
        let action: Element<'a, Message> = if installed {
            button(text("Remove"))
                .style(style::danger)
                .on_press_maybe((!self.busy).then(|| Message::RemoveMod(m.name.clone())))
                .into()
        } else {
            button(text("Install"))
                .style(style::primary)
                .on_press_maybe((!self.busy).then(|| Message::InstallMod(m.name.clone())))
                .into()
        };

        let mut actions = row![].spacing(style::SM).align_y(iced::Alignment::Center);
        if let Some(url) = &m.repository {
            let icon = svg(svg::Handle::from_memory(GITHUB_MARK))
                .width(Length::Fixed(16.0))
                .height(Length::Fixed(16.0))
                .style(style::icon);
            actions = actions.push(
                button(icon)
                    .padding(style::SM)
                    .style(style::secondary)
                    .on_press(Message::OpenUrl(url.clone())),
            );
        }
        actions = actions.push(action);

        let mut chips =
            row![chip(format!("v{}", m.version), style::chip_neutral)].spacing(style::XS);
        if installed {
            chips = chips.push(chip("Installed".into(), style::chip_success));
        }
        for tag in m.tags.iter().take(2) {
            chips = chips.push(chip(tag.clone(), style::chip_neutral));
        }

        let info = column![
            text(&m.name).size(15),
            text(truncate(&m.description, 100))
                .size(12)
                .style(style::muted),
            chips,
        ]
        .spacing(style::XS)
        .width(Length::Fill);

        card(
            row![info, actions]
                .spacing(style::MD)
                .align_y(iced::Alignment::Center),
        )
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

            let version = m.version.clone().unwrap_or_else(|| "?".into());
            let mut chips =
                row![chip(format!("v{version}"), style::chip_neutral)].spacing(style::XS);
            if m.enabled {
                chips = chips.push(chip("Enabled".into(), style::chip_success));
            } else {
                chips = chips.push(chip("Disabled".into(), style::chip_neutral));
            }
            if update {
                chips = chips.push(chip("Update available".into(), style::chip));
            }

            let toggle_label = if m.enabled { "Disable" } else { "Enable" };
            let enabled = m.enabled;
            let name = m.name.clone();
            let name2 = m.name.clone();

            let info = column![text(&m.name).size(15), chips]
                .spacing(style::XS)
                .width(Length::Fill);
            let actions = row![
                button(text(toggle_label))
                    .style(style::secondary)
                    .on_press_maybe((!self.busy).then_some(Message::SetModEnabled(name, !enabled))),
                button(text("Remove"))
                    .style(style::danger)
                    .on_press_maybe((!self.busy).then_some(Message::RemoveMod(name2))),
            ]
            .spacing(style::SM);

            list = list.push(card(
                row![info, actions]
                    .spacing(style::MD)
                    .align_y(iced::Alignment::Center),
            ));
        }

        let subtitle = format!("{} installed", self.installed.len());
        column![
            header("Installed", Some(subtitle), None),
            scrollable(list).height(Length::Fill)
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
            let id_act = pack.id.clone();
            let id_clone = pack.id.clone();
            let id_del = pack.id.clone();
            let id_share = pack.id.clone();
            let deletable = !is_active && pack.id != modpack::VANILLA_ID;

            let mut title_row = row![text(&pack.name).size(15)]
                .spacing(style::SM)
                .align_y(iced::Alignment::Center);
            if is_active {
                title_row = title_row.push(chip("Active".into(), style::chip_success));
            }

            let actions = row![
                button(text("Activate"))
                    .style(style::primary)
                    .on_press_maybe(
                        (!self.busy && !is_active).then_some(Message::ActivatePack(id_act))
                    ),
                button(text("Share"))
                    .style(style::secondary)
                    .on_press_maybe((!self.busy).then_some(Message::ExportPack(id_share))),
                button(text("Clone"))
                    .style(style::secondary)
                    .on_press_maybe((!self.busy).then_some(Message::ClonePack(id_clone))),
                button(text("Delete")).style(style::danger).on_press_maybe(
                    (!self.busy && deletable).then_some(Message::DeletePack(id_del))
                ),
            ]
            .spacing(style::SM);

            let body = row![title_row.width(Length::Fill), actions]
                .spacing(style::MD)
                .align_y(iced::Alignment::Center);

            // Highlight the active pack with the accent (hero) surface.
            let row_card = if is_active {
                container(body)
                    .padding(style::LG)
                    .width(Length::Fill)
                    .style(style::hero)
                    .into()
            } else {
                card(body)
            };
            list = list.push(row_card);
        }

        let create: Element<'_, Message> = row![
            text_input("New pack name…", &self.new_pack_name)
                .on_input(Message::NewPackNameChanged)
                .on_submit(Message::CreatePack)
                .padding(style::SM)
                .width(Length::Fixed(200.0)),
            button(text("Create"))
                .style(style::primary)
                .on_press_maybe((!self.busy).then_some(Message::CreatePack)),
        ]
        .spacing(style::SM)
        .into();

        // Import / share card.
        let mut share_card = column![
            text("Share & import").size(16),
            text("Share a pack to copy its mod list as a code. Paste one here to recreate it.")
                .size(12)
                .style(style::muted),
            row![
                text_input("Paste a modpack code…", &self.import_code)
                    .on_input(Message::ImportCodeChanged)
                    .on_submit(Message::ImportPack)
                    .padding(style::SM)
                    .width(Length::Fill),
                button(text("Import"))
                    .style(style::primary)
                    .on_press_maybe((!self.busy).then_some(Message::ImportPack)),
            ]
            .spacing(style::SM)
            .align_y(iced::Alignment::Center),
        ]
        .spacing(style::SM);

        if let Some(code) = &self.share_code {
            share_card = share_card.push(
                text("Exported code (copied to clipboard):")
                    .size(12)
                    .style(style::muted),
            );
            share_card = share_card.push(
                text_input("", code)
                    .on_input(Message::ShareCodeChanged)
                    .padding(style::SM),
            );
        }

        column![
            header("Modpacks", None, Some(create)),
            scrollable(list).height(Length::Fill),
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

        for kind in skins::ALL_KINDS {
            let installed = self
                .install
                .as_ref()
                .map(|i| skins::is_mod_installed(i, kind))
                .unwrap_or(false);

            let status_chip = if installed {
                chip("Installed".into(), style::chip_success)
            } else {
                chip("Mod not installed".into(), style::chip_neutral)
            };
            let head = row![
                text(kind.label).size(16),
                status_chip,
                container(text("")).width(Length::Fill),
                button(text("Sync to game"))
                    .style(style::secondary)
                    .on_press_maybe(
                        (!self.busy && installed).then_some(Message::SyncSkins(kind.id))
                    ),
            ]
            .spacing(style::SM)
            .align_y(iced::Alignment::Center);

            let mut section = column![head].spacing(style::SM);

            let library = store.list(kind).unwrap_or_default();
            if library.is_empty() {
                section = section.push(
                    text("No skins in your library yet.")
                        .size(12)
                        .style(style::muted),
                );
            } else {
                let active = self.config.active_skins.get(kind.id);
                for name in library {
                    let is_active = active == Some(&name);
                    let set_name = name.clone();
                    let rm_name = name.clone();
                    let mut label_row = row![text(name.clone()).size(14)]
                        .spacing(style::SM)
                        .align_y(iced::Alignment::Center);
                    if is_active {
                        label_row = label_row.push(chip("Active".into(), style::chip_success));
                    }
                    let row = row![
                        label_row.width(Length::Fill),
                        button(text("Set active"))
                            .style(style::secondary)
                            .on_press_maybe(
                                (!is_active).then_some(Message::SetActiveSkin(kind.id, set_name))
                            ),
                        button(text("Remove")).style(style::danger).on_press_maybe(
                            (!self.busy).then_some(Message::RemoveSkin(kind.id, rm_name))
                        ),
                    ]
                    .spacing(style::SM)
                    .align_y(iced::Alignment::Center);
                    section = section.push(row);
                }
            }

            col = col.push(card(section));
        }

        // Catalog card (HKSkins).
        let header_row = row![
            text("Skin catalog").size(16).width(Length::Fill),
            button(text("Import skin file…"))
                .style(style::secondary)
                .on_press_maybe((!self.busy).then_some(Message::ImportSkinFile)),
            button(text(if self.skin_catalog.is_some() {
                "Reload"
            } else {
                "Browse hkskins.art"
            }))
            .style(style::primary)
            .on_press_maybe((!self.busy).then_some(Message::LoadSkinCatalog)),
        ]
        .spacing(style::SM)
        .align_y(iced::Alignment::Center);

        // Controls card: title/actions, plus a search box + count once loaded.
        let mut controls = column![header_row].spacing(style::SM);
        const CAP: usize = 90;
        let q = self.skin_search.to_lowercase();
        let pass = |s: &HkSkin| {
            q.is_empty()
                || s.name.to_lowercase().contains(&q)
                || s.author.to_lowercase().contains(&q)
        };
        match &self.skin_catalog {
            None => {
                controls = controls.push(
                    text(
                        "Browse 600+ community skins from hkskins.art. Most are hosted \
                         externally: open a skin to download it, then use “Import skin \
                         file…” to add the downloaded .zip to your library.",
                    )
                    .size(12)
                    .style(style::muted),
                );
            }
            Some(skins) => {
                let total = skins.iter().filter(|s| pass(s)).count();
                controls = controls.push(
                    text_input("Search skins…", &self.skin_search)
                        .on_input(Message::SkinSearchChanged)
                        .padding(style::SM),
                );
                controls = controls.push(
                    text(if total > CAP {
                        format!("showing {CAP} of {total} — search to narrow")
                    } else {
                        format!("{total} skins")
                    })
                    .size(12)
                    .style(style::muted),
                );
            }
        }
        col = col.push(card(controls));

        // Card grid of skins.
        if let Some(skins) = &self.skin_catalog {
            const COLS: usize = 3;
            // Library names (lower-cased) to mark already-installed skins with a tick.
            let installed: std::collections::HashSet<String> = self
                .skin_store
                .as_ref()
                .and_then(|s| s.list(skins::CUSTOM_KNIGHT).ok())
                .unwrap_or_default()
                .into_iter()
                .map(|n| n.to_lowercase())
                .collect();

            let items: Vec<(usize, &HkSkin)> = skins
                .iter()
                .enumerate()
                .filter(|(_, s)| pass(s))
                .take(CAP)
                .collect();
            let mut grid = column![].spacing(style::MD);
            for chunk in items.chunks(COLS) {
                let mut r = row![].spacing(style::MD);
                for (i, skin) in chunk {
                    r = r.push(self.skin_card(*i, skin, &installed));
                }
                grid = grid.push(r);
            }
            col = col.push(grid);
        }

        scrollable(col).height(Length::Fill).into()
    }

    /// A single skin card: a state icon (top-right), preview image, name, author, and
    /// components. `installed` is the set of library skin names (lower-cased).
    fn skin_card<'a>(
        &'a self,
        index: usize,
        skin: &'a HkSkin,
        installed: &std::collections::HashSet<String>,
    ) -> Element<'a, Message> {
        fn icon(
            mark: &'static [u8],
            sty: fn(&Theme, svg::Status) -> svg::Style,
        ) -> svg::Svg<'static, Theme> {
            svg(svg::Handle::from_memory(mark))
                .width(Length::Fixed(16.0))
                .height(Length::Fixed(16.0))
                .style(sty)
        }

        // Top-right state icon: installed → green tick; auto-downloadable → download;
        // otherwise → external-link button that opens the "how to install" popup.
        let is_installed = installed.contains(&skin.name.to_lowercase());
        let action: Element<'a, Message> = if is_installed {
            container(icon(CHECK_MARK, style::icon_success))
                .padding(style::XS)
                .into()
        } else if skin.is_auto_downloadable() {
            button(icon(DOWNLOAD_MARK, style::icon))
                .style(style::ghost)
                .padding(style::XS)
                .on_press_maybe((!self.busy).then_some(Message::DownloadSkin(index)))
                .into()
        } else {
            button(icon(LINK_MARK, style::icon))
                .style(style::ghost)
                .padding(style::XS)
                .on_press(Message::ShowExternalSkin(index))
                .into()
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

        let mut body = column![
            row![Space::new().width(Length::Fill), action],
            container(preview).center_x(Length::Fill),
            container(text(skin.name.clone()).size(14).style(style::accent)).center_x(Length::Fill),
        ]
        .spacing(style::XS)
        .width(Length::Fill);

        if !skin.author.is_empty() {
            body = body.push(
                container(
                    text(format!("by {}", skin.author))
                        .size(12)
                        .style(style::muted),
                )
                .center_x(Length::Fill),
            );
        }
        if !skin.desc.is_empty() {
            body = body.push(
                container(text(skin.desc.clone()).size(11).style(style::muted))
                    .center_x(Length::Fill),
            );
        }
        if !skin.date_added.is_empty() {
            body = body.push(
                container(
                    text(format!("Added {}", skin.date_added))
                        .size(10)
                        .style(style::muted),
                )
                .center_x(Length::Fill),
            );
        }

        container(body)
            .width(Length::Fixed(228.0))
            .padding(style::MD)
            .style(style::card)
            .into()
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
                text("Game").size(16),
                text(detected).size(12).style(style::muted),
                row![button(text("Detect via Steam"))
                    .style(style::secondary)
                    .on_press(Message::DetectSteam),]
                .spacing(style::SM),
                row![
                    text_input("Or enter the Hollow Knight folder…", &self.manual_path)
                        .on_input(Message::ManualPathChanged)
                        .on_submit(Message::SetManualPath)
                        .padding(style::SM)
                        .width(Length::Fill),
                    button(text("Set path"))
                        .style(style::primary)
                        .on_press(Message::SetManualPath),
                ]
                .spacing(style::SM)
                .align_y(iced::Alignment::Center),
            ]
            .spacing(style::MD),
        );

        let catalog_card = card(
            row![
                column![
                    text("Mod catalog").size(16),
                    text(catalog_line).size(12).style(style::muted),
                ]
                .spacing(style::XS)
                .width(Length::Fill),
                button(text("Refresh"))
                    .style(style::secondary)
                    .on_press_maybe((!self.busy).then_some(Message::RefreshCatalog)),
            ]
            .align_y(iced::Alignment::Center)
            .spacing(style::MD),
        );

        let presets = theme::preset_names();
        let selected = Some(self.config.theme.preset.clone());
        let accent = self.config.theme.accent.clone().unwrap_or_default();
        let appearance_card = card(
            column![
                text("Appearance").size(16),
                row![
                    text("Theme").width(Length::Fixed(120.0)),
                    pick_list(presets, selected, Message::ThemePresetChanged),
                ]
                .spacing(style::MD)
                .align_y(iced::Alignment::Center),
                row![
                    text("Accent").width(Length::Fixed(120.0)),
                    text_input("#RRGGBB (blank = preset default)", &accent)
                        .on_input(Message::AccentChanged)
                        .padding(style::SM)
                        .width(Length::Fixed(280.0)),
                ]
                .spacing(style::MD)
                .align_y(iced::Alignment::Center),
            ]
            .spacing(style::MD),
        );

        scrollable(
            column![
                header("Settings", None, None),
                game_card,
                catalog_card,
                appearance_card,
            ]
            .spacing(style::LG),
        )
        .into()
    }
}

/// A standard page header: large title, optional subtitle, optional right-aligned actions.
fn header<'a>(
    title: &'a str,
    subtitle: Option<String>,
    actions: Option<Element<'a, Message>>,
) -> Element<'a, Message> {
    let mut titles = column![text(title).size(26)].spacing(2);
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

/// A small rounded chip/badge.
fn chip<'a>(
    label: String,
    sty: fn(&Theme) -> iced::widget::container::Style,
) -> Element<'a, Message> {
    container(text(label).size(11))
        .padding([3, 8])
        .style(sty)
        .into()
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
