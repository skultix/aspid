{ pkgs, lib, ... }: {
	languages.rust = {
		enable = true;
		channel = "stable";
		version = "1.96.0";
		components = ["cargo" "rustc" "rust-src" "rust-analyzer" "rustfmt" "clippy"];

		lsp.enable = true;

		# Linker
		wild.enable = true;
	};

	packages = with pkgs; [
		# Wayland
		wayland
		libxkbcommon
		# X11
		xorg.libX11
		xorg.libXcursor
		xorg.libXrandr
		xorg.libXi
		libGL
	];

	env.LD_LIBRARY_PATH = lib.makeLibraryPath (with pkgs; [
		wayland
		libxkbcommon
		libX11
		libXcursor
		libXrandr
		libXi
		libGL
		vulkan-loader  # if using wgpu/vulkan
	]);
}
