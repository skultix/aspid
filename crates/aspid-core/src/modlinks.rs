//! Fetching and parsing the ModLinks and ApiLinks XML catalogs, with an on-disk cache.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use serde::Deserialize;

use crate::config::Config;
use crate::error::{Error, Result};
use crate::{net, paths};

/// How long a cached catalog is considered fresh before we re-fetch.
pub const CACHE_TTL: Duration = Duration::from_secs(60 * 60); // 1 hour

/// The platform a download link targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    /// Linux.
    Linux,
    /// macOS.
    Mac,
    /// Windows.
    Windows,
}

impl Platform {
    /// The platform aspid is currently running on.
    pub const fn current() -> Platform {
        #[cfg(target_os = "windows")]
        {
            Platform::Windows
        }
        #[cfg(target_os = "macos")]
        {
            Platform::Mac
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        {
            Platform::Linux
        }
    }
}

/// A single downloadable archive plus its expected checksum.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadLink {
    /// The archive URL.
    pub url: String,
    /// Expected lower-/upper-case hex SHA-256.
    pub sha256: String,
}

/// A mod's download, either universal or split per platform.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModLink {
    /// One archive for all platforms.
    Universal(DownloadLink),
    /// Per-platform archives (any subset may be present).
    Platform {
        /// Linux archive.
        linux: Option<DownloadLink>,
        /// macOS archive.
        mac: Option<DownloadLink>,
        /// Windows archive.
        windows: Option<DownloadLink>,
    },
}

impl ModLink {
    /// The download for a specific platform, if available.
    pub fn for_platform(&self, platform: Platform) -> Option<&DownloadLink> {
        match self {
            ModLink::Universal(link) => Some(link),
            ModLink::Platform {
                linux,
                mac,
                windows,
            } => match platform {
                Platform::Linux => linux.as_ref(),
                Platform::Mac => mac.as_ref(),
                Platform::Windows => windows.as_ref(),
            },
        }
    }

    /// The download for the current platform, if available.
    pub fn current(&self) -> Option<&DownloadLink> {
        self.for_platform(Platform::current())
    }
}

/// A catalog mod entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mod {
    /// Unique mod name (also the key used by dependencies).
    pub name: String,
    /// Human description.
    pub description: String,
    /// Version string (dot-separated numeric tuple).
    pub version: String,
    /// Download link(s).
    pub link: ModLink,
    /// Names of mods this one depends on.
    pub dependencies: Vec<String>,
    /// Source repository URL, if given.
    pub repository: Option<String>,
    /// Categorisation tags.
    pub tags: Vec<String>,
    /// Author names.
    pub authors: Vec<String>,
    /// Optional integrations with other mods.
    pub integrations: Vec<String>,
}

/// The modding-API manifest from ApiLinks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiManifest {
    /// API version (an integer, e.g. `77`).
    pub version: String,
    /// Per-platform download links.
    pub link: ModLink,
    /// Files contained in the API archive that belong under `Managed/`.
    pub files: Vec<String>,
}

impl ApiManifest {
    /// The download link for the current platform.
    pub fn current_link(&self) -> Option<&DownloadLink> {
        self.link.current()
    }
}

/// The parsed mod catalog with name-indexed lookup.
#[derive(Debug, Clone, Default)]
pub struct Catalog {
    mods: Vec<Mod>,
    index: HashMap<String, usize>,
}

impl Catalog {
    /// Build a catalog from a list of mods.
    pub fn new(mods: Vec<Mod>) -> Self {
        let index = mods
            .iter()
            .enumerate()
            .map(|(i, m)| (m.name.clone(), i))
            .collect();
        Catalog { mods, index }
    }

    /// All mods, in catalog order.
    pub fn mods(&self) -> &[Mod] {
        &self.mods
    }

    /// Look up a mod by exact name.
    pub fn get(&self, name: &str) -> Option<&Mod> {
        self.index.get(name).map(|&i| &self.mods[i])
    }

    /// Number of mods in the catalog.
    pub fn len(&self) -> usize {
        self.mods.len()
    }

    /// Whether the catalog is empty.
    pub fn is_empty(&self) -> bool {
        self.mods.is_empty()
    }
}

// ---- Parsing -----------------------------------------------------------------

/// Parse ModLinks XML into a [`Catalog`].
pub fn parse_modlinks(xml: &str) -> Result<Catalog> {
    let doc: raw::ModLinks = quick_xml::de::from_str(xml).map_err(|source| Error::Xml {
        what: "ModLinks.xml",
        source,
    })?;
    let mods = doc.manifest.into_iter().map(Mod::from).collect();
    Ok(Catalog::new(mods))
}

/// Parse ApiLinks XML into an [`ApiManifest`].
pub fn parse_apilinks(xml: &str) -> Result<ApiManifest> {
    let doc: raw::ApiLinks = quick_xml::de::from_str(xml).map_err(|source| Error::Xml {
        what: "ApiLinks.xml",
        source,
    })?;
    Ok(ApiManifest::from(doc.manifest))
}

