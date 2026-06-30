//! Cosmetic skin management for Custom Knight (and Enemy HP Bar) skins.
//!
//! Skins are kept in a central library under `<data>/skins/<kind>/<SkinName>/` so that a
//! user's skin collection and selection **persist across modpacks** (modpacks isolate
//! `Mods/` and saves, but the skin library lives outside them). To make skins available
//! in-game, [`SkinStore::sync_to_game`] copies the library into the active install's
//! `Mods/<Mod>/Skins/` directory; call it after switching packs so the active pack sees
//! the same skins.
//!
//! Note: the in-game *selected* skin is stored by Custom Knight in its own settings (which
//! live in the per-pack save data). aspid remembers the chosen skin in its config
//! ([`crate::config::Config::active_skins`]) and keeps it in sync with the game both ways:
//! [`read_active_skin`] mirrors in-game changes back into the app, and [`write_active_skin`]
//! applies the app's choice to Custom Knight (effective on the next launch). The catalog is
//! sourced from
//! [HKSkins](https://hkskins.art) (see [`fetch_catalog`]); since most skins are hosted
//! externally, importing a downloaded archive is done with [`SkinStore::import_zip`].

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::game::Install;
use crate::{archive, net, paths};

/// A category of cosmetic skin, identifying the mod it belongs to and where its skins live.
///
/// The mod is located by its assembly file ([`dll`](Self::dll)) rather than a fixed folder
/// name, because the install folder is the mod's ModLinks name (e.g. `Custom Knight`, with
/// a space) which differs from the assembly name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkinKind {
    /// Stable id used in config and the library path.
    pub id: &'static str,
    /// Display name.
    pub label: &'static str,
    /// The mod's assembly file name, used to locate its install folder.
    pub dll: &'static str,
    /// The skins subdirectory within the mod folder.
    pub skins_subdir: &'static str,
    /// The default skin folder the mod expects (created so skins work before first launch).
    pub default_skin: &'static str,
}

/// Custom Knight — player/knight skins.
pub const CUSTOM_KNIGHT: SkinKind = SkinKind {
    id: "customknight",
    label: "Custom Knight",
    dll: "CustomKnight.dll",
    skins_subdir: "Skins",
    default_skin: "Default",
};

/// Enemy HP Bar — skinnable health bars for enemies and bosses. The mod ships as
/// `EnemyHPBar.dll` under its ModLinks folder "Enemy HP Bar" and loads standalone skins from
/// a `CustomHPBar/<skin>/` directory beside the assembly.
pub const ENEMY_HP_BAR: SkinKind = SkinKind {
    id: "enemyhpbar",
    label: "Enemy HP Bar",
    dll: "EnemyHPBar.dll",
    skins_subdir: "CustomHPBar",
    default_skin: "Default",
};

/// All skin kinds aspid knows about.
pub const ALL_KINDS: [SkinKind; 2] = [CUSTOM_KNIGHT, ENEMY_HP_BAR];

/// Locate the install folder of a skin kind's mod by finding the folder (enabled or
/// disabled, under any name) that contains its assembly. CustomKnight loads skins from
/// *its own* directory, so this is where skins must go.
pub fn find_mod_dir(install: &Install, kind: SkinKind) -> Option<PathBuf> {
    let mods = install.mods_dir();
    for base in [mods.clone(), mods.join("Disabled")] {
        let Ok(entries) = std::fs::read_dir(&base) else {
            continue;
        };
        for entry in entries.flatten() {
            let dir = entry.path();
            if dir.is_dir() && dir.join(kind.dll).is_file() {
                return Some(dir);
            }
        }
    }
    None
}

/// The live `Skins/` directory for a kind in the active install, if the mod is installed.
pub fn game_skins_dir(install: &Install, kind: SkinKind) -> Option<PathBuf> {
    find_mod_dir(install, kind).map(|d| d.join(kind.skins_subdir))
}

/// Whether the mod backing a skin kind is installed (enabled or disabled) in the active pack.
pub fn is_mod_installed(install: &Install, kind: SkinKind) -> bool {
    find_mod_dir(install, kind).is_some()
}

/// Custom Knight persists the selected skin in its Modding-API global settings file, named
/// after the mod class (`CustomKnight`), in the Unity save directory. The `DefaultSkin`
/// field holds the skin's *id*, which for on-disk skins is the skin folder name.
#[derive(serde::Deserialize)]
struct CkGlobalSettings {
    #[serde(rename = "DefaultSkin", default)]
    default_skin: Option<String>,
}

