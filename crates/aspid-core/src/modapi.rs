//! Installing, updating, and toggling the Hollow Knight modding API.
//!
//! The modding API replaces `Managed/Assembly-CSharp.dll` with a patched build. We keep
//! the pristine assembly as `Assembly-CSharp.dll.vanilla` so the API can be uninstalled,
//! and — when the user wants a one-off vanilla launch — stash the modded assembly as
//! `Assembly-CSharp.dll.modded` while the vanilla one is active. See
//! [`crate::game::detect_api_state`] for how these sidecars map to [`ApiState`].

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::game::{detect_api_state, ApiState, Install};
use crate::modlinks::ApiManifest;
use crate::version;
use crate::{archive, net, paths};

#[derive(Debug, Serialize, Deserialize)]
struct ApiMarker {
    version: String,
}

fn copy(from: &std::path::Path, to: &std::path::Path) -> Result<()> {
    std::fs::copy(from, to)
        .map(|_| ())
        .map_err(|e| Error::io(to, e))
}

fn remove_if_exists(path: &std::path::Path) -> Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(Error::io(path, e)),
    }
}

/// The modding-API version currently installed, if any (read from the marker file).
pub fn installed_version(install: &Install) -> Option<String> {
    let text = std::fs::read_to_string(paths::api_marker(&install.root)).ok()?;
    serde_json::from_str::<ApiMarker>(&text)
        .ok()
        .map(|m| m.version)
}

/// Whether the catalog API manifest is newer than what's installed.
pub fn update_available(install: &Install, manifest: &ApiManifest) -> bool {
    match installed_version(install) {
        Some(current) => version::is_newer(&manifest.version, &current),
        None => false,
    }
}

/// Download and install (or update) the modding API, leaving it active (modded).
pub async fn install(install: &Install, manifest: &ApiManifest) -> Result<String> {
    let link = manifest
        .current_link()
        .ok_or_else(|| Error::NoDownloadForPlatform {
            what: "modding API".to_string(),
        })?;
    let bytes = net::download_verified(&link.url, &link.sha256).await?;

    let managed = &install.managed;
    let assembly = install.assembly_dll();
    let vanilla_backup = install.vanilla_backup();

    // Preserve the pristine assembly the first time the API is installed.
    if !vanilla_backup.exists() && assembly.exists() {
        copy(&assembly, &vanilla_backup)?;
    }

    // Drop the API files into Managed/. The manifest lists exactly which files belong;
    // if (defensively) it's empty, fall back to extracting everything.
    let allow: HashSet<&str> = manifest.files.iter().map(String::as_str).collect();
    archive::extract_flat(&bytes, managed, |name| {
        allow.is_empty() || allow.contains(name)
    })?;

    // We are now freshly modded and active: clear any vanilla-launch stash.
    remove_if_exists(&paths::modded_backup(&install.root))?;

    let marker = ApiMarker {
        version: manifest.version.clone(),
    };
    let marker_json =
        serde_json::to_string_pretty(&marker).map_err(|e| Error::Config(e.to_string()))?;
    std::fs::write(paths::api_marker(&install.root), marker_json)
        .map_err(|e| Error::io(paths::api_marker(&install.root), e))?;

    Ok(manifest.version.clone())
}

/// Switch the install to vanilla for a vanilla launch, keeping the modded assembly stashed.
///
/// No-op if already running vanilla. Errors if the API is not installed.
pub fn disable_for_vanilla(install: &Install) -> Result<()> {
    match detect_api_state(&install.root) {
        ApiState::DisabledForVanilla => Ok(()),
        ApiState::Installed => {
            let assembly = install.assembly_dll();
            let modded_backup = paths::modded_backup(&install.root);
            let vanilla_backup = install.vanilla_backup();
            // Stash the modded assembly, then activate vanilla.
            copy(&assembly, &modded_backup)?;
            copy(&vanilla_backup, &assembly)?;
            Ok(())
        }
        ApiState::NotInstalled | ApiState::Missing => Err(Error::ApiNotInstalled),
    }
}

