//! Persisted application configuration.
//!
//! Stored as human-editable TOML in the platform config directory. Machine state that
//! changes frequently (catalog cache, per-pack bookkeeping) lives elsewhere as JSON.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::paths;

/// The default ModLinks catalog feed.
pub const DEFAULT_MODLINKS_URL: &str =
    "https://raw.githubusercontent.com/hk-modding/modlinks/main/ModLinks.xml";

/// The default ApiLinks (modding API) feed.
pub const DEFAULT_APILINKS_URL: &str =
    "https://raw.githubusercontent.com/hk-modding/modlinks/main/ApiLinks.xml";

/// The default skin catalog: HKSkins' bulk metadata export (a zip of per-skin
/// `metadata.json` + `preview.png`). Overridable in config.
pub const DEFAULT_SKIN_CATALOG_URL: &str = "https://hkskins.art/skins.zip";

/// Theme appearance settings (preset + accent override).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    /// Name of the built-in iced preset to start from (e.g. `"Dark"`, `"TokyoNight"`).
    pub preset: String,
    /// Optional accent colour as `#RRGGBB`; overrides the preset's primary colour.
    pub accent: Option<String>,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            // aspid's signature theme (see the `theme` module in the UI crate).
            preset: "Aspid Dark".to_string(),
            accent: None,
        }
    }
}

/// Top-level persisted configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Config {
    /// Manually-selected or last-detected game install root.
    pub game_path: Option<PathBuf>,
    /// Override for the ModLinks feed URL.
    pub modlinks_url: Option<String>,
    /// Override for the ApiLinks feed URL.
    pub apilinks_url: Option<String>,
    /// If set, pin the modding API to this version instead of auto-updating.
    pub pinned_api_version: Option<String>,
    /// The currently active modpack id (`None` means the implicit vanilla pack).
    pub active_pack: Option<String>,
    /// Override for the skin catalog manifest URL.
    pub skin_catalog_url: Option<String>,
    /// Active skin per cosmetic kind (kind id → skin name). Persists across modpacks.
    #[serde(default)]
    pub active_skins: std::collections::BTreeMap<String, String>,
    /// Appearance settings.
    pub theme: ThemeConfig,
}

impl Config {
    /// The effective ModLinks URL (override or default).
    pub fn modlinks_url(&self) -> &str {
        self.modlinks_url.as_deref().unwrap_or(DEFAULT_MODLINKS_URL)
    }

    /// The effective ApiLinks URL (override or default).
    pub fn apilinks_url(&self) -> &str {
        self.apilinks_url.as_deref().unwrap_or(DEFAULT_APILINKS_URL)
    }

    /// The effective skin catalog URL (override or default).
    pub fn skin_catalog_url(&self) -> &str {
        self.skin_catalog_url
            .as_deref()
            .unwrap_or(DEFAULT_SKIN_CATALOG_URL)
    }

    /// The on-disk location of the config file.
    pub fn path() -> Result<PathBuf> {
        Ok(paths::config_dir()?.join("config.toml"))
    }

    /// Load config from the default location, returning defaults if it does not exist yet.
    pub fn load() -> Result<Self> {
        let path = Self::path()?;
        Self::load_from(&path)
    }

    /// Load config from an explicit path, returning defaults if absent.
    pub fn load_from(path: &Path) -> Result<Self> {
        match std::fs::read_to_string(path) {
            Ok(text) => toml::from_str(&text).map_err(|e| Error::Config(e.to_string())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(Error::io(path, e)),
        }
    }

    /// Persist config to the default location, creating parent directories as needed.
    pub fn save(&self) -> Result<()> {
        let path = Self::path()?;
        self.save_to(&path)
    }

    /// Persist config to an explicit path.
    pub fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
        }
        let text = toml::to_string_pretty(self).map_err(|e| Error::Config(e.to_string()))?;
        std::fs::write(path, text).map_err(|e| Error::io(path, e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_have_sane_feeds() {
        let cfg = Config::default();
        assert_eq!(cfg.modlinks_url(), DEFAULT_MODLINKS_URL);
        assert_eq!(cfg.apilinks_url(), DEFAULT_APILINKS_URL);
        assert!(cfg.game_path.is_none());
    }

    #[test]
    fn roundtrips_through_toml() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("config.toml");

        let cfg = Config {
            game_path: Some(PathBuf::from("/games/Hollow Knight")),
            theme: ThemeConfig {
                accent: Some("#E06C75".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };
        cfg.save_to(&path).unwrap();

        let loaded = Config::load_from(&path).unwrap();
        assert_eq!(loaded.game_path, cfg.game_path);
        assert_eq!(loaded.theme.accent.as_deref(), Some("#E06C75"));
    }

    #[test]
    fn missing_file_yields_defaults() {
        let tmp = tempfile::tempdir().unwrap();
        let cfg = Config::load_from(&tmp.path().join("nope.toml")).unwrap();
        assert!(cfg.game_path.is_none());
    }
}