/// Read the Custom Knight skin id currently selected in-game from the global settings file
/// in `save_dir`. Returns the raw selection (which may be the built-in `Default`), or `None`
/// only when the file is absent/unparseable — so callers can tell "no custom skin selected"
/// (`Some("Default")`) apart from "couldn't read" (`None`, leave existing state untouched).
pub fn read_active_skin(save_dir: &Path) -> Option<String> {
    let file = save_dir.join("CustomKnight.GlobalSettings.json");
    let text = std::fs::read_to_string(file).ok()?;
    let settings: CkGlobalSettings = serde_json::from_str(&text).ok()?;
    let name = settings.default_skin?;
    (!name.is_empty()).then_some(name)
}

/// Read the Custom Knight skin selected in-game for the active install's save data
/// (Proton-aware). See [`read_active_skin`] for the `None`-vs-`Default` distinction.
pub fn active_skin_in_game(install: &Install) -> Option<String> {
    let save_dir = paths::unity_save_dir_for(&install.root).ok()?;
    read_active_skin(&save_dir)
}

/// Read-modify-write a single key in a Modding-API global settings file (named `filename`)
/// in `save_dir`, preserving every other setting and creating the file/dir if absent.
fn set_global_setting(
    save_dir: &Path,
    filename: &str,
    key: &str,
    val: serde_json::Value,
) -> Result<()> {
    let file = save_dir.join(filename);
    let mut value = match std::fs::read_to_string(&file) {
        Ok(text) => serde_json::from_str::<serde_json::Value>(&text)
            .unwrap_or_else(|_| serde_json::json!({})),
        Err(_) => serde_json::json!({}),
    };
    if !value.is_object() {
        value = serde_json::json!({});
    }
    value[key] = val;

    std::fs::create_dir_all(save_dir).map_err(|e| Error::io(save_dir, e))?;
    let text = serde_json::to_string_pretty(&value).map_err(|e| Error::Config(e.to_string()))?;
    std::fs::write(&file, text).map_err(|e| Error::io(&file, e))
}

/// Set the Custom Knight skin selection (its `DefaultSkin`) in the global settings file in
/// `save_dir`, preserving every other setting. Creates the file (and directory) if absent.
///
/// Custom Knight reads global settings when the game loads, so a change made while the game
/// is running takes effect on the next launch (and may be overwritten by the running game
/// on save); applied with the game closed, the next launch starts on the chosen skin.
pub fn write_active_skin(save_dir: &Path, skin: &str) -> Result<()> {
    set_global_setting(
        save_dir,
        "CustomKnight.GlobalSettings.json",
        "DefaultSkin",
        serde_json::Value::String(skin.to_string()),
    )
}

/// Set the in-game Custom Knight skin for the active install's save data (Proton-aware).
pub fn set_active_skin_in_game(install: &Install, skin: &str) -> Result<()> {
    let save_dir = paths::unity_save_dir_for(&install.root)?;
    write_active_skin(&save_dir, skin)
}

/// Whether Enemy HP Bar's Custom Knight integration applies: it only does anything when both
/// that mod and Custom Knight are installed, in which case HP-bar art is read from the
/// active Custom Knight skin's `HPBar/` subfolder instead of the standalone `CustomHPBar/`.
pub fn hp_bar_integration_available(install: &Install) -> bool {
    is_mod_installed(install, CUSTOM_KNIGHT) && is_mod_installed(install, ENEMY_HP_BAR)
}

/// Set Enemy HP Bar's `Intergration` flag (the mod's own spelling) in its global settings
/// file in `save_dir`, preserving every other setting.
pub fn write_hp_bar_integration(save_dir: &Path, enabled: bool) -> Result<()> {
    set_global_setting(
        save_dir,
        "EnemyHPBar.GlobalSettings.json",
        "Intergration",
        serde_json::Value::Bool(enabled),
    )
}

/// Enable Enemy HP Bar's Custom Knight integration for the active install when both mods are
/// installed, so HP bars follow the active Custom Knight skin. Returns whether integration
/// is now in effect (i.e. both mods are present).
pub fn ensure_hp_bar_integration(install: &Install) -> Result<bool> {
    if !hp_bar_integration_available(install) {
        return Ok(false);
    }
    let save_dir = paths::unity_save_dir_for(&install.root)?;
    write_hp_bar_integration(&save_dir, true)?;
    Ok(true)
}

