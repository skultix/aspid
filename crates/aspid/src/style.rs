//! Visual design system: spacing scale and palette-derived widget styles.
//!
//! Every style reads from the active theme's extended palette, so the whole UI adapts to
//! the chosen preset and accent colour. Inspired by modern game mod managers (Modrinth
//! App, Prism Launcher): an elevated sidebar, card-based content, and accent-driven
//! primary actions.

use iced::font::{Family, Stretch, Weight};
use iced::widget::text::LineHeight;
use iced::widget::{button, container, scrollable, svg, text, text_input, Text};
use iced::{Background, Border, Color, Font, Shadow, Theme, Vector};

// Spacing scale (px). f32 so they work with both `spacing`/`size` (Pixels) and `padding`.
pub const XXS: f32 = 2.0;
pub const XS: f32 = 4.0;
pub const SM: f32 = 8.0;
pub const MD: f32 = 12.0;
pub const LG: f32 = 16.0;
pub const XL: f32 = 24.0;

pub const RADIUS_SM: f32 = 6.0;
pub const RADIUS: f32 = 10.0;
pub const RADIUS_LG: f32 = 14.0;
pub const PILL: f32 = 999.0;
pub const SIDEBAR_W: f32 = 224.0;

/// Maximum width of centered screen content, so nothing stretches on wide windows.
pub const CONTENT_MAX: f32 = 940.0;
/// Right gutter reserved for the scrollbar so it never overlaps cards.
pub const SCROLL_GUTTER: f32 = 14.0;

// ---- Typography (Inter, variable) --------------------------------------------

const fn inter(weight: Weight) -> Font {
    Font {
        family: Family::Name("Inter"),
        weight,
        stretch: Stretch::Normal,
        style: iced::font::Style::Normal,
    }
}

pub const REGULAR: Font = inter(Weight::Normal);
pub const MEDIUM: Font = inter(Weight::Medium);
pub const SEMIBOLD: Font = inter(Weight::Semibold);

/// Page title.
pub fn title<'a>(s: impl text::IntoFragment<'a>) -> Text<'a> {
    text(s)
        .size(25)
        .font(SEMIBOLD)
        .line_height(LineHeight::Relative(1.2))
}

/// Card / section heading.
pub fn section<'a>(s: impl text::IntoFragment<'a>) -> Text<'a> {
    text(s).size(15).font(SEMIBOLD)
}

/// Emphasised label (e.g. a mod/card name).
pub fn strong<'a>(s: impl text::IntoFragment<'a>) -> Text<'a> {
    text(s).size(14).font(SEMIBOLD)
}

/// Body copy with comfortable line height.
pub fn body<'a>(s: impl text::IntoFragment<'a>) -> Text<'a> {
    text(s).size(13).line_height(LineHeight::Relative(1.45))
}

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

/// Accent-tinted icon.
pub fn icon_accent(theme: &Theme, _status: svg::Status) -> svg::Style {
    svg::Style {
        color: Some(theme.extended_palette().primary.base.color),
    }
}

/// Icon coloured for placement on a filled accent (primary) button.
pub fn icon_on_accent(theme: &Theme, _status: svg::Status) -> svg::Style {
    svg::Style {
        color: Some(theme.extended_palette().primary.base.text),
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

/// A round swatch filled with a specific colour (accent picker presets).
pub fn swatch(color: Color) -> impl Fn(&Theme) -> container::Style {
    move |_theme: &Theme| container::Style {
        background: Some(Background::Color(color)),
        border: Border {
            color: alpha(Color::WHITE, 0.25),
            width: 1.0,
            radius: PILL.into(),
        },
        ..Default::default()
    }
}

/// A round swatch filled with the active accent.
pub fn accent_swatch(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(
            theme.extended_palette().primary.base.color,
        )),
        border: Border {
            color: alpha(Color::WHITE, 0.25),
            width: 1.0,
            radius: PILL.into(),
        },
        ..Default::default()
    }
}

/// A small filled accent dot (busy indicator).
pub fn dot(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(
            theme.extended_palette().primary.base.color,
        )),
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

/// Sidebar nav item; `active` highlights the current screen with an accent left-bar.
pub fn nav(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme, status| {
        let p = theme.extended_palette();
        if active {
            return button::Style {
                background: Some(Background::Color(alpha(p.primary.base.color, 0.16))),
                text_color: p.primary.base.color,
                border: Border {
                    color: p.primary.base.color,
                    width: 0.0,
                    radius: RADIUS.into(),
                },
                ..Default::default()
            };
        }
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        button::Style {
            background: hovered.then(|| Background::Color(alpha(p.background.strong.color, 0.45))),
            text_color: alpha(p.background.base.text, if hovered { 0.95 } else { 0.72 }),
            border: rounded(RADIUS),
            ..Default::default()
        }
    }
}

