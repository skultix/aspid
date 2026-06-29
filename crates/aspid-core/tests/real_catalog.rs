//! Parse the real ModLinks/ApiLinks documents (committed fixtures) to guard against
//! schema drift and edge cases the small inline fixtures miss.

use aspid_core::modlinks::{parse_apilinks, parse_modlinks, ModLink, Platform};

const MODLINKS: &str = include_str!("fixtures/ModLinks.xml");
const APILINKS: &str = include_str!("fixtures/ApiLinks.xml");

#[test]
fn parses_full_modlinks_catalog() {
    let catalog = parse_modlinks(MODLINKS).expect("real ModLinks.xml should parse");
    // The real catalog has hundreds of mods.
    assert!(
        catalog.len() > 100,
        "expected a large catalog, got {}",
        catalog.len()
    );

    // Every mod must have a non-empty name and a usable download for some platform.
    for m in catalog.mods() {
        assert!(!m.name.is_empty(), "mod with empty name");
        let has_any = m.link.for_platform(Platform::Linux).is_some()
            || m.link.for_platform(Platform::Mac).is_some()
            || m.link.for_platform(Platform::Windows).is_some();
        assert!(has_any, "mod {} has no download link", m.name);
    }

    // Satchel is a ubiquitous library dependency and should be present.
    let satchel = catalog.get("Satchel").expect("Satchel present in catalog");
    assert!(!satchel.version.is_empty());

    // Pale Court is a known platform-specific (per-OS) mod.
    if let Some(pale) = catalog.get("Pale Court") {
        assert!(matches!(pale.link, ModLink::Platform { .. }));
        assert!(pale.dependencies.contains(&"SFCore".to_string()));
    }
}

#[test]
fn parses_real_api_manifest() {
    let api = parse_apilinks(APILINKS).expect("real ApiLinks.xml should parse");
    assert!(!api.version.is_empty());
    assert!(api.files.iter().any(|f| f == "Assembly-CSharp.dll"));
    assert!(api.current_link().is_some());
}