/// Create the mod's `Skins/` and `Skins/<Default>/` directories so skins can be installed
/// without launching the game first (the mod normally creates these on first run). Returns
/// the `Skins/` path. Errors if the mod is not installed.
pub fn prepare_skins_dir(install: &Install, kind: SkinKind) -> Result<PathBuf> {
    let skins = game_skins_dir(install, kind)
        .ok_or_else(|| Error::Config(format!("{} is not installed", kind.label)))?;
    let default = skins.join(kind.default_skin);
    std::fs::create_dir_all(&default).map_err(|e| Error::io(&default, e))?;
    Ok(skins)
}

/// The central, cross-pack skin library.
#[derive(Debug, Clone)]
pub struct SkinStore {
    root: PathBuf,
}

impl SkinStore {
    /// Open the default library under aspid's data directory.
    pub fn open() -> Result<Self> {
        Ok(Self::with_root(paths::data_dir()?.join("skins")))
    }

    /// Build a store at an explicit root (used by tests).
    pub fn with_root(root: PathBuf) -> Self {
        SkinStore { root }
    }

    fn kind_dir(&self, kind: SkinKind) -> PathBuf {
        self.root.join(kind.id)
    }

    /// List the skin names stored for a kind.
    pub fn list(&self, kind: SkinKind) -> Result<Vec<String>> {
        let dir = self.kind_dir(kind);
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(Error::io(&dir, e)),
        };
        let mut names = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| Error::io(&dir, e))?;
            if entry.file_type().map_err(|e| Error::io(&dir, e))?.is_dir() {
                names.push(entry.file_name().to_string_lossy().into_owned());
            }
        }
        names.sort();
        Ok(names)
    }

    /// Import a skin folder from disk into the library, returning the stored skin name.
    pub fn import_dir(&self, kind: SkinKind, src: &Path, name: Option<&str>) -> Result<String> {
        let name = name
            .map(str::to_string)
            .or_else(|| src.file_name().map(|n| n.to_string_lossy().into_owned()))
            .ok_or_else(|| Error::Config("could not determine skin name".into()))?;
        let dest = self.kind_dir(kind).join(&name);
        replace_dir_with_copy(src, &dest)?;
        Ok(name)
    }

    /// Import a skin from a zip archive into the library, returning the stored skin name.
    ///
    /// If the archive has a single top-level directory, that becomes the skin; otherwise
    /// the files are placed under a folder named `fallback_name`.
    pub fn import_zip(&self, kind: SkinKind, bytes: &[u8], fallback_name: &str) -> Result<String> {
        // Extract to a temp area under the kind dir, then settle into the final name.
        let staging = self.kind_dir(kind).join(".staging");
        let _ = std::fs::remove_dir_all(&staging);
        let written = archive::extract_all(bytes, &staging)?;

        let top = single_top_dir(&written);
        let result = (|| {
            let (name, src) = match &top {
                Some(dir) => (dir.clone(), staging.join(dir)),
                None => (fallback_name.to_string(), staging.clone()),
            };
            let dest = self.kind_dir(kind).join(&name);
            replace_dir_with_copy(&src, &dest)?;
            Ok(name)
        })();
        let _ = std::fs::remove_dir_all(&staging);
        result
    }

    /// Remove a skin from the library.
    pub fn remove(&self, kind: SkinKind, name: &str) -> Result<()> {
        let dir = self.kind_dir(kind).join(name);
        if dir.is_dir() {
            std::fs::remove_dir_all(&dir).map_err(|e| Error::io(&dir, e))?;
            Ok(())
        } else {
            Err(Error::UnknownDependency(name.to_string()))
        }
    }

    /// Copy every library skin of a kind into the live game skins directory. Returns the
    /// number of skins synced. The mod for the kind must be installed. Also creates the
    /// `Skins/` and `Skins/<Default>/` structure the mod expects.
    pub fn sync_to_game(&self, install: &Install, kind: SkinKind) -> Result<usize> {
        let target = prepare_skins_dir(install, kind)?;
        let mut count = 0;
        for name in self.list(kind)? {
            let src = self.kind_dir(kind).join(&name);
            let dest = target.join(&name);
            replace_dir_with_copy(&src, &dest)?;
            count += 1;
        }
        Ok(count)
    }
}