/// Re-activate the modding API after a vanilla launch.
///
/// No-op if already modded. Errors if the API is not installed.
pub fn enable_modded(install: &Install) -> Result<()> {
    match detect_api_state(&install.root) {
        ApiState::Installed => Ok(()),
        ApiState::DisabledForVanilla => {
            let assembly = install.assembly_dll();
            let modded_backup = paths::modded_backup(&install.root);
            copy(&modded_backup, &assembly)?;
            remove_if_exists(&modded_backup)?;
            Ok(())
        }
        ApiState::NotInstalled | ApiState::Missing => Err(Error::ApiNotInstalled),
    }
}

/// Fully uninstall the modding API, restoring the vanilla assembly.
pub fn uninstall(install: &Install) -> Result<()> {
    let assembly = install.assembly_dll();
    let vanilla_backup = install.vanilla_backup();
    if vanilla_backup.exists() {
        copy(&vanilla_backup, &assembly)?;
        remove_if_exists(&vanilla_backup)?;
    }
    remove_if_exists(&paths::modded_backup(&install.root))?;
    remove_if_exists(&paths::api_marker(&install.root))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game;

    /// Create a fake vanilla install and return the validated handle.
    fn fake_install() -> (tempfile::TempDir, Install) {
        let tmp = tempfile::tempdir().unwrap();
        let managed = tmp.path().join("hollow_knight_Data").join("Managed");
        std::fs::create_dir_all(&managed).unwrap();
        std::fs::write(managed.join("UnityEngine.dll"), b"unity").unwrap();
        std::fs::write(managed.join("Assembly-CSharp.dll"), b"VANILLA").unwrap();
        let install = game::validate(tmp.path()).unwrap();
        (tmp, install)
    }

    /// Simulate `install()` without network by laying down the modded assembly + marker.
    fn fake_api_install(install: &Install, version: &str) {
        let assembly = install.assembly_dll();
        copy(&assembly, &install.vanilla_backup()).unwrap();
        std::fs::write(&assembly, b"MODDED").unwrap();
        std::fs::write(
            paths::api_marker(&install.root),
            serde_json::to_string(&ApiMarker {
                version: version.to_string(),
            })
            .unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn toggle_vanilla_roundtrip_preserves_assemblies() {
        let (_tmp, install) = fake_install();
        fake_api_install(&install, "77");
        assert_eq!(install.api_state(), ApiState::Installed);

        disable_for_vanilla(&install).unwrap();
        assert_eq!(install.api_state(), ApiState::DisabledForVanilla);
        assert_eq!(std::fs::read(install.assembly_dll()).unwrap(), b"VANILLA");

        enable_modded(&install).unwrap();
        assert_eq!(install.api_state(), ApiState::Installed);
        assert_eq!(std::fs::read(install.assembly_dll()).unwrap(), b"MODDED");
    }

    #[test]
    fn uninstall_restores_vanilla() {
        let (_tmp, install) = fake_install();
        fake_api_install(&install, "77");
        uninstall(&install).unwrap();
        assert_eq!(install.api_state(), ApiState::NotInstalled);
        assert_eq!(std::fs::read(install.assembly_dll()).unwrap(), b"VANILLA");
        assert!(installed_version(&install).is_none());
    }

    #[test]
    fn update_available_compares_versions() {
        let (_tmp, install) = fake_install();
        fake_api_install(&install, "77");
        let mut manifest = ApiManifest {
            version: "78".to_string(),
            link: crate::modlinks::ModLink::Universal(crate::modlinks::DownloadLink {
                url: "x".into(),
                sha256: "y".into(),
            }),
            files: vec![],
        };
        assert!(update_available(&install, &manifest));
        manifest.version = "77".to_string();
        assert!(!update_available(&install, &manifest));
    }

    #[test]
    fn toggle_without_api_errors() {
        let (_tmp, install) = fake_install();
        assert!(matches!(
            disable_for_vanilla(&install),
            Err(Error::ApiNotInstalled)
        ));
    }
}
