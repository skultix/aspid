//! aspid — a cross-platform Hollow Knight mod manager (Iced front-end).

mod theme;

use aspid_core::config::Config;
use aspid_core::game::{self, ApiState, Install};

use iced::widget::{button, column, container, row, scrollable, text, Space};
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
    status: String,
}

#[derive(Debug, Clone)]
enum Message {
    Navigate(Screen),
    DetectSteam,
    SteamDetected(Result<Install, String>),
}

impl App {
    fn new() -> (Self, Task<Message>) {
        let config = Config::load().unwrap_or_default();
        let theme = theme::from_config(&config.theme);

        // If a game path is already configured, validate it eagerly.
        let install = config
            .game_path
            .as_ref()
            .and_then(|p| game::validate(p).ok());

        let screen = if install.is_some() {
            Screen::Dashboard
        } else {
            Screen::Settings
        };

        let app = App {
            config,
            theme,
            screen,
            install,
            status: String::new(),
        };
        (app, Task::none())
    }

    fn theme(&self) -> Theme {
        self.theme.clone()
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Navigate(screen) => {
                self.screen = screen;
                Task::none()
            }
            Message::DetectSteam => {
                self.status = "Searching for a Steam installation…".to_string();
                Task::perform(
                    async { game::locate_steam().map_err(|e| e.to_string()) },
                    Message::SteamDetected,
                )
            }
            Message::SteamDetected(Ok(install)) => {
                self.status = format!("Found install at {}", install.root.display());
                self.config.game_path = Some(install.root.clone());
                let _ = self.config.save();
                self.install = Some(install);
                Task::none()
            }
            Message::SteamDetected(Err(e)) => {
                self.status = format!("Detection failed: {e}");
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let content = match self.screen {
            Screen::Settings => self.view_settings(),
            Screen::Dashboard => self.view_dashboard(),
            other => placeholder(other.label()),
        };

        row![
            self.sidebar(),
            container(content).width(Length::Fill).padding(24),
        ]
        .into()
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

    fn view_dashboard(&self) -> Element<'_, Message> {
        let body = match &self.install {
            Some(install) => {
                let state = match install.api_state() {
                    ApiState::Modded => "Modding API installed (modded)",
                    ApiState::Vanilla => "Vanilla (no modding API)",
                    ApiState::Missing => "Install looks broken — assembly missing",
                };
                column![
                    text("Dashboard").size(24),
                    text(format!("Install: {}", install.root.display())),
                    text(state),
                ]
            }
            None => column![
                text("Dashboard").size(24),
                text("No game configured yet — head to Settings."),
            ],
        };
        body.spacing(12).into()
    }

    fn view_settings(&self) -> Element<'_, Message> {
        let detected = match &self.install {
            Some(i) => format!("Game path: {}", i.root.display()),
            None => "Game path: not set".to_string(),
        };

        scrollable(
            column![
                text("Settings").size(24),
                text(detected),
                button(text("Detect via Steam")).on_press(Message::DetectSteam),
                Space::new().height(8),
                text(&self.status),
            ]
            .spacing(12),
        )
        .into()
    }
}

fn placeholder(name: &str) -> Element<'_, Message> {
    column![text(name).size(24), text("Coming soon."),]
        .spacing(12)
        .into()
}
