# Weft Launcher

A fast, small desktop launcher for [Weft](https://github.com/Weft-Rust/Weft), the voxel sandbox engine written from scratch in Rust and Vulkan.

Instead of downloading every new Weft release by hand and pasting files into a folder, the launcher lists the releases, installs them side by side, checks that each download is intact, and starts the game for you. Your worlds and settings are never touched.

> **Status:** version 1.0. Windows is tested. Linux and macOS builds are published but **experimental** (see [Platform support](#platform-support)).

<!-- Add screenshots here:
![Home](docs/screenshot-home.png)
![Versions](docs/screenshot-versions.png)
-->

## Table of contents

- [Features](#features)
- [Download and install](#download-and-install)
- [How to use it](#how-to-use-it)
- [Where your data lives](#where-your-data-lives)
- [Platform support](#platform-support)
- [Languages](#languages)
- [Security](#security)
- [Build from source](#build-from-source)
- [Project structure](#project-structure)
- [Releasing a new version](#releasing-a-new-version)
- [Troubleshooting](#troubleshooting)
- [FAQ](#faq)
- [Contributing](#contributing)
- [License](#license)
- [Credits](#credits)

## Features

- **Version list** pulled from the Weft GitHub releases, newest first, with a **New** marker for versions you have not installed yet.
- **One-click install** with a live progress bar (percentage and speed in Mbps).
- **Integrity check:** every download is verified against its SHA-256 before anything is extracted. No checksum means no install.
- **Side-by-side versions.** Keep as many versions as you like and pick one in the selector. The latest one is marked.
- **Safe by design:** the game's worlds, settings, logs and resource packs live outside the launcher and are never read, moved or deleted by it.
- **Play button** that starts the selected version, with only one game running at a time.
- **Safe delete:** removing a version is refused while a game is running, and uses a two-click confirmation (no pop-ups).
- **Game log:** the output of the game is saved per version, and the launcher can open it if the game closes with an error.
- **Open game folder** button in the instance card.
- **Remembers the last version you played.**
- **Single window:** opening the launcher twice just brings the first window to the front.
- **Automatic launcher updates** (signed), so you get new launcher versions without downloading anything.
- **4 languages:** English (default), Portuguese (BR), Spanish and Russian.
- **Native feel:** no text selection or context menu, except in the release notes.
- **Release notes** page that shows the changelog of every Weft release.

## Download and install

Go to the [**Releases**](../../releases) page and download the file for your system.

| System | File | Notes |
|---|---|---|
| Windows 10/11 (x64) | `Weft.Launcher_<version>_x64-setup.exe` | Installer. |
| Linux (x64) | `.AppImage` or `.deb` | See below. |
| macOS (Apple Silicon) | `.dmg` | Unsigned, see below. |

### Windows

1. Run the `...-setup.exe` installer.
2. Windows SmartScreen may say "Windows protected your PC", because the installer is not code-signed yet. Click **More info**, then **Run anyway**.
3. Open **Weft Launcher** from the Start menu.

The launcher uses WebView2, which is already included in Windows 11 and in up-to-date Windows 10.

### Linux (experimental)

AppImage:

```bash
chmod +x Weft.Launcher_*.AppImage
./Weft.Launcher_*.AppImage
```

Debian/Ubuntu:

```bash
sudo apt install ./Weft.Launcher_*.deb
```

You need Vulkan drivers for the game itself (for example `mesa-vulkan-drivers`, or your GPU vendor's driver). The launcher window needs WebKitGTK, which the `.deb` installs for you.

### macOS (experimental, Apple Silicon only)

1. Open the `.dmg` and drag **Weft Launcher** to **Applications**.
2. The app is not signed or notarized yet, so macOS may refuse to open it ("damaged" or "unidentified developer"). Either right-click the app and choose **Open**, or run:

```bash
xattr -dr com.apple.quarantine "/Applications/Weft Launcher.app"
```

Only Apple Silicon (M1 and newer) builds of Weft exist, so Intel Macs are not supported.

## How to use it

1. Open the **Versions** tab and press **Install** on a release.
2. Wait for the progress bar: download, verify, extract, finalize.
3. Go to **Home**. The newest installed version is selected automatically.
4. Press **Play**.

To remove a version you no longer want, go to **Versions** and click **Delete** twice (the second click says "Delete for real"). You cannot delete anything while a game is running.

## Where your data lives

The launcher and the game keep their files in different places on purpose.

**The game** stores your worlds, settings, logs and resource packs by itself. On Windows that is:

```
%APPDATA%\Weft
```

The launcher never reads from or writes to that folder. It only opens it in your file manager when you press **Open folder**. Deleting or updating a game version cannot affect your worlds.

**The launcher** stores its own files in its app data folder (on Windows `%APPDATA%\dev.weft.launcher`):

```
versions/<tag>/      installed game versions (safe to delete)
downloads/           temporary downloads
logs/<tag>.log       output of the game, one file per version
cache/               cached list of releases
```

Because worlds are outside the version folders, a save written by a newer version may not open in an older one. Weft is still early and the save format can change, so back up `%APPDATA%\Weft\saves` before trying an older version.

## Platform support

| Platform | Launcher builds | Tested |
|---|---|---|
| Windows x64 | yes | yes |
| Linux x64 | yes | no |
| macOS arm64 | yes | no |

"Tested" means a maintainer ran the full flow (install a version, play, delete). Linux and macOS are built by CI but nobody has verified them end to end yet. If you try one, please [open an issue](../../issues) with the result and the log from `logs/<tag>.log`, so the status can be updated.

## Languages

English is the default. Use the language button at the right end of the tab bar to switch between **EN**, **BR** (Portuguese), **ES** (Spanish) and **RU** (Russian). Your choice is remembered.

The Spanish and Russian texts need review by native speakers. Corrections are welcome (see [Contributing](#contributing)).

## Security

A launcher downloads and runs programs, so it is strict about it:

- Only HTTPS is used.
- Every download is checked against the SHA-256 published by GitHub before extracting. If the checksum is missing or wrong, nothing is installed.
- Archives are extracted with protection against path traversal ("zip slip"): absolute paths and entries that try to leave the destination folder are rejected.
- Installs are atomic: files go to a temporary folder first and are moved into place only when everything is verified. A failed install never leaves a half-installed version behind.
- The game is started with a fixed argument list. No shell commands are built from text.
- Launcher updates are signed and verified before they are applied.
- No telemetry and no analytics.
- The launcher only talks to GitHub (to list releases and download assets and its own updates).

Windows SmartScreen and the macOS warning appear because the launcher is not code-signed. They are expected for now.

## Build from source

Requirements:

- [Node.js](https://nodejs.org) (LTS)
- [Rust](https://rustup.rs) (stable)
- The [Tauri 2 prerequisites](https://tauri.app/start/prerequisites/) for your system (on Windows: Microsoft C++ Build Tools and WebView2; on Linux: WebKitGTK and a few libraries)

```bash
git clone https://github.com/Fernandes585/Weft-Launcher.git
cd REPO
npm ci
npm run tauri dev      # run in development mode
npm run tauri build    # build the installer for your system
```

Useful checks before sending a pull request:

```bash
cd src-tauri
cargo fmt --check
cargo clippy
cargo check
cd ..
npm run build
```

## Project structure

```
src/                     React + TypeScript frontend
  pages/                 Home, Versions, Notes
  components/            Sidebar, Tabs, language menu, banner, version actions
  hooks/                 install and game state
  i18n/                  translations (en, pt-BR, es, ru) and error mapping
  theme.css              colors and base styles (CSS variables)
  overrides.css          visual adjustments, loaded last
src-tauri/               Rust backend (Tauri 2)
  src/releases.rs        list and cache GitHub releases
  src/install.rs         download, verify, extract, install, remove
  src/launch.rs          start the game, track it, open folders and logs
  src/error.rs           typed errors sent to the UI
.github/workflows/       release build for Windows, Linux and macOS
```

Errors travel from Rust to the UI as `{ "code": "...", "detail": "..." }`, and the frontend translates the code into the selected language.

## Releasing a new version

1. Change the version in `package.json`, `src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json`.
2. Commit, then create and push a tag:

```bash
git tag v1.0.1
git push origin v1.0.1
```

3. GitHub Actions builds the three platforms and creates a **draft** release.
4. Check the draft, then press **Publish release**. Running launchers will offer the update.

The workflow needs two repository secrets, `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`, used to sign updates. Keep the private key safe: without it, existing users cannot receive updates.

## Troubleshooting

**"Open folder" says the game folder was not found.** The launcher learns the folder from the game's own log. Start the game once from the launcher, close it, and try again.

**The game closed with an error.** Press **Open log** in the error banner, or open `logs/<tag>.log` in the launcher's app data folder, and include it when you report the problem.

**The version list is empty or stale.** GitHub limits unauthenticated requests. The launcher caches the list and shows a notice when it is using the cache. Wait a while and press **Refresh list**.

**Install fails with a checksum error.** The download was corrupted or incomplete. Nothing was installed. Try again; if it repeats, open an issue.

**I cannot delete a version.** Close the game first. Deletion is blocked while any game is running or while a file of that version is locked.

## FAQ

**Does it need a Minecraft account, Java or any Mojang file?**
No. Weft ships its own assets.

**Will updating the launcher or the game delete my worlds?**
No. Worlds are stored outside the launcher and outside the version folders.

**Can I keep several versions installed?**
Yes. They live side by side, and you choose one on the Home tab.

**Is this the official launcher?**
It is a community launcher made with the permission of the Weft maintainer. It is not affiliated with Mojang or Microsoft.

**Does it support mods?**
Not yet. Weft mods are native libraries tied to an exact engine version, so mod support needs more design work.

## Contributing

Issues and pull requests are welcome. For code changes:

- Keep changes small and focused.
- All UI text goes through `src/i18n`. Add a key to all four language files.
- Run the checks listed under [Build from source](#build-from-source).
- Do not touch the game's data folder from launcher code.

Translation fixes (especially Spanish and Russian) are very welcome.

## License

[MPL-2.0](LICENSE), the same license as Weft.

## Credits

- [Weft](https://github.com/Weft-Rust/Weft) by its maintainers.
- Built with [Tauri](https://tauri.app), [React](https://react.dev) and [Vite](https://vitejs.dev).
- Pixel font: [Silkscreen](https://fonts.google.com/specimen/Silkscreen).
- Community: [Discord](https://discord.gg/aWECX7EPmV).
