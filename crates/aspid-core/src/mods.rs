//! Installing, removing, and inspecting mods, with transitive dependency resolution.
//!
//! Each mod lives in its own folder under `Managed/Mods/<Name>/`. Disabled mods are
//! moved to `Managed/Mods/Disabled/<Name>/`, which the modding API ignores. aspid writes
//! an `aspid-mod.json` marker into each mod folder to track the installed version.

use std::collections::HashSet;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::game::Install;
use crate::modlinks::{Catalog, Mod};
use crate::version;
use crate::{archive, net};

/// Folder name (under `Mods/`) holding disabled mods.
const DISABLED_DIR: &str = "Disabled";
/// Per-mod marker filename.
const MARKER: &str = "aspid-mod.json";

#[derive(Debug, Serialize, Deserialize)]
struct ModMarker {
    name: String,
    version: String,
}

/// An installed mod as seen on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledMod {
    /// Mod name (folder name).
    pub name: String,
    /// Installed version, if a marker was found.
    pub version: Option<String>,
    /// Whether the mod is enabled (in `Mods/`) or disabled (in `Mods/Disabled/`).
    pub enabled: bool,
}

impl InstalledMod {
    /// Whether `catalog_mod` offers a newer version than what's installed.
    pub fn update_available(&self, catalog_mod: &Mod) -> bool {
        match &self.version {
            Some(current) => version::is_newer(&catalog_mod.version, current),
            None => false,
        }
    }
}

fn mod_path(install: &Install, name: &str, enabled: bool) -> PathBuf {
    if enabled {
        install.mods_dir().join(name)
    } else {
        install.mods_dir().join(DISABLED_DIR).join(name)
    }
}

fn read_marker(dir: &std::path::Path) -> Option<String> {
    let text = std::fs::read_to_string(dir.join(MARKER)).ok()?;
    serde_json::from_str::<ModMarker>(&text)
        .ok()
        .map(|m| m.version)
}

fn scan_dir(dir: &std::path::Path, enabled: bool, out: &mut Vec<InstalledMod>) -> Result<()> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(Error::io(dir, e)),
    };
    for entry in entries {
        let entry = entry.map_err(|e| Error::io(dir, e))?;
        if !entry.file_type().map_err(|e| Error::io(dir, e))?.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if enabled && name == DISABLED_DIR {
            continue; // the Disabled container, not a mod
        }
        out.push(InstalledMod {
            version: read_marker(&entry.path()),
            name,
            enabled,
        });
    }
    Ok(())
}

