//! Locating and validating the Hollow Knight installation, and detecting the
//! current modding-API state.

use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::paths::{self, HOLLOW_KNIGHT_APP_ID};

/// A validated Hollow Knight installation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Install {
    /// The install root (contains `hollow_knight_Data/` or the macOS `.app` bundle).
    pub root: PathBuf,
    /// The resolved `Managed` directory.
    pub managed: PathBuf,
}

impl Install {
    /// The `Mods` directory for this install.
    pub fn mods_dir(&self) -> PathBuf {
        self.managed.join("Mods")
    }

    /// The active `Assembly-CSharp.dll`.
    pub fn assembly_dll(&self) -> PathBuf {
        self.managed.join("Assembly-CSharp.dll")
    }

    /// The vanilla backup created when the modding API is installed.
    pub fn vanilla_backup(&self) -> PathBuf {
        self.managed.join("Assembly-CSharp.dll.vanilla")
    }

    /// Current state of the modding API for this install.
    pub fn api_state(&self) -> ApiState {
        detect_api_state(&self.root)
    }
}

/// State of the modding API for an install.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiState {
    /// Modding API installed and currently active (the active assembly is modded).
    Installed,
    /// Modding API installed but temporarily switched to vanilla for a vanilla launch.
    DisabledForVanilla,
    /// No modding API installed — a pristine vanilla install.
    NotInstalled,
    /// The managed assembly is missing entirely — the install looks broken.
    Missing,
}

impl ApiState {
    /// Whether the modding API is present at all (active or toggled off).
    pub fn is_installed(self) -> bool {
        matches!(self, ApiState::Installed | ApiState::DisabledForVanilla)
    }
}

/// Validate that `root` looks like a real Hollow Knight install and return it.
pub fn validate(root: impl Into<PathBuf>) -> Result<Install> {
    let root = root.into();
    let managed = paths::managed_dir(&root);
    if !managed.is_dir() {
        return Err(Error::InvalidInstall {
            path: root,
            reason: format!(
                "no Managed directory found (looked at {})",
                managed.display()
            ),
        });
    }
    // A real install always has the core Unity assembly alongside the game assembly.
    let unity = managed.join("UnityEngine.dll");
    let assembly = managed.join("Assembly-CSharp.dll");
    if !unity.exists() && !assembly.exists() {
        return Err(Error::InvalidInstall {
            path: root,
            reason: "Managed directory is missing the expected assemblies".to_string(),
        });
    }
    Ok(Install { root, managed })
}

/// Attempt to locate the Hollow Knight install automatically via Steam.
pub fn locate_steam() -> Result<Install> {
    let steam = steamlocate::locate().map_err(|_| Error::GameNotFound)?;
    let (app, library) = steam
        .find_app(HOLLOW_KNIGHT_APP_ID)
        .map_err(|_| Error::GameNotFound)?
        .ok_or(Error::GameNotFound)?;
    let dir = library.resolve_app_dir(&app);
    validate(dir)
}

/// Detect the modding-API state at an install root without full validation.
///
/// The active `Assembly-CSharp.dll` is always the one the game loads. We track two
/// sidecars: `.vanilla` (present once the API has ever been installed) and `.modded`
/// (present only while temporarily running vanilla, holding the modded assembly aside).
pub fn detect_api_state(root: &Path) -> ApiState {
    let vanilla = paths::vanilla_backup(root);
    let modded = paths::modded_backup(root);
    let assembly = paths::assembly_dll(root);
    if modded.exists() {
        ApiState::DisabledForVanilla
    } else if vanilla.exists() {
        ApiState::Installed
    } else if assembly.exists() {
        ApiState::NotInstalled
    } else {
        ApiState::Missing
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_install(managed_layout: &str) -> (tempfile::TempDir, PathBuf) {
        let tmp = tempfile::tempdir().unwrap();
        let managed = tmp.path().join(managed_layout).join("Managed");
        std::fs::create_dir_all(&managed).unwrap();
        std::fs::write(managed.join("UnityEngine.dll"), b"x").unwrap();
        std::fs::write(managed.join("Assembly-CSharp.dll"), b"x").unwrap();
        let root = tmp.path().to_path_buf();
        (tmp, root)
    }

    #[test]
    fn validates_linux_layout() {
        let (_tmp, root) = fake_install("hollow_knight_Data");
        let install = validate(&root).unwrap();
        assert!(install.managed.ends_with("Managed"));
        assert_eq!(install.api_state(), ApiState::NotInstalled);
    }

    #[test]
    fn rejects_non_install() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(validate(tmp.path()).is_err());
    }

    #[test]
    fn detects_modded_when_backup_present() {
        let (_tmp, root) = fake_install("hollow_knight_Data");
        std::fs::write(paths::vanilla_backup(&root), b"vanilla").unwrap();
        assert_eq!(detect_api_state(&root), ApiState::Installed);
    }

    #[test]
    fn detects_disabled_for_vanilla_when_modded_stashed() {
        let (_tmp, root) = fake_install("hollow_knight_Data");
        std::fs::write(paths::vanilla_backup(&root), b"vanilla").unwrap();
        std::fs::write(paths::modded_backup(&root), b"modded").unwrap();
        assert_eq!(detect_api_state(&root), ApiState::DisabledForVanilla);
    }
}