// ---- Extra surfaces ----------------------------------------------------------

fn surface_color(p: &iced::theme::palette::Extended, lift: f32) -> Color {
    if p.is_dark {
        mix(p.background.base.color, Color::WHITE, lift)
    } else {
        mix(Color::WHITE, p.background.base.color, lift)
    }
}

/// A card lifted on hover (lighter surface + accent border + a touch more shadow).
pub fn card_hover(theme: &Theme) -> container::Style {
    let p = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(surface_color(
            p,
            if p.is_dark { 0.08 } else { 0.02 },
        ))),
        border: Border {
            color: alpha(p.primary.base.color, 0.45),
            width: 1.0,
            radius: RADIUS_LG.into(),
        },
        shadow: Shadow {
            color: alpha(Color::BLACK, if p.is_dark { 0.35 } else { 0.12 }),
            offset: Vector::new(0.0, 3.0),
            blur_radius: 14.0,
        },
        ..Default::default()
    }
}

/// A neutral inner panel/row surface (less elevated than [`card`]).
pub fn surface(theme: &Theme) -> container::Style {
    let p = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(surface_color(
            p,
            if p.is_dark { 0.03 } else { 0.04 },
        ))),
        border: Border {
            color: alpha(p.background.strong.color, 0.4),
            width: 1.0,
            radius: RADIUS.into(),
        },
        ..Default::default()
    }
}

/// A warning chip (e.g. "update available").
pub fn chip_warn(theme: &Theme) -> container::Style {
    let p = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(alpha(p.warning.base.color, 0.16))),
        text_color: Some(p.warning.base.color),
        border: Border {
            radius: PILL.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

/// A category tag chip, coloured by a stable hash of its label.
pub fn tag(label: &str) -> impl Fn(&Theme) -> container::Style {
    let hue = stable_hue(label);
    move |theme: &Theme| {
        let p = theme.extended_palette();
        let c = hsl(hue, 0.55, if p.is_dark { 0.72 } else { 0.42 });
        container::Style {
            background: Some(Background::Color(alpha(c, 0.16))),
            text_color: Some(c),
            border: Border {
                radius: PILL.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

// ---- Inputs & scrollbars -----------------------------------------------------

/// Text input with a recessed background and an accent focus ring.
pub fn input(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let p = theme.extended_palette();
    let bg = if p.is_dark {
        mix(p.background.base.color, Color::BLACK, 0.22)
    } else {
        surface_color(p, 0.05)
    };
    let border_color = match status {
        text_input::Status::Focused { .. } => p.primary.base.color,
        text_input::Status::Hovered => alpha(p.background.strong.color, 0.9),
        _ => alpha(p.background.strong.color, 0.6),
    };
    text_input::Style {
        background: Background::Color(bg),
        border: Border {
            color: border_color,
            width: 1.0,
            radius: RADIUS.into(),
        },
        icon: alpha(p.background.base.text, 0.6),
        placeholder: alpha(p.background.base.text, 0.4),
        value: p.background.base.text,
        selection: alpha(p.primary.base.color, 0.4),
    }
}

/// Slim, rounded, theme-tinted scrollbar.
pub fn scrollbar(theme: &Theme, status: scrollable::Status) -> scrollable::Style {
    let p = theme.extended_palette();
    let active = matches!(
        status,
        scrollable::Status::Hovered { .. } | scrollable::Status::Dragged { .. }
    );
    let scroller = scrollable::Scroller {
        background: Background::Color(alpha(
            p.background.base.text,
            if active { 0.4 } else { 0.22 },
        )),
        border: rounded(RADIUS_SM),
    };
    let rail = scrollable::Rail {
        background: None,
        border: rounded(RADIUS_SM),
        scroller,
    };
    scrollable::Style {
        container: container::Style::default(),
        vertical_rail: rail,
        horizontal_rail: rail,
        gap: None,
        auto_scroll: scrollable::AutoScroll {
            background: Background::Color(Color::TRANSPARENT),
            border: Border::default(),
            shadow: Shadow::default(),
            icon: alpha(p.background.base.text, 0.6),
        },
    }
}

// ---- Colour helpers ----------------------------------------------------------

fn stable_hue(label: &str) -> f32 {
    let mut h: u32 = 2166136261;
    for b in label.bytes() {
        h = (h ^ b as u32).wrapping_mul(16777619);
    }
    (h % 360) as f32
}

/// HSL → linear-ish sRGB Color (h in 0..360, s/l in 0..1).
fn hsl(h: f32, s: f32, l: f32) -> Color {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let hp = h / 60.0;
    let x = c * (1.0 - (hp % 2.0 - 1.0).abs());
    let (r, g, b) = match hp as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c / 2.0;
    Color::from_rgb(r + m, g + m, b + m)
}
