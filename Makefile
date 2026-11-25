.PHONY: help setup dev check test build-appimage clean

help: ## List available targets
	@awk 'BEGIN {FS = ":.*##"} /^[a-zA-Z0-9_-]+:.*##/ {printf "  %-18s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

setup: ## Check Linux build dependencies and install frontend packages
	@echo "QuellDock requires Node.js/npm, Rust/Cargo, and Tauri Linux libraries:"
	@echo "  webkit2gtk-4.1, gtk3, libayatana-appindicator3, librsvg, squashfs-tools, fuse2"
	@echo "  plus base-devel (Arch) or build-essential (Debian/Ubuntu)"
	@for bin in node npm cargo; do command -v $$bin >/dev/null || echo "Missing command: $$bin"; done
	@if command -v pkg-config >/dev/null; then \
	  for module in webkit2gtk-4.1 gtk+-3.0 ayatana-appindicator3-0.1 librsvg-2.0; do \
	    pkg-config --exists $$module || echo "Missing development library: $$module"; \
	  done; \
	else echo "Missing command: pkg-config (cannot check Tauri development libraries)"; fi
	@for bin in mksquashfs fusermount; do command -v $$bin >/dev/null || echo "Missing AppImage tool: $$bin"; done
	@npm install

dev: ## Run the Tauri development application
	npm run tauri dev

check: ## Type-check/build the frontend and check all Rust targets
	npx tsc --noEmit
	npm run build
	cargo check --manifest-path src-tauri/Cargo.toml --all-targets

test: ## Run frontend and Rust tests
	npm test -- --run
	cargo test --manifest-path src-tauri/Cargo.toml

build-appimage: ## Build the release AppImage and print its exact path
	npm run tauri build -- --bundles appimage
	@find src-tauri/target/release/bundle/appimage -maxdepth 1 -type f -name '*.AppImage' -print

clean: ## Remove generated frontend and Rust build artifacts
	rm -rf dist src-tauri/target
