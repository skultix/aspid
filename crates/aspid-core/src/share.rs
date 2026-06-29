//! Sharing modpacks as portable codes.
//!
//! A shared modpack is just its name plus the list of mods it contains (name + version) —
//! deliberately *not* saves or config, which are personal and large. The list is encoded
//! as a compact base64 string (prefixed for recognisability) that can be copied/pasted, or
//! written as JSON to a file. Importing recreates a pack and reinstalls the listed mods
//! (with dependencies) from the catalog.

use base64::Engine;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::mods::InstalledMod;

/// Prefix identifying an aspid modpack share code (version 1).
const PREFIX: &str = "ASPID1:";

/// One mod in a shared modpack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedMod {
    /// Mod name (matches the ModLinks catalog key).
    pub name: String,
    /// Version that was installed when shared, if known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// A shareable modpack: a name and the mods it contains.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackShare {
    /// The pack's name.
    pub name: String,
    /// The mods in the pack.
    #[serde(default)]
    pub mods: Vec<SharedMod>,
}

impl PackShare {
    /// Build a share from a pack name and its installed mods.
    pub fn from_installed(name: impl Into<String>, mods: &[InstalledMod]) -> Self {
        let mut mods: Vec<SharedMod> = mods
            .iter()
            .map(|m| SharedMod {
                name: m.name.clone(),
                version: m.version.clone(),
            })
            .collect();
        mods.sort_by(|a, b| a.name.cmp(&b.name));
        PackShare {
            name: name.into(),
            mods,
        }
    }

    /// Encode to a one-line shareable code.
    pub fn to_code(&self) -> Result<String> {
        let json = serde_json::to_vec(self).map_err(|e| Error::Config(e.to_string()))?;
        let b64 = base64::engine::general_purpose::STANDARD.encode(json);
        Ok(format!("{PREFIX}{b64}"))
    }

    /// Decode a shareable code produced by [`PackShare::to_code`].
    pub fn from_code(code: &str) -> Result<Self> {
        let body = code
            .trim()
            .strip_prefix(PREFIX)
            .ok_or_else(|| Error::Config("not an aspid modpack code".into()))?;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(body.trim())
            .map_err(|e| Error::Config(format!("invalid modpack code: {e}")))?;
        serde_json::from_slice(&bytes)
            .map_err(|e| Error::Config(format!("invalid modpack code: {e}")))
    }

    /// Encode to a pretty JSON document (for file sharing).
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self).map_err(|e| Error::Config(e.to_string()))
    }

    /// Decode from a JSON document.
    pub fn from_json(s: &str) -> Result<Self> {
        serde_json::from_str(s).map_err(|e| Error::Config(format!("invalid modpack file: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> PackShare {
        PackShare {
            name: "My Pack".into(),
            mods: vec![
                SharedMod {
                    name: "Satchel".into(),
                    version: Some("1.2.3.4".into()),
                },
                SharedMod {
                    name: "Pale Court".into(),
                    version: None,
                },
            ],
        }
    }

    #[test]
    fn code_roundtrips() {
        let s = sample();
        let code = s.to_code().unwrap();
        assert!(code.starts_with(PREFIX));
        assert_eq!(PackShare::from_code(&code).unwrap(), s);
    }

    #[test]
    fn json_roundtrips() {
        let s = sample();
        assert_eq!(PackShare::from_json(&s.to_json().unwrap()).unwrap(), s);
    }

    #[test]
    fn rejects_garbage() {
        assert!(PackShare::from_code("not-a-code").is_err());
        assert!(PackShare::from_code("ASPID1:!!!notbase64!!!").is_err());
    }

    #[test]
    fn from_installed_sorts_and_copies_version() {
        let installed = vec![
            InstalledMod {
                name: "Zote".into(),
                version: Some("1.0.0.0".into()),
                enabled: true,
            },
            InstalledMod {
                name: "Apple".into(),
                version: None,
                enabled: false,
            },
        ];
        let share = PackShare::from_installed("Pack", &installed);
        assert_eq!(share.mods[0].name, "Apple");
        assert_eq!(share.mods[1].name, "Zote");
        assert_eq!(share.mods[1].version.as_deref(), Some("1.0.0.0"));
    }
}