// ---- Catalog (HKSkins) -------------------------------------------------------

/// How long a fetched HKSkins catalog stays fresh on disk.
const CATALOG_TTL: std::time::Duration = std::time::Duration::from_secs(60 * 60 * 24);

/// A skin listed on [HKSkins](https://hkskins.art).
///
/// HKSkins links to externally-hosted downloads (Discord, Google Drive, Ko-fi, …) rather
/// than direct files, so [`source`](Self::source) is usually a page to open in a browser.
/// When it is a direct `.zip`, aspid can download it straight into the library.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HkSkin {
    /// Skin name.
    pub name: String,
    /// Author credit.
    #[serde(default)]
    pub author: String,
    /// Components the skin covers (e.g. "Knight, Sprint, HUD").
    #[serde(default)]
    pub desc: String,
    /// Where to obtain the skin (often an external page).
    #[serde(default)]
    pub source: String,
    /// Date the skin was added to HKSkins.
    #[serde(default)]
    pub date_added: String,
    /// Local path to the cached `preview.png`, if extracted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview: Option<PathBuf>,
}

impl HkSkin {
    /// Whether [`source`](Self::source) points directly at a downloadable zip.
    pub fn is_direct_zip(&self) -> bool {
        self.source
            .split('?')
            .next()
            .unwrap_or(&self.source)
            .ends_with(".zip")
    }

    /// Whether aspid can download this skin automatically (a direct zip, or a Pingvin
    /// share on skins.hk-modding.org).
    pub fn is_auto_downloadable(&self) -> bool {
        self.is_direct_zip() || pingvin_share_id(&self.source).is_some()
    }
}

/// Raw HKSkins `metadata.json` shape.
#[derive(Debug, Deserialize)]
struct RawMeta {
    name: String,
    #[serde(default)]
    author: String,
    #[serde(default)]
    game: String,
    #[serde(default)]
    desc: String,
    #[serde(default)]
    source: String,
    #[serde(default, rename = "dateAdded")]
    date_added: String,
}

/// Make a folder name safe to use as a single filesystem path component.
fn sanitize(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            other => other,
        })
        .collect()
}

/// Parse the HKSkins `skins.zip` bulk export into Hollow Knight skin entries. When
/// `preview_dir` is given, each skin's `preview.png` is extracted there (named after its
/// folder) and referenced from [`HkSkin::preview`].
pub fn parse_catalog(bytes: &[u8], preview_dir: Option<&Path>) -> Result<Vec<HkSkin>> {
    use std::collections::HashMap;

    if let Some(dir) = preview_dir {
        std::fs::create_dir_all(dir).map_err(|e| Error::io(dir, e))?;
    }

    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes))?;
    let mut metas: HashMap<String, RawMeta> = HashMap::new();

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let Some(path) = entry.enclosed_name() else {
            continue;
        };
        let folder = match path.parent().and_then(|p| p.file_name()) {
            Some(f) => f.to_string_lossy().into_owned(),
            None => continue,
        };
        match path.file_name().and_then(|n| n.to_str()) {
            Some("metadata.json") => {
                let mut text = String::new();
                if std::io::Read::read_to_string(&mut entry, &mut text).is_ok() {
                    if let Ok(m) = serde_json::from_str::<RawMeta>(&text) {
                        metas.insert(folder, m);
                    }
                }
            }
            Some("preview.png") => {
                if let Some(dir) = preview_dir {
                    let mut buf = Vec::new();
                    if std::io::Read::read_to_end(&mut entry, &mut buf).is_ok() {
                        let out = dir.join(format!("{}.png", sanitize(&folder)));
                        let _ = std::fs::write(out, buf);
                    }
                }
            }
            _ => {}
        }
    }

    let mut out: Vec<HkSkin> = metas
        .into_iter()
        .filter(|(_, m)| m.game == "hollowKnight")
        .map(|(folder, m)| {
            let preview = preview_dir.and_then(|dir| {
                let p = dir.join(format!("{}.png", sanitize(&folder)));
                p.exists().then_some(p)
            });
            HkSkin {
                name: m.name,
                author: m.author,
                desc: m.desc,
                source: m.source,
                date_added: m.date_added,
                preview,
            }
        })
        .collect();
    out.sort_by_key(|a| a.name.to_lowercase());
    Ok(out)
}

