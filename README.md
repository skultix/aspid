# aspid

A fast, native, cross-platform **Hollow Knight mod manager**, written in Rust with
[Iced](https://iced.rs). Inspired by [Lumafly](https://github.com/TheMulhima/Lumafly),
rebuilt from the ground up with first-class **modpacks** (completely separate saves + mods)
and built-in **skin** management.

## Features

- **Mod management** — browse the official [ModLinks](https://github.com/hk-modding/modlinks)
  catalog, install/remove with automatic transitive dependencies, and reverse-dependency
  warnings before removal. Update-available badges.
- **Modding API** — one-click install/update of the
  [Hollow Knight Modding API](https://github.com/hk-modding/api), with a vanilla⇄modded
  toggle for one-off vanilla launches (the vanilla assembly is always backed up).
- **Modpacks** — each pack has completely separate data (saves, installed mods, config).
  The active pack is surfaced to the game via directory links; switching is instant and
  duplicates nothing. Vanilla is a managed pack too.
- **Skins** — a central, cross-pack library for Custom Knight (and boss-bar) skins, with a
  downloadable catalog. Skins persist regardless of the active modpack.
- **Appearance** — preset themes plus a configurable accent colour.
- **Cross-platform** — Linux (primary), Windows, and macOS. Game detection via Steam, or a
  manual path.

## Architecture

A Cargo workspace:

- [`aspid-core`](crates/aspid-core) — UI-agnostic domain logic (game detection, catalog
  parsing, downloads, the modding-API and mod install engines, modpacks, skins). Fully
  unit-tested.
- [`aspid`](crates/aspid) — the Iced 0.14 front-end.

## Building

Requires Rust (see [`devenv.nix`](devenv.nix); stable, 1.96+).

```sh
cargo run        # launch the app
cargo test       # run the test suite
cargo clippy --all-targets -- -D warnings
```

On Linux you may need `libxkbcommon` and `libwayland` development packages for the GUI.

## Status

Implements the planned vertical slice, modpacks, skins, and theming. Silksong support is a
future consideration; the core is written to be game-parameterised to make that easier.
