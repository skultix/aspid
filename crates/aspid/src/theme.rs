//! Translates the persisted [`ThemeConfig`] into a concrete iced [`Theme`].
//!
//! aspid ships a signature **"Aspid Dark"** theme as the default look, but every built-in
//! iced preset is also selectable, and a user accent colour overrides the palette's
//! `primary` role on top of whichever base is chosen.

use aspid_core::config::ThemeConfig;
use iced::theme::Palette;
use iced::{Color, Theme};

/// Name of aspid's signature theme.
pub const SIGNATURE: &str = "Aspid Dark";

/// The tuned palette behind [`SIGNATURE`] — a deep neutral dark with a soft blue accent.
fn aspid_palette() -> Palette {
    Palette {
        background: Color::from_rgb8(0x16, 0x18, 0x1C),
        text: Color::from_rgb8(0xE7, 0xEA, 0xEF),
        // aspid orange — sampled and brightened from the app icon.
        primary: Color::from_rgb8(0xE0, 0x65, 0x2E),
        success: Color::from_rgb8(0x3F, 0xB9, 0x50),
        warning: Color::from_rgb8(0xD6, 0xA2, 0x1E),
        danger: Color::from_rgb8(0xE5, 0x53, 0x4B),
    }
}

/// The signature theme.
fn aspid_dark() -> Theme {
    Theme::custom(SIGNATURE.to_string(), aspid_palette())
}

/// Build a concrete iced [`Theme`] from the persisted appearance config.
pub fn from_config(cfg: &ThemeConfig) -> Theme {
    let base = preset(&cfg.preset);
    match cfg.accent.as_deref().and_then(parse_hex) {
        Some(accent) => {
            let palette = Palette {
                primary: accent,
                ..base.palette()
            };
            Theme::custom(format!("{} ·", base), palette)
        }
        None => base,
    }
}

/// The list of preset names a user can pick from: the signature theme first, then iced's.
pub fn preset_names() -> Vec<String> {
    let mut names = vec![SIGNATURE.to_string()];
    names.extend(Theme::ALL.iter().map(|t| t.to_string()));
    names
}

/// Resolve a preset name to a [`Theme`], defaulting to the signature theme.
fn preset(name: &str) -> Theme {
    if name == SIGNATURE {
        return aspid_dark();
    }
    Theme::ALL
        .iter()
        .find(|t| t.to_string() == name)
        .cloned()
        .unwrap_or_else(aspid_dark)
}

/// Parse a `#RRGGBB` (or `RRGGBB`) string into a [`Color`].
fn parse_hex(s: &str) -> Option<Color> {
    let s = s.trim().trim_start_matches('#');
    if s.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&s[0..2], 16).ok()?;
    let g = u8::from_str_radix(&s[2..4], 16).ok()?;
    let b = u8::from_str_radix(&s[4..6], 16).ok()?;
    Some(Color::from_rgb8(r, g, b))
}
