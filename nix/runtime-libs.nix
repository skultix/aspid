# Libraries that aspid's GUI stack (wgpu/Vulkan + winit) loads at runtime via `dlopen`,
# so they must be on `LD_LIBRARY_PATH` rather than just in the build closure. Shared by the
# wrapped package and the dev shell.
{ pkgs }:
with pkgs;
[
	vulkan-loader # wgpu's Vulkan backend
	libGL # GL fallback / compositing
	libxkbcommon # winit keyboard handling
	wayland # winit Wayland backend
	# winit X11 backend (loaded through x11-dl)
	libx11
	libxcursor
	libxi
	libxrandr
	libxcb
]
