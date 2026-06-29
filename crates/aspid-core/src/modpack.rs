//! Modpacks with completely separate data.
//!
//! Each pack owns its own `Mods/` and Unity `saves/` directories under aspid's data
//! directory (`<data>/packs/<id>/{Mods,saves}`). The **active** pack is surfaced to the
//! game by linking its directories over the live locations — `Managed/Mods` inside the
//! install and the Unity save directory — using [`paths::link_dir`] (symlink on
//! Unix/macOS, junction or copy on Windows). Switching packs just repoints those links,
//! so saves and installed mods are fully isolated per pack. Vanilla is a real pack too.
//!
//! The modding API itself (the patched assemblies in `Managed/`) and skins live outside
//! a pack, so they persist across pack switches.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::game::Install;
use crate::paths::{self, LinkKind};

/// The id of the always-present vanilla pack.
pub const VANILLA_ID: &str = "vanilla";

/// Metadata for one modpack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackMeta {
    /// Stable identifier (folder name).
    pub id: String,
    /// Human-facing name.
    pub name: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct State {
    packs: Vec<PackMeta>,
    active: Option<String>,
    mods_kind: Option<LinkKind>,
    saves_kind: Option<LinkKind>,
}

/// Manages a set of modpacks and which one is active.
pub struct Manager {
    /// Root holding `<id>/Mods` and `<id>/saves` plus `index.json`.
    root: PathBuf,
    /// Live `Managed/Mods` location to link the active pack's mods over.
    mods_link: PathBuf,
    /// Live Unity save directory to link the active pack's saves over.
    saves_link: PathBuf,
    state: State,
}

impl Manager {
    /// Build a manager from explicit paths (used by tests).
    pub fn with_paths(root: PathBuf, mods_link: PathBuf, saves_link: PathBuf) -> Result<Self> {
        let state = load_state(&root)?;
        Ok(Manager {
            root,
            mods_link,
            saves_link,
            state,
        })
    }

    /// Build a manager for a real install, using aspid's data dir and the platform paths.
    pub fn for_install(install: &Install) -> Result<Self> {
        let root = paths::data_dir()?.join("packs");
        Self::with_paths(root, install.mods_dir(), paths::unity_save_dir()?)
    }

    /// All known packs (vanilla is included once initialised).
    pub fn packs(&self) -> &[PackMeta] {
        &self.state.packs
    }

    /// The active pack id, if initialised.
    pub fn active(&self) -> Option<&str> {
        self.state.active.as_deref()
    }

    fn pack_dir(&self, id: &str, mods: bool) -> PathBuf {
        self.root.join(id).join(if mods { "Mods" } else { "saves" })
    }

    fn has_pack(&self, id: &str) -> bool {
        self.state.packs.iter().any(|p| p.id == id)
    }

    fn save(&self) -> Result<()> {
        std::fs::create_dir_all(&self.root).map_err(|e| Error::io(&self.root, e))?;
        let path = self.root.join("index.json");
        let json =
            serde_json::to_string_pretty(&self.state).map_err(|e| Error::Config(e.to_string()))?;
        std::fs::write(&path, json).map_err(|e| Error::io(&path, e))
    }

    /// Initialise pack management on first use: capture the install's current `Mods/` and
    /// save data into a "Default" pack, add an empty "Vanilla" pack, and link the default
    /// pack live. Idempotent.
    pub fn ensure_initialized(&mut self) -> Result<()> {
        if self.state.active.is_some() {
            return Ok(());
        }

        let default = PackMeta {
            id: "default".to_string(),
            name: "Default".to_string(),
        };
        let vanilla = PackMeta {
            id: VANILLA_ID.to_string(),
            name: "Vanilla".to_string(),
        };

        // Capture existing live data into the default pack, then link it.
        let mods_storage = self.pack_dir(&default.id, true);
        let saves_storage = self.pack_dir(&default.id, false);
        let mods_link = self.mods_link.clone();
        let saves_link = self.saves_link.clone();
        self.state.mods_kind = Some(capture_and_link(&mods_link, &mods_storage)?);
        self.state.saves_kind = Some(capture_and_link(&saves_link, &saves_storage)?);

        // Pre-create the vanilla pack's storage so it can be activated later.
        std::fs::create_dir_all(self.pack_dir(&vanilla.id, true))
            .map_err(|e| Error::io(self.pack_dir(&vanilla.id, true), e))?;
        std::fs::create_dir_all(self.pack_dir(&vanilla.id, false))
            .map_err(|e| Error::io(self.pack_dir(&vanilla.id, false), e))?;

        self.state.packs = vec![default.clone(), vanilla];
        self.state.active = Some(default.id);
        self.save()
    }

