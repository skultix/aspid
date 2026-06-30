{
	description = "aspid — a cross-platform Hollow Knight mod manager";

	inputs = {
		nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
		# Pins an exact Rust toolchain so the workspace's MSRV (rust 1.96) is met regardless of
		# the rustc nixpkgs happens to ship.
		rust-overlay = {
			url = "github:oxalica/rust-overlay";
			inputs.nixpkgs.follows = "nixpkgs";
		};
	};

	outputs =
	{ self, nixpkgs, rust-overlay }:
	let
	# aspid is a Linux/Wayland+X11 GUI; these are the systems the flake builds for.
	systems = [
		"x86_64-linux"
		"aarch64-linux"
	];
	forAllSystems = nixpkgs.lib.genAttrs systems;
	pkgsFor =
	system:
	import nixpkgs {
		inherit system;
		overlays = [ rust-overlay.overlays.default ];
	};

	# The pinned toolchain, matching the workspace's `rust-version`.
	toolchainFor = pkgs: pkgs.rust-bin.stable."1.96.0".default;

	aspidFor =
	pkgs:
	let
	toolchain = toolchainFor pkgs;
	rustPlatform = pkgs.makeRustPlatform {
		cargo = toolchain;
		rustc = toolchain;
	};
	in
	pkgs.callPackage ./nix/package.nix {
		inherit rustPlatform;
		runtimeLibs = import ./nix/runtime-libs.nix { inherit pkgs; };
	};
	in
	{
		packages = forAllSystems (
		system:
		let
		pkgs = pkgsFor system;
		in
		{
			aspid = aspidFor pkgs;
			default = self.packages.${system}.aspid;
		}
		);

		# `nix run github:marlstar/aspid`
		apps = forAllSystems (system: {
			aspid = {
				type = "app";
				program = nixpkgs.lib.getExe self.packages.${system}.aspid;
			};
			default = self.apps.${system}.aspid;
		});

		# Lets other flakes pull aspid into their package set: add `overlays.default`.
		overlays.default = _final: prev: {
			aspid = aspidFor prev;
		};

		devShells = forAllSystems (
		system:
		let
		pkgs = pkgsFor system;
		in
		{
			default = pkgs.mkShell {
				inputsFrom = [ self.packages.${system}.aspid ];
				packages = [
					(toolchainFor pkgs)
					pkgs.rust-analyzer
				];
				# The dev binary is unwrapped, so expose the runtime libraries it dlopens.
				LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (
				import ./nix/runtime-libs.nix { inherit pkgs; }
				);
			};
		}
		);

		formatter = forAllSystems (system: (pkgsFor system).nixfmt-rfc-style);
	};
}
