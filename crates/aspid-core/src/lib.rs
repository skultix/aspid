//! `aspid-core` — UI-agnostic domain logic for the aspid Hollow Knight mod manager.
//!
//! This crate knows how to locate the game, talk to the ModLinks/ApiLinks catalogs,
//! install/remove mods and the modding API, manage modpacks with isolated data, and
//! manage cosmetic skins. It contains no GUI code so it can be driven by the Iced
//! front-end, a future CLI, or tests.

#![warn(missing_docs)]

pub mod config;
pub mod error;
pub mod game;
pub mod modlinks;
pub mod net;
pub mod paths;
pub mod version;

pub use config::Config;
pub use error::{Error, Result};
pub use game::{ApiState, Install};
pub use modlinks::{ApiManifest, Catalog, DownloadLink, Mod, ModLink, Platform};
