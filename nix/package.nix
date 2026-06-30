{
  lib,
  rustPlatform,
  pkg-config,
  copyDesktopItems,
  makeDesktopItem,
  # GUI libraries aspid dlopens at runtime; passed in so the dev shell can share the list.
  runtimeLibs,
}:

let
  cargoToml = lib.importTOML ../crates/aspid/Cargo.toml;
in
rustPlatform.buildRustPackage {
  pname = "aspid";
  version = (lib.importTOML ../Cargo.toml).workspace.package.version;

  src = lib.cleanSource ../.;

  cargoLock = {
    lockFile = ../Cargo.lock;
  };

  nativeBuildInputs = [
    pkg-config
    copyDesktopItems
  ];

  # The GUI libraries are dlopened at runtime (see runtime-libs.nix), so nothing extra is
  # needed to *link*. zip's zstd/bzip2 codecs vendor their C and build with the stdenv cc.

  desktopItems = [
    (makeDesktopItem {
      name = "aspid";
      desktopName = "aspid";
      comment = cargoToml.package.description or "Hollow Knight mod manager";
      exec = "aspid";
      icon = "aspid";
      categories = [
        "Game"
        "Utility"
      ];
      terminal = false;
      # Matches the window's Wayland app_id / X11 WM_CLASS (set via iced's Settings.id) so
      # taskbars associate the running window with this entry.
      startupWMClass = "aspid";
    })
  ];

  postInstall = ''
    install -Dm644 ${../icons/aspid.png} $out/share/pixmaps/aspid.png
  '';

  # dlopen consults the executable's RUNPATH, so embedding it makes the GUI libs resolvable
  # without an LD_LIBRARY_PATH wrapper.
  postFixup = ''
    patchelf --add-rpath "${lib.makeLibraryPath runtimeLibs}" "$out/bin/aspid"
  '';

  meta = {
    description = cargoToml.package.description or "A cross-platform Hollow Knight mod manager";
    homepage = "https://github.com/marlstar/aspid";
    license = lib.licenses.mit;
    mainProgram = "aspid";
    platforms = lib.platforms.linux;
  };
}
