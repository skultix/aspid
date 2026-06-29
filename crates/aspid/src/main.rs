//! aspid — a cross-platform Hollow Knight mod manager (Iced front-end).

mod theme;

use aspid_core::config::Config;
use aspid_core::game::{self, ApiState, Install};
use aspid_core::modlinks::{ApiManifest, Catalog, Mod};
use aspid_core::mods::{self, InstalledMod};
use aspid_core::{launch, modapi, modlinks, modpack};

use iced::widget::{button, column, container, row, scrollable, text, text_input, Space};
use iced::{Element, Length, Task, Theme};

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
    search: String,
    status: String,
    busy: bool,
}

#[derive(Debug, Clone)]
enum Message {
    Navigate(Screen),
    DetectSteam,
    SteamDetected(Result<Install, String>),
    RefreshCatalog,
    CatalogLoaded(Result<Catalog, String>),
    ApiManifestLoaded(Result<ApiManifest, String>),
    SearchChanged(String),
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
    /// A background action finished with a human-readable status (or error).
    ActionDone(Result<String, String>),
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
            search: String::new(),
            status: String::new(),
            busy: false,
        };
        let boot = app.refresh_all(false);
        (app, boot)
    }

    fn theme(&self) -> Theme {
        self.theme.clone()
    }

    /// Re-scan installed mods from disk (cheap, synchronous).
    fn refresh_installed(&mut self) {
        if let Some(install) = &self.install {
            self.installed = mods::list_installed(install).unwrap_or_default();
        }
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
        match message {
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
                self.config.game_path = Some(install.root.clone());
                let _ = self.config.save();
                self.modpacks = modpack::Manager::for_install(&install).ok();
                self.install = Some(install);
                self.screen = Screen::Dashboard;
                self.refresh_all(false)
            }
            Message::SteamDetected(Err(e)) => {
                self.status = format!("Detection failed: {e}");
                Task::none()
            }
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
                Task::none()
            }
            Message::SearchChanged(q) => {
                self.search = q;
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
            Message::ActionDone(result) => {
                self.busy = false;
                self.apply_sync_result(result);
                Task::none()
            }
        }
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
            Screen::Settings => self.view_settings(),
            other => placeholder(other.label()),
        };

        let body = column![
            container(content)
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(24),
            self.status_bar(),
        ];

        row![self.sidebar(), body].into()
    }

    fn sidebar(&self) -> Element<'_, Message> {
        let mut col = column![text("aspid").size(28)].spacing(8).padding(16);
        for screen in Screen::ALL {
            let selected = screen == self.screen;
            let btn = button(text(screen.label()))
                .width(Length::Fill)
                .style(if selected {
                    button::primary
                } else {
                    button::text
                })
                .on_press(Message::Navigate(screen));
            col = col.push(btn);
        }
        container(col).width(180).height(Length::Fill).into()
    }

    fn status_bar(&self) -> Element<'_, Message> {
        let label = if self.busy {
            format!("⏳ {}", self.status)
        } else {
            self.status.clone()
        };
        container(text(label).size(13))
            .width(Length::Fill)
            .padding([6, 24])
            .into()
    }

    fn view_dashboard(&self) -> Element<'_, Message> {
        let Some(install) = &self.install else {
            return column![
                text("Dashboard").size(24),
                text("No game configured yet — head to Settings."),
            ]
            .spacing(12)
            .into();
        };

        let state = install.api_state();
        let state_text = match state {
            ApiState::Installed => "Modding API: installed (modded)".to_string(),
            ApiState::DisabledForVanilla => {
                "Modding API: installed (currently vanilla)".to_string()
            }
            ApiState::NotInstalled => "Modding API: not installed".to_string(),
            ApiState::Missing => "Install looks broken — assembly missing".to_string(),
        };

        let api_update = matches!(
            (&self.api_manifest, state.is_installed()),
            (Some(m), true) if modapi::update_available(install, m)
        );

        let api_button_label = if !state.is_installed() {
            "Install modding API"
        } else if api_update {
            "Update modding API"
        } else {
            "Reinstall modding API"
        };

        let mut col = column![
            text("Dashboard").size(24),
            text(format!("Install: {}", install.root.display())),
            text(state_text),
            text(format!("{} mods installed", self.installed.len())),
            Space::new().height(8),
        ]
        .spacing(12);

        col = col.push(button(text(api_button_label)).on_press_maybe(
            (!self.busy && self.api_manifest.is_some()).then_some(Message::InstallOrUpdateApi),
        ));

        let launch_enabled = !self.busy && state.is_installed();
        col = col.push(
            row![
                button(text("Launch modded"))
                    .style(button::primary)
                    .on_press_maybe(launch_enabled.then_some(Message::LaunchModded)),
                button(text("Launch vanilla"))
                    .style(button::secondary)
                    .on_press_maybe(launch_enabled.then_some(Message::LaunchVanilla)),
            ]
            .spacing(8),
        );

        col.into()
    }

    fn view_browse(&self) -> Element<'_, Message> {
        let header = column![
            text("Browse mods").size(24),
            text_input("Search mods…", &self.search).on_input(Message::SearchChanged),
        ]
        .spacing(12);

        let Some(catalog) = &self.catalog else {
            return column![header, text("Catalog not loaded yet.")]
                .spacing(12)
                .into();
        };

        let query = self.search.to_lowercase();
        let mut list = column![].spacing(6);
        let mut shown = 0usize;
        for m in catalog.mods() {
            if !query.is_empty()
                && !m.name.to_lowercase().contains(&query)
                && !m.description.to_lowercase().contains(&query)
            {
                continue;
            }
            list = list.push(self.mod_row(m));
            shown += 1;
        }

        column![
            header,
            text(format!("{shown} of {} mods", catalog.len())).size(13),
            scrollable(list).height(Length::Fill),
        ]
        .spacing(12)
        .into()
    }

    fn mod_row<'a>(&'a self, m: &'a Mod) -> Element<'a, Message> {
        let installed = self.is_installed(&m.name);
        let action: Element<'a, Message> = if installed {
            button(text("Remove"))
                .style(button::danger)
                .on_press_maybe((!self.busy).then(|| Message::RemoveMod(m.name.clone())))
                .into()
        } else {
            button(text("Install"))
                .style(button::primary)
                .on_press_maybe((!self.busy).then(|| Message::InstallMod(m.name.clone())))
                .into()
        };

        let info = column![
            text(&m.name).size(16),
            text(format!(
                "v{}  ·  {}",
                m.version,
                truncate(&m.description, 90)
            ))
            .size(12),
        ]
        .spacing(2)
        .width(Length::Fill);

        container(
            row![info, action]
                .spacing(12)
                .align_y(iced::Alignment::Center),
        )
        .padding(8)
        .into()
    }

    fn view_installed(&self) -> Element<'_, Message> {
        if self.installed.is_empty() {
            return column![
                text("Installed mods").size(24),
                text("Nothing installed yet — find mods in Browse."),
            ]
            .spacing(12)
            .into();
        }

        let mut list = column![].spacing(6);
        for m in &self.installed {
            let update = self
                .catalog
                .as_ref()
                .and_then(|c| c.get(&m.name))
                .map(|cm| m.update_available(cm))
                .unwrap_or(false);

            let version = m.version.clone().unwrap_or_else(|| "?".into());
            let label = if update {
                format!("{}  ·  v{version}  (update available)", m.name)
            } else {
                format!("{}  ·  v{version}", m.name)
            };

            let toggle_label = if m.enabled { "Disable" } else { "Enable" };
            let enabled = m.enabled;
            let name = m.name.clone();
            let name2 = m.name.clone();

            let actions = row![
                button(text(toggle_label))
                    .style(button::secondary)
                    .on_press_maybe((!self.busy).then_some(Message::SetModEnabled(name, !enabled))),
                button(text("Remove"))
                    .style(button::danger)
                    .on_press_maybe((!self.busy).then_some(Message::RemoveMod(name2))),
            ]
            .spacing(8);

            list = list.push(
                container(
                    row![text(label).width(Length::Fill), actions,]
                        .spacing(12)
                        .align_y(iced::Alignment::Center),
                )
                .padding(8),
            );
        }

        column![
            text("Installed mods").size(24),
            scrollable(list).height(Length::Fill)
        ]
        .spacing(12)
        .into()
    }

    fn view_modpacks(&self) -> Element<'_, Message> {
        let title = text("Modpacks").size(24);

        let Some(manager) = &self.modpacks else {
            return column![title, text("No game configured yet — head to Settings.")]
                .spacing(12)
                .into();
        };

        // Not yet initialised: explain the one-time capture and offer to enable.
        if manager.active().is_none() {
            return column![
                title,
                text(
                    "Modpacks give each setup its own mods and saves. Enabling will \
                     capture your current mods and save files as a “Default” pack \
                     (your data is moved, never deleted), and add an empty “Vanilla” pack."
                ),
                button(text("Enable modpacks"))
                    .style(button::primary)
                    .on_press_maybe((!self.busy).then_some(Message::EnableModpacks)),
            ]
            .spacing(16)
            .into();
        }

        let active = manager.active().map(str::to_string);
        let mut list = column![].spacing(6);
        for pack in manager.packs() {
            let is_active = active.as_deref() == Some(pack.id.as_str());
            let label = if is_active {
                format!("● {}  (active)", pack.name)
            } else {
                format!("○ {}", pack.name)
            };

            let id_act = pack.id.clone();
            let id_clone = pack.id.clone();
            let id_del = pack.id.clone();
            let deletable = !is_active && pack.id != modpack::VANILLA_ID;

            let actions = row![
                button(text("Activate"))
                    .style(button::primary)
                    .on_press_maybe(
                        (!self.busy && !is_active).then_some(Message::ActivatePack(id_act))
                    ),
                button(text("Clone"))
                    .style(button::secondary)
                    .on_press_maybe((!self.busy).then_some(Message::ClonePack(id_clone))),
                button(text("Delete")).style(button::danger).on_press_maybe(
                    (!self.busy && deletable).then_some(Message::DeletePack(id_del))
                ),
            ]
            .spacing(8);

            list = list.push(
                container(
                    row![text(label).width(Length::Fill), actions]
                        .spacing(12)
                        .align_y(iced::Alignment::Center),
                )
                .padding(8),
            );
        }

        let create_row = row![
            text_input("New pack name…", &self.new_pack_name)
                .on_input(Message::NewPackNameChanged)
                .on_submit(Message::CreatePack)
                .width(Length::Fill),
            button(text("Create")).on_press_maybe((!self.busy).then_some(Message::CreatePack)),
        ]
        .spacing(8);

        column![title, scrollable(list).height(Length::Fill), create_row,]
            .spacing(12)
            .into()
    }

    fn view_settings(&self) -> Element<'_, Message> {
        let detected = match &self.install {
            Some(i) => format!("Game path: {}", i.root.display()),
            None => "Game path: not set".into(),
        };
        let catalog_line = match &self.catalog {
            Some(c) => format!("Catalog: {} mods", c.len()),
            None => "Catalog: not loaded".into(),
        };

        scrollable(
            column![
                text("Settings").size(24),
                text(detected),
                button(text("Detect via Steam")).on_press(Message::DetectSteam),
                Space::new().height(8),
                text(catalog_line),
                button(text("Refresh catalog"))
                    .on_press_maybe((!self.busy).then_some(Message::RefreshCatalog)),
            ]
            .spacing(12),
        )
        .into()
    }
}

fn placeholder(name: &str) -> Element<'_, Message> {
    column![text(name).size(24), text("Coming soon.")]
        .spacing(12)
        .into()
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
