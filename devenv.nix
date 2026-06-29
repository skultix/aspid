{ ... }: {
	languages.rust = {
		enable = true;
		channel = "stable";
		version = "1.96.0";
		components = ["cargo" "rustc" "rust-src" "rust-analyzer" "rustfmt" "clippy"];

		lsp.enable = true;

		# Linker
		wild.enable = true;
	};
}
