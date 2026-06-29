//! Visual design system: spacing scale and palette-derived widget styles.
//!
//! Every style reads from the active theme's extended palette, so the whole UI adapts to
//! the chosen preset and accent colour. Inspired by modern game mod managers (Modrinth
//! App, Prism Launcher): an elevated sidebar, card-based content, and accent-driven
//! primary actions.

use iced::widget::{button, container, svg, text};
use iced::{Background, Border, Color, Shadow, Theme, Vector};

// Spacing scale (px). f32 so they work with both `spacing`/`size` (Pixels) and `padding`.
pub const XS: f32 = 4.0;
pub const SM: f32 = 8.0;
pub const MD: f32 = 12.0;
pub const LG: f32 = 16.0;
pub const XL: f32 = 24.0;

pub const RADIUS: f32 = 8.0;
pub const RADIUS_LG: f32 = 12.0;
pub const PILL: f32 = 999.0;
pub const SIDEBAR_W: f32 = 216.0;

/// Symmetric padding: `v` vertical, `h` horizontal.
pub fn pad(v: f32, h: f32) -> iced::Padding {
    iced::Padding {
        top: v,
        right: h,
        bottom: v,
        left: h,
    }
}

fn alpha(c: Color, a: f32) -> Color {
    Color { a, ..c }
}

/// Mix two colours by `t` (0 = a, 1 = b).
fn mix(a: Color, b: Color, t: f32) -> Color {
    Color {
        r: a.r + (b.r - a.r) * t,
        g: a.g + (b.g - a.g) * t,
        b: a.b + (b.b - a.b) * t,
        a: 1.0,
    }
}

// ---- Text colours ------------------------------------------------------------

/// Secondary, de-emphasised text.
pub fn muted(theme: &Theme) -> text::Style {
    let p = theme.extended_palette();
    text::Style {
        color: Some(alpha(p.background.base.text, 0.6)),
    }
}

/// Accent-coloured text.
pub fn accent(theme: &Theme) -> text::Style {
    text::Style {
        color: Some(theme.extended_palette().primary.base.color),
    }
}

/// Monochrome icon (SVG), tinted to the foreground and brightening on hover.
pub fn icon(theme: &Theme, status: svg::Status) -> svg::Style {
    let p = theme.extended_palette();
    let a = if matches!(status, svg::Status::Hovered) {
        1.0
    } else {
        0.7
    };
    svg::Style {
        color: Some(alpha(p.background.base.text, a)),
    }
}

/// Success-tinted icon (e.g. an "installed" tick).
pub fn icon_success(theme: &Theme, _status: svg::Status) -> svg::Style {
    svg::Style {
        color: Some(theme.extended_palette().success.base.color),
    }
}

/// A dimmed full-screen backdrop behind a modal.
pub fn backdrop(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.55,
        })),
        ..Default::default()
    }
}

// ---- Surfaces ----------------------------------------------------------------

/// The window background.
pub fn root(theme: &Theme) -> container::Style {
    let p = theme.extended_palette();
    let bg = if p.is_dark {
        mix(p.background.base.color, Color::BLACK, 0.35)
    } else {
        p.background.base.color
    };
    container::Style {
        background: Some(Background::Color(bg)),
        text_color: Some(p.background.base.text),
        ..Default::default()
    }
}

/// The elevated left navigation rail.
pub fn sidebar(theme: &Theme) -> container::Style {
    let p = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(p.background.base.color)),
        border: Border {
            color: alpha(p.background.strong.color, 0.5),
            width: 1.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

/// A standard content card.
pub fn card(theme: &Theme) -> container::Style {
    let p = theme.extended_palette();
    let bg = if p.is_dark {
        mix(p.background.base.color, Color::WHITE, 0.04)
    } else {
        Color::WHITE
    };
    container::Style {
        background: Some(Background::Color(bg)),
        border: Border {
            color: alpha(p.background.strong.color, 0.6),
            width: 1.0,
            radius: RADIUS_LG.into(),
        },
        shadow: Shadow {
            color: alpha(Color::BLACK, if p.is_dark { 0.25 } else { 0.08 }),
            offset: Vector::new(0.0, 1.0),
            blur_radius: 6.0,
        },
        ..Default::default()
    }
}

/// A prominent, accent-tinted card (e.g. the dashboard hero, active modpack).
pub fn hero(theme: &Theme) -> container::Style {
    let p = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(alpha(p.primary.base.color, 0.10))),
        border: Border {
            color: alpha(p.primary.base.color, 0.5),
            width: 1.0,
            radius: RADIUS_LG.into(),
        },
        ..Default::default()
    }
}