/// Parse metadata only (no preview extraction).
pub fn parse_catalog_zip(bytes: &[u8]) -> Result<Vec<HkSkin>> {
    parse_catalog(bytes, None)
}

/// Directory where extracted skin previews are cached.
fn preview_dir() -> Result<PathBuf> {
    Ok(paths::app_dirs()?.cache_dir().join("hkskins-previews"))
}

fn cache_file() -> Result<PathBuf> {
    Ok(paths::app_dirs()?.cache_dir().join("hkskins.json"))
}

fn is_fresh(path: &Path, ttl: std::time::Duration) -> bool {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| std::time::SystemTime::now().duration_since(t).ok())
        .map(|age| age < ttl)
        .unwrap_or(false)
}

/// Fetch the HKSkins catalog (downloading + parsing `skins.zip`), caching the parsed list
/// so the ~13 MB archive is only re-fetched once a day (or when `force`).
pub async fn fetch_catalog(url: &str, force: bool) -> Result<Vec<HkSkin>> {
    if let Ok(cache) = cache_file() {
        if !force && is_fresh(&cache, CATALOG_TTL) {
            if let Ok(text) = std::fs::read_to_string(&cache) {
                if let Ok(skins) = serde_json::from_str::<Vec<HkSkin>>(&text) {
                    // Only trust the cache if its extracted previews still exist; an older
                    // cache (or a cleared cache dir) is treated as stale so previews are
                    // re-extracted.
                    let previews_ok = skins.is_empty()
                        || skins
                            .iter()
                            .any(|s| s.preview.as_ref().is_some_and(|p| p.exists()));
                    if previews_ok {
                        return Ok(skins);
                    }
                }
            }
        }
    }
    let bytes = net::download_bytes(url).await?;
    let skins = parse_catalog(&bytes, preview_dir().ok().as_deref())?;
    if let Ok(cache) = cache_file() {
        if let Some(parent) = cache.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string(&skins) {
            let _ = std::fs::write(&cache, json);
        }
    }
    Ok(skins)
}

/// Download an auto-downloadable skin into the Custom Knight library: either a direct zip
/// URL, or a Pingvin share on skins.hk-modding.org.
pub async fn download_into(store: &SkinStore, skin: &HkSkin) -> Result<String> {
    if let Some(id) = pingvin_share_id(&skin.source) {
        return download_pingvin_share(store, &id, &skin.name).await;
    }
    if skin.is_direct_zip() {
        let bytes = net::download_bytes(&skin.source).await?;
        return store.import_zip(CUSTOM_KNIGHT, &bytes, &skin.name);
    }
    Err(Error::Config(
        "this skin is hosted externally — use Open then “Import file…”".into(),
    ))
}

// ---- skins.hk-modding.org (Pingvin Share) ------------------------------------

const PINGVIN_BASE: &str = "https://skins.hk-modding.org";

#[derive(Debug, Deserialize)]
struct PingvinToken {
    token: String,
}

#[derive(Debug, Deserialize)]
struct PingvinShare {
    #[serde(default)]
    name: String,
    #[serde(default)]
    files: Vec<PingvinFile>,
}

#[derive(Debug, Deserialize)]
struct PingvinFile {
    id: String,
    #[serde(default)]
    name: String,
}

/// Extract the share id from a skins.hk-modding.org share URL.
fn pingvin_share_id(url: &str) -> Option<String> {
    let u = url.trim();
    for host in [
        "https://skins.hk-modding.org",
        "http://skins.hk-modding.org",
    ] {
        if let Some(rest) = u.strip_prefix(host) {
            for prefix in ["/share/", "/s/"] {
                if let Some(id) = rest.strip_prefix(prefix) {
                    let id = id.split(['/', '?', '#']).next().unwrap_or("").trim();
                    if !id.is_empty() {
                        return Some(id.to_string());
                    }
                }
            }
        }
    }
    None
}

