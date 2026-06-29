//! Launching the game. We go through Steam's `rungameid` URI, which works regardless of
//! where the library lives and lets Steam set up the runtime/overlay as usual.

use std::process::Command;

use crate::error::{Error, Result};
use crate::paths::HOLLOW_KNIGHT_APP_ID;

/// Open an arbitrary URL (e.g. a mod's homepage) with the platform's default handler.
pub fn open_url(url: &str) -> Result<()> {
    open_uri(url)
}

/// Open a URI/URL with the platform's default handler.
fn open_uri(uri: &str) -> Result<()> {
    let mut cmd = platform_opener();
    cmd.arg(uri);
    cmd.spawn()
        .map(|_| ())
        .map_err(|e| Error::io(std::path::Path::new(uri), e))
}

fn platform_opener() -> Command {
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
    }
    #[cfg(target_os = "windows")]
    {
        let mut c = Command::new("cmd");
        // `start` needs an empty title argument before the URL.
        c.args(["/C", "start", ""]);
        c
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Command::new("xdg-open")
    }
}

/// Launch Hollow Knight via Steam. Whether it runs modded or vanilla depends on which
/// `Assembly-CSharp.dll` is currently active (see [`crate::modapi`]).
pub fn launch_via_steam() -> Result<()> {
    open_uri(&format!("steam://rungameid/{HOLLOW_KNIGHT_APP_ID}"))
}