    /// Create a new empty pack and return its id.
    pub fn create(&mut self, name: &str) -> Result<String> {
        self.ensure_initialized()?;
        let id = self.unique_id(name);
        std::fs::create_dir_all(self.pack_dir(&id, true))
            .map_err(|e| Error::io(self.pack_dir(&id, true), e))?;
        std::fs::create_dir_all(self.pack_dir(&id, false))
            .map_err(|e| Error::io(self.pack_dir(&id, false), e))?;
        self.state.packs.push(PackMeta {
            id: id.clone(),
            name: name.to_string(),
        });
        self.save()?;
        Ok(id)
    }

    /// Clone an existing pack's mods and saves into a new pack.
    pub fn clone_pack(&mut self, src_id: &str, name: &str) -> Result<String> {
        self.ensure_initialized()?;
        if !self.has_pack(src_id) {
            return Err(Error::UnknownDependency(src_id.to_string()));
        }
        let id = self.unique_id(name);
        copy_tree(&self.pack_dir(src_id, true), &self.pack_dir(&id, true))?;
        copy_tree(&self.pack_dir(src_id, false), &self.pack_dir(&id, false))?;
        self.state.packs.push(PackMeta {
            id: id.clone(),
            name: name.to_string(),
        });
        self.save()?;
        Ok(id)
    }

    /// Delete a pack and its data. The active pack and vanilla cannot be deleted.
    pub fn delete(&mut self, id: &str) -> Result<()> {
        if id == VANILLA_ID {
            return Err(Error::Config("the vanilla pack cannot be deleted".into()));
        }
        if self.state.active.as_deref() == Some(id) {
            return Err(Error::Config("cannot delete the active pack".into()));
        }
        if !self.has_pack(id) {
            return Err(Error::UnknownDependency(id.to_string()));
        }
        let dir = self.root.join(id);
        if dir.exists() {
            std::fs::remove_dir_all(&dir).map_err(|e| Error::io(&dir, e))?;
        }
        self.state.packs.retain(|p| p.id != id);
        self.save()
    }

    /// Make `id` the active pack, repointing the live mods/saves links to it.
    pub fn activate(&mut self, id: &str) -> Result<()> {
        self.ensure_initialized()?;
        if !self.has_pack(id) {
            return Err(Error::UnknownDependency(id.to_string()));
        }
        if self.state.active.as_deref() == Some(id) {
            return Ok(());
        }
        let active = self.state.active.clone();

        let mods_link = self.mods_link.clone();
        let new_mods = self.pack_dir(id, true);
        let old_mods = active.as_ref().map(|a| self.pack_dir(a, true));
        self.state.mods_kind = Some(self.relink(
            &mods_link,
            &new_mods,
            old_mods.as_deref(),
            self.state.mods_kind,
        )?);

        let saves_link = self.saves_link.clone();
        let new_saves = self.pack_dir(id, false);
        let old_saves = active.as_ref().map(|a| self.pack_dir(a, false));
        self.state.saves_kind = Some(self.relink(
            &saves_link,
            &new_saves,
            old_saves.as_deref(),
            self.state.saves_kind,
        )?);

        self.state.active = Some(id.to_string());
        self.save()
    }

