//! Translates the persisted [`ThemeConfig`] into a concrete iced [`Theme`].
//!
//! We start from one of iced's built-in preset palettes and, if the user has chosen an
//! accent colour, override the palette's `primary` role. iced derives the per-widget
//! "extended" palette from these base colours, so the accent propagates everywhere.

use aspid_core::config::ThemeConfig;
use iced::theme::Palette;
use iced::{Color, Theme};

/// Build a concrete iced [`Theme`] from the persisted appearance config.
pub fn from_config(cfg: &ThemeConfig) -> Theme {
    let base = preset(&cfg.preset);
    match cfg.accent.as_deref().and_then(parse_hex) {
        Some(accent) => {
            let palette = Palette {
                primary: accent,
                ..base.palette()
            };
            Theme::custom("aspid".to_string(), palette)
        }
        None => base,
    }
}

/// The list of preset names a user can pick from (the names of iced's built-in themes).
// Wired into the Settings theme picker in Phase 3.
#[allow(dead_code)]
pub fn preset_names() -> Vec<String> {
    Theme::ALL.iter().map(|t| t.to_string()).collect()
}

/// Resolve a preset name to a built-in [`Theme`], defaulting to [`Theme::Dark`].
fn preset(name: &str) -> Theme {
    Theme::ALL
        .iter()
        .find(|t| t.to_string() == name)
        .cloned()
        .unwrap_or(Theme::Dark)
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