/// A small rounded chip/badge (accent).
pub fn chip(theme: &Theme) -> container::Style {
    let p = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(alpha(p.primary.base.color, 0.16))),
        text_color: Some(p.primary.base.color),
        border: Border {
            radius: PILL.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

/// A neutral chip (e.g. tags, version).
pub fn chip_neutral(theme: &Theme) -> container::Style {
    let p = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(alpha(p.background.strong.color, 0.5))),
        text_color: Some(alpha(p.background.base.text, 0.75)),
        border: Border {
            radius: PILL.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

/// A success chip (e.g. "active", "installed").
pub fn chip_success(theme: &Theme) -> container::Style {
    let p = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(alpha(p.success.base.color, 0.16))),
        text_color: Some(p.success.base.color),
        border: Border {
            radius: PILL.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

/// The bottom status bar.
pub fn status_bar(theme: &Theme) -> container::Style {
    let p = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(p.background.base.color)),
        border: Border {
            color: alpha(p.background.strong.color, 0.5),
            width: 1.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

// ---- Buttons -----------------------------------------------------------------

fn rounded(radius: f32) -> Border {
    Border {
        radius: radius.into(),
        ..Default::default()
    }
}

/// Filled accent button for primary actions.
pub fn primary(theme: &Theme, status: button::Status) -> button::Style {
    let p = theme.extended_palette();
    let bg = match status {
        button::Status::Hovered => p.primary.strong.color,
        button::Status::Pressed => mix(p.primary.base.color, Color::BLACK, 0.2),
        button::Status::Disabled => alpha(p.primary.base.color, 0.35),
        button::Status::Active => p.primary.base.color,
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: p.primary.base.text,
        border: rounded(RADIUS),
        ..Default::default()
    }
}

/// Subtle neutral button.
pub fn secondary(theme: &Theme, status: button::Status) -> button::Style {
    let p = theme.extended_palette();
    let base = alpha(p.background.strong.color, 0.5);
    let bg = match status {
        button::Status::Hovered => alpha(p.background.strong.color, 0.8),
        button::Status::Pressed => p.background.strong.color,
        button::Status::Disabled => alpha(p.background.strong.color, 0.25),
        button::Status::Active => base,
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: p.background.base.text,
        border: rounded(RADIUS),
        ..Default::default()
    }
}

/// Destructive button — muted until hovered.
pub fn danger(theme: &Theme, status: button::Status) -> button::Style {
    let p = theme.extended_palette();
    let (bg, text) = match status {
        button::Status::Hovered => (Some(p.danger.base.color), p.danger.base.text),
        button::Status::Pressed => (
            Some(mix(p.danger.base.color, Color::BLACK, 0.2)),
            p.danger.base.text,
        ),
        button::Status::Disabled => (
            Some(alpha(p.danger.base.color, 0.2)),
            alpha(p.danger.base.text, 0.5),
        ),
        button::Status::Active => (Some(alpha(p.danger.base.color, 0.14)), p.danger.base.color),
    };
    button::Style {
        background: bg.map(Background::Color),
        text_color: text,
        border: rounded(RADIUS),
        ..Default::default()
    }
}

/// Borderless icon/text button — transparent until hovered.
pub fn ghost(theme: &Theme, status: button::Status) -> button::Style {
    let p = theme.extended_palette();
    let bg = match status {
        button::Status::Hovered => Some(alpha(p.background.strong.color, 0.5)),
        button::Status::Pressed => Some(alpha(p.background.strong.color, 0.7)),
        _ => None,
    };
    button::Style {
        background: bg.map(Background::Color),
        text_color: alpha(p.background.base.text, 0.85),
        border: rounded(RADIUS),
        ..Default::default()
    }
}

/// Sidebar nav item; `active` highlights the current screen.
pub fn nav(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme, status| {
        let p = theme.extended_palette();
        if active {
            return button::Style {
                background: Some(Background::Color(alpha(p.primary.base.color, 0.18))),
                text_color: p.primary.base.color,
                border: rounded(RADIUS),
                ..Default::default()
            };
        }
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        button::Style {
            background: hovered.then(|| Background::Color(alpha(p.background.strong.color, 0.5))),
            text_color: alpha(p.background.base.text, if hovered { 0.95 } else { 0.7 }),
            border: rounded(RADIUS),
            ..Default::default()
        }
    }
}
