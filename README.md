# QuellDock

QuellDock is a Linux desktop application for managing AppImages, Flatpaks, AUR packages, and packages from the native distribution. It is built with Tauri 2, Rust, React, TypeScript, and Vite and is distributed as an AppImage.

## Features

- Search available Flatpak, AUR/Arch, and native distribution packages by name.
- Import, scan, launch, and integrate local AppImages; download direct `.AppImage` URLs and replace them atomically from their saved source.
- View installed applications from supported package sources in one library.
- Check Flatpak, Arch/AUR, and native package sources for available updates, then update selected packages.
- Preview executable name and argument arrays before install, update, and removal.
- Stream command output in the console and cancel active subprocesses.
- Persist settings and managed AppImage metadata to `~/.local/share/quelldock/state.json`.
- Generate XDG desktop launchers in `~/.local/share/applications/`.

Supported sources are **AppImage**, **Flatpak**, **yay**, **paru**, **pacman**, **apt**, **dnf**, and **zypper**. Provider availability depends on installed system tools and distribution support. Package search is best-effort and uses each provider's local/remote CLI interface.

## Project layout

```text
.
├── .github/workflows/ci.yml
├── src/
│   ├── components/       # Navigation, discovery, library, AppImage, console, settings
│   ├── App.tsx
│   ├── main.tsx
│   ├── styles.css
│   └── types.ts
└── src-tauri/
    ├── capabilities/
    ├── icons/
    └── src/
        ├── providers/    # AppImage, Flatpak, Arch/AUR, native distro
        ├── desktop_entry.rs
        ├── lib.rs
        ├── models.rs
        ├── runner.rs
        └── state.rs
```

## Architecture highlights

1. Tauri 2 hosts a React/TypeScript/Vite desktop interface and targets a Linux AppImage release.
2. Strongly typed Rust models define providers, candidates, installed apps, settings, and streamed job events.
3. `StateStore` persists settings and AppImage metadata under the XDG data directory using a temporary file and atomic rename.
4. The subprocess runner uses explicit argument vectors, emits stdout/stderr events, supports job cancellation, and forwards interactive input.
5. Provider modules query Flatpak, AUR/Arch, and the detected native package manager without constructing shell commands.
6. AppImage support includes local import, configured-folder scans, direct URL downloads, atomic source-based replacement, detached launch, and XDG desktop entries.
7. The UI previews install/update/uninstall actions, uses system `pkexec` for privileged managers, and exposes provider status, settings, and live logs.

## Build prerequisites

Install Node.js/npm and Rust/Cargo, then install the Tauri Linux development libraries: WebKitGTK 4.1, GTK 3, Ayatana AppIndicator, librsvg, and the platform build tools (`base-devel` on Arch or `build-essential` on Debian/Ubuntu). Building AppImages additionally needs `squashfs-tools`, `file`, `curl`, and FUSE tooling. The resulting image may require FUSE 2 at runtime; on systems without it, extract the AppImage with `--appimage-extract` and run the extracted application.

The SVG at `src-tauri/icons/icon.svg` is the source icon; the generated PNG/ICO/ICNS bundle icons are checked in. Regenerate them after editing the source with `npx tauri icon src-tauri/icons/icon.svg`.

## Development and release

```sh
make help
make setup
make dev
make check
make test
make build-appimage
```

The release image is written under `src-tauri/target/release/bundle/appimage/`. CI runs the frontend type/build checks, Rust tests, and an AppImage build on Ubuntu 24.04.

## Privileges and safety

QuellDock never interpolates package names into a shell command. It launches programs with explicit argument arrays and displays the same argument vector before mutating package actions. Native system package operations use `pkexec` when available and when the process is not already root; review the requested operation in the desktop authorization prompt. QuellDock does not collect or store administrator credentials.

`yay` and `paru` may ask interactive questions. The setting to pass `--noconfirm` is opt-in; leaving it disabled preserves the helper's interactive behavior. AUR packages are community-maintained and are not reviewed or endorsed by QuellDock. Inspect package sources and build scripts before installing them.

AppImage URLs are fetched by `curl` as direct arguments and must use HTTP or HTTPS. Only import binaries you trust. Desktop entries are created for managed AppImages and removed when those entries are unlinked.

## State and support

Settings and managed AppImage records are stored in `~/.local/share/quelldock/state.json`; AppImage files default to `~/Applications`. QuellDock does not track packages installed outside its own actions, but reports packages enumerated by the installed provider CLIs.

QuellDock is open source under the MIT license. Contributions and issue reports are welcome.