// ---- Fetching + caching ------------------------------------------------------

fn cache_path(file: &str) -> Result<PathBuf> {
    Ok(paths::app_dirs()?.cache_dir().join(file))
}

fn is_fresh(path: &std::path::Path, ttl: Duration) -> bool {
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    let Ok(modified) = meta.modified() else {
        return false;
    };
    SystemTime::now()
        .duration_since(modified)
        .map(|age| age < ttl)
        .unwrap_or(false)
}

async fn fetch_cached(url: &str, cache_file: &str, force: bool) -> Result<String> {
    let path = cache_path(cache_file)?;
    if !force && is_fresh(&path, CACHE_TTL) {
        if let Ok(text) = std::fs::read_to_string(&path) {
            return Ok(text);
        }
    }
    let text = net::fetch_text(url).await?;
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&path, &text);
    Ok(text)
}

/// Fetch and parse the mod catalog, using the cache when fresh (unless `force`).
pub async fn fetch_catalog(config: &Config, force: bool) -> Result<Catalog> {
    let xml = fetch_cached(config.modlinks_url(), "ModLinks.xml", force).await?;
    parse_modlinks(&xml)
}

/// Fetch and parse the API manifest, using the cache when fresh (unless `force`).
pub async fn fetch_api_manifest(config: &Config, force: bool) -> Result<ApiManifest> {
    let xml = fetch_cached(config.apilinks_url(), "ApiLinks.xml", force).await?;
    parse_apilinks(&xml)
}

// ---- Raw serde representation ------------------------------------------------

mod raw {
    use super::*;

    #[derive(Debug, Deserialize)]
    pub struct ModLinks {
        #[serde(rename = "Manifest", default)]
        pub manifest: Vec<ModManifest>,
    }

    #[derive(Debug, Deserialize)]
    pub struct ApiLinks {
        #[serde(rename = "Manifest")]
        pub manifest: ApiManifest,
    }

    #[derive(Debug, Deserialize)]
    pub struct Link {
        #[serde(rename = "@SHA256")]
        pub sha256: String,
        #[serde(rename = "$text", default)]
        pub url: String,
    }

    #[derive(Debug, Deserialize)]
    pub struct Links {
        #[serde(rename = "Linux")]
        pub linux: Option<Link>,
        #[serde(rename = "Mac")]
        pub mac: Option<Link>,
        #[serde(rename = "Windows")]
        pub windows: Option<Link>,
    }

    #[derive(Debug, Default, Deserialize)]
    pub struct Dependencies {
        #[serde(rename = "Dependency", default)]
        pub dependency: Vec<String>,
    }

    #[derive(Debug, Default, Deserialize)]
    pub struct Tags {
        #[serde(rename = "Tag", default)]
        pub tag: Vec<String>,
    }

    #[derive(Debug, Default, Deserialize)]
    pub struct Authors {
        #[serde(rename = "Author", default)]
        pub author: Vec<String>,
    }

    #[derive(Debug, Default, Deserialize)]
    pub struct Integrations {
        #[serde(rename = "Integration", default)]
        pub integration: Vec<String>,
    }

    #[derive(Debug, Deserialize)]
    pub struct ModManifest {
        #[serde(rename = "Name")]
        pub name: String,
        #[serde(rename = "Description", default)]
        pub description: String,
        #[serde(rename = "Version")]
        pub version: String,
        #[serde(rename = "Link")]
        pub link: Option<Link>,
        #[serde(rename = "Links")]
        pub links: Option<Links>,
        #[serde(rename = "Dependencies", default)]
        pub dependencies: Dependencies,
        #[serde(rename = "Repository")]
        pub repository: Option<String>,
        #[serde(rename = "Tags", default)]
        pub tags: Tags,
        #[serde(rename = "Authors", default)]
        pub authors: Authors,
        #[serde(rename = "Integrations", default)]
        pub integrations: Integrations,
    }

    #[derive(Debug, Deserialize)]
    pub struct ApiManifest {
        #[serde(rename = "Version")]
        pub version: String,
        #[serde(rename = "Links")]
        pub links: Links,
        #[serde(rename = "Files", default)]
        pub files: Files,
    }

    #[derive(Debug, Default, Deserialize)]
    pub struct Files {
        #[serde(rename = "File", default)]
        pub file: Vec<String>,
    }

    impl From<Link> for DownloadLink {
        fn from(l: Link) -> Self {
            DownloadLink {
                url: l.url.trim().to_string(),
                sha256: l.sha256.trim().to_string(),
            }
        }
    }

    impl Links {
        pub fn into_modlink(self) -> ModLink {
            ModLink::Platform {
                linux: self.linux.map(Into::into),
                mac: self.mac.map(Into::into),
                windows: self.windows.map(Into::into),
            }
        }
    }
}

