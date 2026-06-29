//! Per-OS path resolution for the game install, Unity save data, and aspid's own
//! application directories, plus the cross-platform directory-link abstraction used
//! by the modpack swapper.

use std::path::{Path, PathBuf};

use directories::{BaseDirs, ProjectDirs};

use crate::error::{Error, Result};

/// The Hollow Knight Steam app id.
pub const HOLLOW_KNIGHT_APP_ID: u32 = 367520;

/// aspid's application directories (config / data / cache), namespaced per platform.
pub fn app_dirs() -> Result<ProjectDirs> {
    ProjectDirs::from("com", "marlstar", "aspid").ok_or(Error::MissingPlatformDir("application"))
}

/// Directory where aspid stores modpacks, cached catalogs, skins, etc.
pub fn data_dir() -> Result<PathBuf> {
    Ok(app_dirs()?.data_dir().to_path_buf())
}

/// Directory where aspid stores its config + persisted state.
pub fn config_dir() -> Result<PathBuf> {
    Ok(app_dirs()?.config_dir().to_path_buf())
}

/// The Unity `persistentDataPath` for Hollow Knight, where `userN.dat` saves and mod
/// settings live. This is independent of the game install location.
pub fn unity_save_dir() -> Result<PathBuf> {
    let base = BaseDirs::new().ok_or(Error::MissingPlatformDir("home"))?;

    #[cfg(target_os = "linux")]
    {
        Ok(base
            .config_dir()
            .join("unity3d")
            .join("Team Cherry")
            .join("Hollow Knight"))
    }

    #[cfg(target_os = "windows")]
    {
        // Unity uses %USERPROFILE%\AppData\LocalLow, which `directories` does not expose
        // directly (it gives AppData\Local), so derive it from the home directory.
        Ok(base
            .home_dir()
            .join("AppData")
            .join("LocalLow")
            .join("Team Cherry")
            .join("Hollow Knight"))
    }

    #[cfg(target_os = "macos")]
    {
        Ok(base
            .home_dir()
            .join("Library")
            .join("Application Support")
            .join("unity.Team Cherry.Hollow Knight"))
    }
}

/// Candidate names for the `*_Data` folder relative to the install root, in priority order.
/// macOS ships the game as an `.app` bundle, so its data folder lives elsewhere.
fn managed_candidates(game_root: &Path) -> Vec<PathBuf> {
    vec![
        // Windows / Linux layout.
        game_root.join("hollow_knight_Data").join("Managed"),
        // macOS app-bundle layout (root is the install dir containing the .app).
        game_root
            .join("Hollow Knight.app")
            .join("Contents")
            .join("Resources")
            .join("Data")
            .join("Managed"),
        // macOS layout when the root *is* the .app bundle.
        game_root
            .join("Contents")
            .join("Resources")
            .join("Data")
            .join("Managed"),
    ]
}

/// Resolve the `Managed` directory for a given install root, probing the known layouts
/// and falling back to the platform default when none exist yet.
pub fn managed_dir(game_root: &Path) -> PathBuf {
    let candidates = managed_candidates(game_root);
    candidates
        .iter()
        .find(|p| p.is_dir())
        .cloned()
        .unwrap_or_else(|| {
            candidates
                .into_iter()
                .next()
                .expect("at least one candidate")
        })
}

/// The `Mods` directory under `Managed`.
pub fn mods_dir(game_root: &Path) -> PathBuf {
    managed_dir(game_root).join("Mods")
}

/// The active `Assembly-CSharp.dll` (vanilla or modded API, depending on state).
pub fn assembly_dll(game_root: &Path) -> PathBuf {
    managed_dir(game_root).join("Assembly-CSharp.dll")
}

/// The backup of the vanilla `Assembly-CSharp.dll`, created when the modding API is installed.
pub fn vanilla_backup(game_root: &Path) -> PathBuf {
    managed_dir(game_root).join("Assembly-CSharp.dll.vanilla")
}

/// The stash of the modded `Assembly-CSharp.dll`, created while temporarily running vanilla.
pub fn modded_backup(game_root: &Path) -> PathBuf {
    managed_dir(game_root).join("Assembly-CSharp.dll.modded")
}