    /// Detach the current link, then link `new_storage` over `live`.
    fn relink(
        &self,
        live: &Path,
        new_storage: &Path,
        old_storage: Option<&Path>,
        current_kind: Option<LinkKind>,
    ) -> Result<LinkKind> {
        if live.exists() || is_symlink(live) {
            match current_kind {
                Some(LinkKind::Copy) => {
                    // The live dir holds the authoritative data; sync it back first.
                    if let Some(old) = old_storage {
                        let _ = std::fs::remove_dir_all(old);
                        copy_tree(live, old)?;
                    }
                    std::fs::remove_dir_all(live).map_err(|e| Error::io(live, e))?;
                }
                Some(kind) => paths::unlink_dir(live, kind)?,
                None => remove_link_or_dir(live)?,
            }
        }
        std::fs::create_dir_all(new_storage).map_err(|e| Error::io(new_storage, e))?;
        paths::link_dir(new_storage, live)
    }

    fn unique_id(&self, name: &str) -> String {
        let base = slugify(name);
        let base = if base.is_empty() {
            "pack".to_string()
        } else {
            base
        };
        if !self.id_taken(&base) {
            return base;
        }
        let mut n = 2;
        loop {
            let candidate = format!("{base}-{n}");
            if !self.id_taken(&candidate) {
                return candidate;
            }
            n += 1;
        }
    }

    fn id_taken(&self, id: &str) -> bool {
        id == VANILLA_ID || self.has_pack(id) || self.root.join(id).exists()
    }
}

fn load_state(root: &Path) -> Result<State> {
    let path = root.join("index.json");
    match std::fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str(&text).map_err(|e| Error::Config(e.to_string())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(State::default()),
        Err(e) => Err(Error::io(&path, e)),
    }
}

fn slugify(name: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in name.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    out.trim_matches('-').to_string()
}

fn is_symlink(path: &Path) -> bool {
    std::fs::symlink_metadata(path)
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
}

/// Remove a path that may be a symlink or a real directory (used pre-init only).
fn remove_link_or_dir(path: &Path) -> Result<()> {
    if is_symlink(path) {
        // A directory symlink: `remove_file` on Unix, `remove_dir` elsewhere.
        #[cfg(unix)]
        let r = std::fs::remove_file(path);
        #[cfg(not(unix))]
        let r = std::fs::remove_dir(path);
        r.map_err(|e| Error::io(path, e))
    } else if path.is_dir() {
        std::fs::remove_dir_all(path).map_err(|e| Error::io(path, e))
    } else {
        Ok(())
    }
}

/// Move the live directory's contents into `storage`, then link `storage` over `live`.
fn capture_and_link(live: &Path, storage: &Path) -> Result<LinkKind> {
    if is_symlink(live) {
        // Already linked (unexpected pre-init) — detach without touching the target.
        remove_link_or_dir(live)?;
        std::fs::create_dir_all(storage).map_err(|e| Error::io(storage, e))?;
    } else if live.is_dir() {
        if storage.exists() {
            // Storage already populated; discard the (presumably empty) live dir.
            std::fs::remove_dir_all(live).map_err(|e| Error::io(live, e))?;
        } else {
            move_tree(live, storage)?;
        }
    } else {
        std::fs::create_dir_all(storage).map_err(|e| Error::io(storage, e))?;
    }
    paths::link_dir(storage, live)
}

/// Move a directory tree (rename, falling back to copy+remove across filesystems).
fn move_tree(src: &Path, dst: &Path) -> Result<()> {
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
    }
    match std::fs::rename(src, dst) {
        Ok(()) => Ok(()),
        Err(_) => {
            paths::copy_dir_recursive(src, dst)?;
            std::fs::remove_dir_all(src).map_err(|e| Error::io(src, e))
        }
    }
}