/// Resolve a public Pingvin share and import its skin zip into the library.
async fn download_pingvin_share(store: &SkinStore, id: &str, name: &str) -> Result<String> {
    let client = net::client();

    // 1. Obtain a share token (no password) and use it as a cookie.
    let token: PingvinToken = client
        .post(format!("{PINGVIN_BASE}/api/shares/{id}/token"))
        .json(&serde_json::json!({ "password": serde_json::Value::Null }))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let cookie = format!("share_{id}_token={}", token.token);

    // 2. List the share's files.
    let share: PingvinShare = client
        .get(format!("{PINGVIN_BASE}/api/shares/{id}"))
        .header(reqwest::header::COOKIE, &cookie)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let display = if share.name.is_empty() {
        name
    } else {
        &share.name
    };

    // 3a. A single zip file: download and import it directly.
    if share.files.len() == 1 && share.files[0].name.to_lowercase().ends_with(".zip") {
        let bytes = pingvin_file_bytes(client, id, &share.files[0].id, &cookie).await?;
        return store.import_zip(CUSTOM_KNIGHT, &bytes, display);
    }

    if share.files.is_empty() {
        return Err(Error::Config("the share contains no files".into()));
    }

    // 3b. Otherwise treat the files as a skin folder: download each into a temp dir and
    // import that directory.
    let staging = paths::app_dirs()?
        .cache_dir()
        .join("pingvin")
        .join(sanitize(id));
    let _ = std::fs::remove_dir_all(&staging);
    std::fs::create_dir_all(&staging).map_err(|e| Error::io(&staging, e))?;
    for file in &share.files {
        let bytes = pingvin_file_bytes(client, id, &file.id, &cookie).await?;
        let out = staging.join(sanitize(&file.name));
        std::fs::write(&out, bytes).map_err(|e| Error::io(&out, e))?;
    }
    let result = store.import_dir(CUSTOM_KNIGHT, &staging, Some(display));
    let _ = std::fs::remove_dir_all(&staging);
    result
}

async fn pingvin_file_bytes(
    client: &reqwest::Client,
    id: &str,
    file_id: &str,
    cookie: &str,
) -> Result<Vec<u8>> {
    let resp = client
        .get(format!(
            "{PINGVIN_BASE}/api/shares/{id}/files/{file_id}?download=true"
        ))
        .header(reqwest::header::COOKIE, cookie)
        .send()
        .await?
        .error_for_status()?;
    Ok(resp.bytes().await?.to_vec())
}

// ---- Helpers -----------------------------------------------------------------

/// The single top-level directory shared by all entries, if there is exactly one.
///
/// Requires every entry to live *inside* that directory (depth > 1); a flat file at the
/// archive root is not a top-level directory.
fn single_top_dir(entries: &[PathBuf]) -> Option<String> {
    if entries.is_empty() {
        return None;
    }
    let mut top: Option<String> = None;
    for e in entries {
        if e.components().count() < 2 {
            return None; // a root-level file — not nested under a single dir
        }
        let first = e.components().next()?;
        let comp = first.as_os_str().to_string_lossy().into_owned();
        match &top {
            None => top = Some(comp),
            Some(t) if *t == comp => {}
            Some(_) => return None, // more than one distinct top-level component
        }
    }
    top
}