/// List all installed mods (enabled and disabled).
pub fn list_installed(install: &Install) -> Result<Vec<InstalledMod>> {
    let mods_dir = install.mods_dir();
    let mut out = Vec::new();
    scan_dir(&mods_dir, true, &mut out)?;
    scan_dir(&mods_dir.join(DISABLED_DIR), false, &mut out)?;
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// Whether a mod is installed (enabled or disabled).
pub fn is_installed(install: &Install, name: &str) -> bool {
    mod_path(install, name, true).is_dir() || mod_path(install, name, false).is_dir()
}

/// Resolve the transitive install order for `name`: every dependency appears before the
/// mod that needs it, with `name` itself last. Errors on an unknown mod/dependency.
pub fn resolve_install_order(catalog: &Catalog, name: &str) -> Result<Vec<String>> {
    let mut order = Vec::new();
    let mut visited = HashSet::new();
    let mut on_stack = HashSet::new();
    visit(catalog, name, &mut order, &mut visited, &mut on_stack)?;
    Ok(order)
}

fn visit(
    catalog: &Catalog,
    name: &str,
    order: &mut Vec<String>,
    visited: &mut HashSet<String>,
    on_stack: &mut HashSet<String>,
) -> Result<()> {
    if visited.contains(name) {
        return Ok(());
    }
    if !on_stack.insert(name.to_string()) {
        return Ok(()); // cycle guard — already being processed
    }
    let m = catalog
        .get(name)
        .ok_or_else(|| Error::UnknownDependency(name.to_string()))?;
    for dep in &m.dependencies {
        visit(catalog, dep, order, visited, on_stack)?;
    }
    on_stack.remove(name);
    visited.insert(name.to_string());
    order.push(name.to_string());
    Ok(())
}

/// Installed mods that depend on `name` (the reverse-dependency warning set).
pub fn installed_dependents(
    install: &Install,
    catalog: &Catalog,
    name: &str,
) -> Result<Vec<String>> {
    let installed = list_installed(install)?;
    let mut dependents = Vec::new();
    for im in installed {
        if im.name == name {
            continue;
        }
        if let Some(m) = catalog.get(&im.name) {
            if m.dependencies.iter().any(|d| d == name) {
                dependents.push(im.name);
            }
        }
    }
    Ok(dependents)
}

/// Place an already-downloaded mod archive into `Mods/<name>/` and write its marker.
/// Exposed (crate-internal) so tests can install without the network.
fn place_mod(install: &Install, m: &Mod, bytes: &[u8]) -> Result<()> {
    let dir = mod_path(install, &m.name, true);
    // Replace any previous copy (enabled or disabled) for a clean install/update.
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(mod_path(install, &m.name, false));

    archive::extract_all(bytes, &dir)?;

    let marker = ModMarker {
        name: m.name.clone(),
        version: m.version.clone(),
    };
    let json = serde_json::to_string_pretty(&marker).map_err(|e| Error::Config(e.to_string()))?;
    std::fs::write(dir.join(MARKER), json).map_err(|e| Error::io(dir.join(MARKER), e))?;
    Ok(())
}

/// Download and install a single catalog mod (no dependency handling).
pub async fn install_one(install: &Install, m: &Mod) -> Result<()> {
    let link = m
        .link
        .current()
        .ok_or_else(|| Error::NoDownloadForPlatform {
            what: m.name.clone(),
        })?;
    let bytes = net::download_verified(&link.url, &link.sha256).await?;
    place_mod(install, m, &bytes)
}

/// Install a mod and all of its missing dependencies, in dependency order.
///
/// Returns the names actually installed (skipping ones already present at the same or a
/// newer version). `force_reinstall` ignores the skip check for the named mod's chain.
pub async fn install_with_dependencies(
    install: &Install,
    catalog: &Catalog,
    name: &str,
) -> Result<Vec<String>> {
    let order = resolve_install_order(catalog, name)?;
    let mut installed_now = Vec::new();
    for dep_name in order {
        let m = catalog
            .get(&dep_name)
            .ok_or_else(|| Error::UnknownDependency(dep_name.clone()))?;
        // Skip if already installed at >= the catalog version.
        if let Some(existing) = current_installed_version(install, &dep_name) {
            if !version::is_newer(&m.version, &existing) {
                continue;
            }
        }
        install_one(install, m).await?;
        installed_now.push(dep_name);
    }
    Ok(installed_now)
}

fn current_installed_version(install: &Install, name: &str) -> Option<String> {
    for enabled in [true, false] {
        let dir = mod_path(install, name, enabled);
        if dir.is_dir() {
            return read_marker(&dir).or(Some(String::new()));
        }
    }
    None
}

/// Remove an installed mod (whether enabled or disabled). Does **not** check for
/// dependents — callers should warn using [`installed_dependents`] first.
pub fn remove(install: &Install, name: &str) -> Result<()> {
    let mut removed = false;
    for enabled in [true, false] {
        let dir = mod_path(install, name, enabled);
        if dir.is_dir() {
            std::fs::remove_dir_all(&dir).map_err(|e| Error::io(&dir, e))?;
            removed = true;
        }
    }
    if removed {
        Ok(())
    } else {
        Err(Error::UnknownDependency(name.to_string()))
    }
}

/// Enable or disable an installed mod by moving it between `Mods/` and `Mods/Disabled/`.
pub fn set_enabled(install: &Install, name: &str, enabled: bool) -> Result<()> {
    let from = mod_path(install, name, !enabled);
    let to = mod_path(install, name, enabled);
    if !from.is_dir() {
        // Already in the desired state, or not installed.
        return if to.is_dir() {
            Ok(())
        } else {
            Err(Error::UnknownDependency(name.to_string()))
        };
    }
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
    }
    std::fs::rename(&from, &to).map_err(|e| Error::io(&to, e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game;
    use crate::modlinks::{DownloadLink, ModLink};
    use std::io::{Cursor, Write};

    fn fake_install() -> (tempfile::TempDir, Install) {
        let tmp = tempfile::tempdir().unwrap();
        let managed = tmp.path().join("hollow_knight_Data").join("Managed");
        std::fs::create_dir_all(&managed).unwrap();
        std::fs::write(managed.join("UnityEngine.dll"), b"unity").unwrap();
        std::fs::write(managed.join("Assembly-CSharp.dll"), b"asm").unwrap();
        let install = game::validate(tmp.path()).unwrap();
        (tmp, install)
    }

    fn mk_mod(name: &str, version: &str, deps: &[&str]) -> Mod {
        Mod {
            name: name.to_string(),
            description: String::new(),
            version: version.to_string(),
            link: ModLink::Universal(DownloadLink {
                url: "x".into(),
                sha256: "y".into(),
            }),
            dependencies: deps.iter().map(|s| s.to_string()).collect(),
            repository: None,
            tags: vec![],
            authors: vec![],
            integrations: vec![],
        }
    }

    fn dll_zip(name: &str) -> Vec<u8> {
        let mut buf = Vec::new();
        {
            let mut w = zip::ZipWriter::new(Cursor::new(&mut buf));
            let opts: zip::write::FileOptions<()> = zip::write::FileOptions::default();
            w.start_file(format!("{name}.dll"), opts).unwrap();
            w.write_all(b"dll").unwrap();
            w.finish().unwrap();
        }
        buf
    }

    #[test]
    fn resolves_transitive_dependencies_in_order() {
        let catalog = Catalog::new(vec![
            mk_mod("PaleCourt", "1.0.0.0", &["SFCore", "Vasi"]),
            mk_mod("SFCore", "1.0.0.0", &["Vasi"]),
            mk_mod("Vasi", "1.0.0.0", &[]),
        ]);
        let order = resolve_install_order(&catalog, "PaleCourt").unwrap();
        // Vasi before SFCore before PaleCourt.
        let pos = |n: &str| order.iter().position(|x| x == n).unwrap();
        assert!(pos("Vasi") < pos("SFCore"));
        assert!(pos("SFCore") < pos("PaleCourt"));
        assert_eq!(order.last().unwrap(), "PaleCourt");
    }

    #[test]
    fn unknown_dependency_errors() {
        let catalog = Catalog::new(vec![mk_mod("A", "1.0.0.0", &["Missing"])]);
        assert!(matches!(
            resolve_install_order(&catalog, "A"),
            Err(Error::UnknownDependency(_))
        ));
    }

    #[test]
    fn place_install_list_and_remove() {
        let (_tmp, install) = fake_install();
        let m = mk_mod("Satchel", "1.2.3.4", &[]);
        place_mod(&install, &m, &dll_zip("Satchel")).unwrap();

        assert!(is_installed(&install, "Satchel"));
        let installed = list_installed(&install).unwrap();
        assert_eq!(installed.len(), 1);
        assert_eq!(installed[0].name, "Satchel");
        assert_eq!(installed[0].version.as_deref(), Some("1.2.3.4"));
        assert!(installed[0].enabled);

        remove(&install, "Satchel").unwrap();
        assert!(!is_installed(&install, "Satchel"));
    }

    #[test]
    fn enable_disable_moves_between_folders() {
        let (_tmp, install) = fake_install();
        place_mod(
            &install,
            &mk_mod("Satchel", "1.0.0.0", &[]),
            &dll_zip("Satchel"),
        )
        .unwrap();

        set_enabled(&install, "Satchel", false).unwrap();
        let installed = list_installed(&install).unwrap();
        assert!(!installed[0].enabled);

        set_enabled(&install, "Satchel", true).unwrap();
        assert!(list_installed(&install).unwrap()[0].enabled);
    }

    #[test]
    fn dependents_are_detected() {
        let (_tmp, install) = fake_install();
        let catalog = Catalog::new(vec![
            mk_mod("PaleCourt", "1.0.0.0", &["SFCore"]),
            mk_mod("SFCore", "1.0.0.0", &[]),
        ]);
        place_mod(&install, catalog.get("SFCore").unwrap(), &dll_zip("SFCore")).unwrap();
        place_mod(
            &install,
            catalog.get("PaleCourt").unwrap(),
            &dll_zip("PaleCourt"),
        )
        .unwrap();

        let deps = installed_dependents(&install, &catalog, "SFCore").unwrap();
        assert_eq!(deps, vec!["PaleCourt"]);
    }

    #[test]
    fn update_available_uses_marker_version() {
        let installed = InstalledMod {
            name: "Satchel".into(),
            version: Some("1.0.0.0".into()),
            enabled: true,
        };
        assert!(installed.update_available(&mk_mod("Satchel", "1.1.0.0", &[])));
        assert!(!installed.update_available(&mk_mod("Satchel", "1.0.0.0", &[])));
    }
}