/// Copy a directory tree, creating `dst` (and any missing source as an empty dir).
fn copy_tree(src: &Path, dst: &Path) -> Result<()> {
    if !src.exists() {
        return std::fs::create_dir_all(dst).map_err(|e| Error::io(dst, e));
    }
    paths::copy_dir_recursive(src, dst)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Env {
        _tmp: tempfile::TempDir,
        root: PathBuf,
        mods_link: PathBuf,
        saves_link: PathBuf,
    }

    fn setup() -> Env {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path();
        // Simulate a live install with one pre-existing mod and one save file.
        let mods_link = base.join("game/Managed/Mods");
        std::fs::create_dir_all(mods_link.join("ExistingMod")).unwrap();
        std::fs::write(mods_link.join("ExistingMod/ExistingMod.dll"), b"x").unwrap();
        let saves_link = base.join("unity/Hollow Knight");
        std::fs::create_dir_all(&saves_link).unwrap();
        std::fs::write(saves_link.join("user1.dat"), b"save").unwrap();

        Env {
            root: base.join("data/packs"),
            mods_link,
            saves_link,
            _tmp: tmp,
        }
    }

    fn mgr(env: &Env) -> Manager {
        Manager::with_paths(
            env.root.clone(),
            env.mods_link.clone(),
            env.saves_link.clone(),
        )
        .unwrap()
    }

    #[test]
    fn init_captures_existing_data_into_default() {
        let env = setup();
        let mut m = mgr(&env);
        m.ensure_initialized().unwrap();

        assert_eq!(m.active(), Some("default"));
        // Existing mod + save are now reachable through the live links.
        assert!(env.mods_link.join("ExistingMod/ExistingMod.dll").exists());
        assert_eq!(
            std::fs::read(env.saves_link.join("user1.dat")).unwrap(),
            b"save"
        );
        // And physically stored under the default pack.
        assert!(env
            .root
            .join("default/Mods/ExistingMod/ExistingMod.dll")
            .exists());
        // Vanilla pack exists for switching.
        assert!(m.packs().iter().any(|p| p.id == VANILLA_ID));
    }

    #[test]
    fn switching_packs_isolates_mods_and_saves() {
        let env = setup();
        let mut m = mgr(&env);
        m.ensure_initialized().unwrap();

        // Switch to vanilla: live dirs should now be empty (fresh pack).
        m.activate(VANILLA_ID).unwrap();
        assert_eq!(m.active(), Some(VANILLA_ID));
        assert!(!env.mods_link.join("ExistingMod").exists());
        assert!(!env.saves_link.join("user1.dat").exists());

        // Write a vanilla-only save through the live link.
        std::fs::write(env.saves_link.join("vanilla.dat"), b"v").unwrap();

        // Switch back to default: original data returns, vanilla's does not leak in.
        m.activate("default").unwrap();
        assert!(env.mods_link.join("ExistingMod").exists());
        assert!(env.saves_link.join("user1.dat").exists());
        assert!(!env.saves_link.join("vanilla.dat").exists());

        // The vanilla save persisted in its own pack storage.
        assert!(env.root.join("vanilla/saves/vanilla.dat").exists());
    }

    #[test]
    fn create_clone_and_delete() {
        let env = setup();
        let mut m = mgr(&env);
        m.ensure_initialized().unwrap();

        let id = m.create("My Pack!").unwrap();
        assert_eq!(id, "my-pack");
        assert!(env.root.join("my-pack/Mods").is_dir());

        let clone_id = m.clone_pack("default", "Copy").unwrap();
        assert!(env
            .root
            .join(&clone_id)
            .join("Mods/ExistingMod/ExistingMod.dll")
            .exists());

        // Cannot delete active or vanilla.
        assert!(m.delete("default").is_err());
        assert!(m.delete(VANILLA_ID).is_err());

        m.delete(&id).unwrap();
        assert!(!env.root.join("my-pack").exists());
        assert!(!m.packs().iter().any(|p| p.id == id));
    }

    #[test]
    fn state_persists_across_managers() {
        let env = setup();
        {
            let mut m = mgr(&env);
            m.ensure_initialized().unwrap();
            m.create("Second").unwrap();
        }
        let m2 = mgr(&env);
        assert_eq!(m2.active(), Some("default"));
        assert!(m2.packs().iter().any(|p| p.name == "Second"));
    }
}