impl From<raw::ModManifest> for Mod {
    fn from(m: raw::ModManifest) -> Self {
        // A manifest carries either a single <Link> or a per-platform <Links>.
        let link = match (m.link, m.links) {
            (Some(link), _) => ModLink::Universal(link.into()),
            (None, Some(links)) => links.into_modlink(),
            (None, None) => ModLink::Platform {
                linux: None,
                mac: None,
                windows: None,
            },
        };
        Mod {
            name: m.name.trim().to_string(),
            description: m.description.trim().to_string(),
            version: m.version.trim().to_string(),
            link,
            dependencies: clean(m.dependencies.dependency),
            repository: m
                .repository
                .map(|r| r.trim().to_string())
                .filter(|s| !s.is_empty()),
            tags: clean(m.tags.tag),
            authors: clean(m.authors.author),
            integrations: clean(m.integrations.integration),
        }
    }
}

impl From<raw::ApiManifest> for ApiManifest {
    fn from(m: raw::ApiManifest) -> Self {
        ApiManifest {
            version: m.version.trim().to_string(),
            link: m.links.into_modlink(),
            files: clean(m.files.file),
        }
    }
}

/// Trim and drop empty entries from a list of strings.
fn clean(items: Vec<String>) -> Vec<String> {
    items
        .into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"<?xml version="1.0"?>
<ModLinks xmlns="https://example/ns">
    <Manifest>
        <Name>Satchel</Name>
        <Description>A library.</Description>
        <Version>1.2.3.4</Version>
        <Link SHA256="aabbcc"><![CDATA[https://example/Satchel.zip]]></Link>
        <Dependencies />
        <Repository><![CDATA[https://example/satchel]]></Repository>
        <Tags><Tag>Library</Tag></Tags>
        <Authors><Author>Someone</Author></Authors>
    </Manifest>
    <Manifest>
        <Name>Pale Court</Name>
        <Description>Bosses.</Description>
        <Version>1.1.1.7</Version>
        <Links>
            <Linux SHA256="lin"><![CDATA[https://example/PaleCourt-Lin.zip]]></Linux>
            <Mac SHA256="mac"><![CDATA[https://example/PaleCourt-Mac.zip]]></Mac>
            <Windows SHA256="win"><![CDATA[https://example/PaleCourt-Win.zip]]></Windows>
        </Links>
        <Dependencies>
            <Dependency>Satchel</Dependency>
            <Dependency>SFCore</Dependency>
        </Dependencies>
        <Integrations><Integration>Enemy HP Bar</Integration></Integrations>
        <Tags><Tag>Boss</Tag><Tag>Expansion</Tag></Tags>
        <Authors><Author>MEBI</Author></Authors>
    </Manifest>
</ModLinks>"#;

    const API_SAMPLE: &str = r#"<?xml version="1.0"?>
<ApiLinks xmlns="https://example/ns">
    <Manifest>
        <Version>77</Version>
        <Links>
            <Linux SHA256="linhash"><![CDATA[https://example/api-linux.zip]]></Linux>
            <Mac SHA256="machash"><![CDATA[https://example/api-mac.zip]]></Mac>
            <Windows SHA256="winhash"><![CDATA[https://example/api-win.zip]]></Windows>
        </Links>
        <Files>
            <File>Assembly-CSharp.dll</File>
            <File>MonoMod.Utils.dll</File>
        </Files>
    </Manifest>
</ApiLinks>"#;

    #[test]
    fn parses_universal_and_platform_mods() {
        let catalog = parse_modlinks(SAMPLE).unwrap();
        assert_eq!(catalog.len(), 2);

        let satchel = catalog.get("Satchel").unwrap();
        assert_eq!(satchel.version, "1.2.3.4");
        assert!(satchel.dependencies.is_empty());
        assert_eq!(satchel.tags, vec!["Library"]);
        match &satchel.link {
            ModLink::Universal(link) => {
                assert_eq!(link.url, "https://example/Satchel.zip");
                assert_eq!(link.sha256, "aabbcc");
            }
            _ => panic!("expected universal link"),
        }

        let pale = catalog.get("Pale Court").unwrap();
        assert_eq!(pale.dependencies, vec!["Satchel", "SFCore"]);
        assert_eq!(pale.integrations, vec!["Enemy HP Bar"]);
        let lin = pale.link.for_platform(Platform::Linux).unwrap();
        assert_eq!(lin.url, "https://example/PaleCourt-Lin.zip");
        assert_eq!(lin.sha256, "lin");
        let win = pale.link.for_platform(Platform::Windows).unwrap();
        assert_eq!(win.sha256, "win");
    }

    #[test]
    fn parses_api_manifest() {
        let api = parse_apilinks(API_SAMPLE).unwrap();
        assert_eq!(api.version, "77");
        assert_eq!(api.files, vec!["Assembly-CSharp.dll", "MonoMod.Utils.dll"]);
        assert_eq!(
            api.link.for_platform(Platform::Linux).unwrap().url,
            "https://example/api-linux.zip"
        );
    }
}