/// Replace `dest` with a fresh copy of `src` (clean overwrite).
fn replace_dir_with_copy(src: &Path, dest: &Path) -> Result<()> {
    let _ = std::fs::remove_dir_all(dest);
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
    }
    paths::copy_dir_recursive(src, dest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game;
    use std::io::{Cursor, Write};

    fn store() -> (tempfile::TempDir, SkinStore) {
        let tmp = tempfile::tempdir().unwrap();
        let store = SkinStore::with_root(tmp.path().join("skins"));
        (tmp, store)
    }

    #[test]
    fn reads_custom_knight_selected_skin() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        // Missing file → couldn't read (None), so callers leave state untouched.
        assert_eq!(read_active_skin(dir), None);

        std::fs::write(
            dir.join("CustomKnight.GlobalSettings.json"),
            br#"{"Version":"1.0","Preloads":true,"DefaultSkin":"My Cool Skin"}"#,
        )
        .unwrap();
        assert_eq!(read_active_skin(dir).as_deref(), Some("My Cool Skin"));

        // The built-in default reads back literally so callers can clear the marker.
        std::fs::write(
            dir.join("CustomKnight.GlobalSettings.json"),
            br#"{"DefaultSkin":"Default"}"#,
        )
        .unwrap();
        assert_eq!(read_active_skin(dir).as_deref(), Some("Default"));
    }

    #[test]
    fn writes_hp_bar_integration_flag_preserving_other_settings() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        let file = dir.join("EnemyHPBar.GlobalSettings.json");

        std::fs::write(
            &file,
            br#"{"NameLength":10,"Intergration":false,"CurrentSkin":"Default"}"#,
        )
        .unwrap();
        write_hp_bar_integration(dir, true).unwrap();

        let value: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
        // The mod's own (mis)spelling of the field is what it reads.
        assert_eq!(value["Intergration"], true);
        assert_eq!(value["NameLength"], 10);
        assert_eq!(value["CurrentSkin"], "Default");
    }

    #[test]
    fn writes_selected_skin_preserving_other_settings() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        let file = dir.join("CustomKnight.GlobalSettings.json");

        // Writing into a fresh save dir creates the file.
        write_active_skin(dir, "Cool Skin").unwrap();
        assert_eq!(read_active_skin(dir).as_deref(), Some("Cool Skin"));

        // Unrelated settings are preserved across an update.
        std::fs::write(
            &file,
            br#"{"Version":"2.0","Preloads":false,"NameLength":20,"DefaultSkin":"Old"}"#,
        )
        .unwrap();
        write_active_skin(dir, "New Skin").unwrap();
        let value: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
        assert_eq!(value["DefaultSkin"], "New Skin");
        assert_eq!(value["Version"], "2.0");
        assert_eq!(value["Preloads"], false);
        assert_eq!(value["NameLength"], 20);
    }

    fn make_skin_dir(base: &Path, files: &[(&str, &[u8])]) -> PathBuf {
        let dir = base.join("incoming_skin");
        for (rel, data) in files {
            let p = dir.join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, data).unwrap();
        }
        dir
    }

    fn skin_zip(top: Option<&str>) -> Vec<u8> {
        let mut buf = Vec::new();
        {
            let mut w = zip::ZipWriter::new(Cursor::new(&mut buf));
            let opts: zip::write::FileOptions<()> = zip::write::FileOptions::default();
            let name = match top {
                Some(t) => format!("{t}/Knight.png"),
                None => "Knight.png".to_string(),
            };
            w.start_file(name, opts).unwrap();
            w.write_all(b"png").unwrap();
            w.finish().unwrap();
        }
        buf
    }

    #[test]
    fn import_dir_list_and_remove() {
        let (tmp, store) = store();
        let src = make_skin_dir(tmp.path(), &[("Knight.png", b"x"), ("skin.json", b"{}")]);
        let name = store
            .import_dir(CUSTOM_KNIGHT, &src, Some("Cool Skin"))
            .unwrap();
        assert_eq!(name, "Cool Skin");
        assert_eq!(store.list(CUSTOM_KNIGHT).unwrap(), vec!["Cool Skin"]);
        store.remove(CUSTOM_KNIGHT, "Cool Skin").unwrap();
        assert!(store.list(CUSTOM_KNIGHT).unwrap().is_empty());
    }

    #[test]
    fn import_zip_detects_top_dir() {
        let (_tmp, store) = store();
        let name = store
            .import_zip(CUSTOM_KNIGHT, &skin_zip(Some("My Skin")), "fallback")
            .unwrap();
        assert_eq!(name, "My Skin");
        assert!(store
            .kind_dir(CUSTOM_KNIGHT)
            .join("My Skin/Knight.png")
            .exists());

        let name2 = store
            .import_zip(CUSTOM_KNIGHT, &skin_zip(None), "Flat Skin")
            .unwrap();
        assert_eq!(name2, "Flat Skin");
        assert!(store
            .kind_dir(CUSTOM_KNIGHT)
            .join("Flat Skin/Knight.png")
            .exists());
    }

    #[test]
    fn sync_copies_into_installed_mod() {
        let (tmp, store) = store();
        // Build a fake install with CustomKnight installed under its ModLinks folder name
        // ("Custom Knight", with a space), located via its assembly.
        let managed = tmp.path().join("game/hollow_knight_Data/Managed");
        std::fs::create_dir_all(managed.join("Mods/Custom Knight")).unwrap();
        std::fs::write(managed.join("Mods/Custom Knight/CustomKnight.dll"), b"dll").unwrap();
        std::fs::write(managed.join("UnityEngine.dll"), b"u").unwrap();
        std::fs::write(managed.join("Assembly-CSharp.dll"), b"a").unwrap();
        let install = game::validate(tmp.path().join("game")).unwrap();

        let src = make_skin_dir(tmp.path(), &[("Knight.png", b"x")]);
        store
            .import_dir(CUSTOM_KNIGHT, &src, Some("Skin A"))
            .unwrap();

        assert!(is_mod_installed(&install, CUSTOM_KNIGHT));
        let n = store.sync_to_game(&install, CUSTOM_KNIGHT).unwrap();
        assert_eq!(n, 1);
        let skins = game_skins_dir(&install, CUSTOM_KNIGHT).unwrap();
        assert!(skins.join("Skin A/Knight.png").exists());
        // The Default skin folder is created so skins work before first launch.
        assert!(skins.join("Default").is_dir());
    }

    #[test]
    fn sync_errors_when_mod_absent() {
        let (tmp, store) = store();
        let managed = tmp.path().join("game/hollow_knight_Data/Managed");
        std::fs::create_dir_all(managed.join("Mods")).unwrap();
        std::fs::write(managed.join("UnityEngine.dll"), b"u").unwrap();
        std::fs::write(managed.join("Assembly-CSharp.dll"), b"a").unwrap();
        let install = game::validate(tmp.path().join("game")).unwrap();
        assert!(store.sync_to_game(&install, CUSTOM_KNIGHT).is_err());
    }

    #[test]
    fn parses_hkskins_zip_and_filters_to_hollow_knight() {
        // Mimic the HKSkins skins.zip layout: per-skin metadata.json (+ preview.png).
        let mut buf = Vec::new();
        {
            let mut w = zip::ZipWriter::new(Cursor::new(&mut buf));
            let opts: zip::write::FileOptions<()> = zip::write::FileOptions::default();
            w.start_file("skins/Among Us/metadata.json", opts).unwrap();
            w.write_all(
                br#"{"name":"Among Us","author":"Amon","game":"hollowKnight","desc":"Knight","source":"https://drive.google.com/x"}"#,
            )
            .unwrap();
            w.start_file("skins/Among Us/preview.png", opts).unwrap();
            w.write_all(b"\x89PNG fake").unwrap();
            w.start_file("skins/Silk Thing/metadata.json", opts)
                .unwrap();
            w.write_all(br#"{"name":"Silk Thing","game":"silksong","source":"x.zip"}"#)
                .unwrap();
            w.finish().unwrap();
        }

        let skins = parse_catalog_zip(&buf).unwrap();
        assert_eq!(skins.len(), 1, "only the hollowKnight skin should be kept");
        assert_eq!(skins[0].name, "Among Us");
        assert_eq!(skins[0].author, "Amon");
        assert!(!skins[0].is_direct_zip());
    }

    #[test]
    fn direct_zip_detection() {
        let mk = |src: &str| HkSkin {
            name: "x".into(),
            author: String::new(),
            desc: String::new(),
            source: src.into(),
            date_added: String::new(),
            preview: None,
        };
        assert!(mk("https://host/skin.zip").is_direct_zip());
        assert!(mk("https://host/skin.zip?dl=1").is_direct_zip());
        assert!(!mk("https://discord.com/channels/1/2/3").is_direct_zip());

        // Pingvin shares are auto-downloadable even though they aren't direct zips.
        assert!(mk("https://skins.hk-modding.org/share/amanbybluebean").is_auto_downloadable());
        assert!(!mk("https://discord.com/channels/1/2/3").is_auto_downloadable());
    }

    #[tokio::test]
    #[ignore = "hits the live skins.hk-modding.org server"]
    async fn pingvin_download_live() {
        let tmp = tempfile::tempdir().unwrap();
        let store = SkinStore::with_root(tmp.path().join("skins"));
        let skin = HkSkin {
            name: "A Man (City of Mist)".into(),
            author: "Bluebean".into(),
            desc: String::new(),
            source: "https://skins.hk-modding.org/share/amanbybluebean".into(),
            date_added: String::new(),
            preview: None,
        };
        let name = download_into(&store, &skin).await.unwrap();
        assert!(!store.list(CUSTOM_KNIGHT).unwrap().is_empty());
        eprintln!("imported skin: {name}");
    }

    #[test]
    fn parses_pingvin_share_ids() {
        assert_eq!(
            pingvin_share_id("https://skins.hk-modding.org/share/amanbybluebean").as_deref(),
            Some("amanbybluebean")
        );
        assert_eq!(
            pingvin_share_id("https://skins.hk-modding.org/share/foo/?x=1").as_deref(),
            Some("foo")
        );
        assert_eq!(pingvin_share_id("https://drive.google.com/x"), None);
    }
}