/// Marker file recording the installed modding-API version (JSON).
pub fn api_marker(game_root: &Path) -> PathBuf {
    managed_dir(game_root).join("aspid-modding-api.json")
}

/// How a directory was made available at a link location.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkKind {
    /// A real symlink (Unix, or Windows with the privilege available).
    Symlink,
    /// A Windows NTFS directory junction.
    Junction,
    /// A physical copy (cross-device / restricted FS fallback); must be re-synced on switch.
    Copy,
}

/// Make `target` available at `link`, choosing the cheapest mechanism the platform allows.
///
/// `link` must not already exist. The returned [`LinkKind`] records which mechanism was used
/// so callers can know whether a later sync-back is required (for [`LinkKind::Copy`]).
pub fn link_dir(target: &Path, link: &Path) -> Result<LinkKind> {
    if let Some(parent) = link.parent() {
        std::fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
    }

    #[cfg(unix)]
    {
        match std::os::unix::fs::symlink(target, link) {
            Ok(()) => Ok(LinkKind::Symlink),
            Err(e) if is_cross_device(&e) => {
                copy_dir_recursive(target, link).map(|()| LinkKind::Copy)
            }
            Err(e) => Err(Error::io(link, e)),
        }
    }

    #[cfg(windows)]
    {
        match std::os::windows::fs::symlink_dir(target, link) {
            Ok(()) => Ok(LinkKind::Symlink),
            Err(_) => match junction::create(target, link) {
                Ok(()) => Ok(LinkKind::Junction),
                Err(_) => copy_dir_recursive(target, link).map(|()| LinkKind::Copy),
            },
        }
    }

    #[cfg(not(any(unix, windows)))]
    {
        copy_dir_recursive(target, link).map(|()| LinkKind::Copy)
    }
}

/// Remove a link/junction created by [`link_dir`] **without** touching the real target data.
///
/// For [`LinkKind::Copy`] the caller is responsible for syncing changes back first; this
/// removes the copied tree.
pub fn unlink_dir(link: &Path, kind: LinkKind) -> Result<()> {
    let result = match kind {
        // On Unix a symlink (even to a directory) is removed as a file; `remove_dir`
        // would fail with ENOTDIR. On Windows a directory symlink uses `remove_dir`.
        LinkKind::Symlink => {
            #[cfg(unix)]
            {
                std::fs::remove_file(link)
            }
            #[cfg(not(unix))]
            {
                std::fs::remove_dir(link)
            }
        }
        // A junction is a directory reparse point, removed with `remove_dir`.
        LinkKind::Junction => std::fs::remove_dir(link),
        LinkKind::Copy => std::fs::remove_dir_all(link),
    };
    result.map_err(|e| Error::io(link, e))
}

#[cfg(unix)]
fn is_cross_device(e: &std::io::Error) -> bool {
    // EXDEV
    e.raw_os_error() == Some(18)
}

/// Recursively copy a directory tree. Used as the universal fallback for [`link_dir`].
pub fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst).map_err(|e| Error::io(dst, e))?;
    for entry in std::fs::read_dir(src).map_err(|e| Error::io(src, e))? {
        let entry = entry.map_err(|e| Error::io(src, e))?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        let ty = entry.file_type().map_err(|e| Error::io(&from, e))?;
        if ty.is_dir() {
            copy_dir_recursive(&from, &to)?;
        } else {
            std::fs::copy(&from, &to).map_err(|e| Error::io(&from, e))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mods_dir_is_under_managed() {
        let root = Path::new("/games/Hollow Knight");
        let mods = mods_dir(root);
        assert!(mods.ends_with("Managed/Mods") || mods.ends_with("Managed\\Mods"));
    }

    #[test]
    fn link_then_unlink_roundtrips() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("real");
        std::fs::create_dir_all(target.join("nested")).unwrap();
        std::fs::write(target.join("nested").join("a.txt"), b"hello").unwrap();

        let link = tmp.path().join("active");
        let kind = link_dir(&target, &link).unwrap();

        // Content is visible through the link regardless of mechanism.
        assert_eq!(
            std::fs::read(link.join("nested").join("a.txt")).unwrap(),
            b"hello"
        );

        unlink_dir(&link, kind).unwrap();
        assert!(!link.exists());
        // The real data survives.
        assert!(target.join("nested").join("a.txt").exists());
    }
}
