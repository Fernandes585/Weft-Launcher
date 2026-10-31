# Weft Launcher: project rules

## What this is
A desktop launcher for Weft (a voxel sandbox engine written in Rust, https://github.com/Weft-Rust/Weft).
It downloads Weft releases, installs them side by side, keeps player data (worlds, settings, mods) OUTSIDE the game folders, and starts the game.
Weft ships its own textures, so no Mojang account, Java or Minecraft files are involved.
Platform order: Windows first. Keep the code cross-platform (Linux x64, macOS arm64) from day one, but only test on Windows for now.

## How to talk to the user
- Reply in Brazilian Portuguese. Code, comments, identifiers and commit messages stay in English.
- The user is learning: explain what you did in short, plain sentences and give exact commands for Windows PowerShell.

## Stack (do not change without asking)
- Tauri 2. Rust backend in `src-tauri/`, frontend React + TypeScript + Vite in `src/`.
- Rust crates: tokio, reqwest (with rustls, no openssl), serde, serde_json, sha2, zip, flate2, tar, thiserror.
- Plain CSS with CSS variables for the first versions. No extra UI frameworks until asked.
- Keep dependencies minimal and say why each new one is needed.

## Weft release facts (taken from a real release page)
- Releases live on GitHub. The repo is a single constant in `src-tauri/src/config.rs`: `WEFT_REPO = "Weft-Rust/Weft"` (confirmed: releases are at https://github.com/Weft-Rust/Weft/releases).
- Tags look like `v26.10.4` (year.month.patch). There is one channel ("Latest"), released often.
- Assets per release:
  - `Weft-<ver>-windows-x64.zip`
  - `Weft-<ver>-linux-x64.tar.gz`
  - `Weft-<ver>-macos-arm64.tar.gz`
  - plus GitHub's automatic "Source code" archives (ignore those).
- Every asset has a sha256 (GitHub API field `digest`, format `sha256:<hex>`).
- The Windows zip contains `weft.exe`, `assets/` and `run/`. It may or may not sit inside a top-level folder; the Linux/macOS tarballs extract to a `Weft/` folder. Normalize so the executable always ends at `versions/<tag>/weft` (`weft.exe` on Windows).
- The game accepts `--run-dir <path>`, which overrides where settings, saves, logs and mods go. ALWAYS launch with it, so the launcher controls the data location.
- `weft --server` runs a world without a window (not needed yet).
- Mods are native libraries (zip files) built for one exact engine version. They can break between versions: warn the user when switching versions while mods exist.
- macOS builds are unsigned and need `xattr -dr com.apple.quarantine Weft` (later phase).
- Save and mod formats can still change between versions (the project is early).
- UNKNOWN for now: the subfolder names inside `run/` other than the mods folder. Never hardcode them: inspect a real run dir or ask the user.

## Folder layout (inside Tauri's app data dir)
```
versions/<tag>/            disposable game installs
instances/<name>/run/      player data, passed to the game with --run-dir
downloads/                 temporary downloads
backups/<name>/<time>.zip  automatic backups
cache/releases.json        cached GitHub response (ETag)
```

## Hard rules (data safety)
1. Never delete, move or overwrite anything under `instances/` unless the user explicitly asks and confirms. Updating means adding a new `versions/<tag>/`; old versions are removed only by the user.
2. Before launching an instance with a different version than the one it last used, back up its run dir to `backups/`.
3. Install atomically: download to `downloads/`, verify, extract to a temp dir inside `versions/`, then rename. On any failure, clean up partial files.
4. Verify the sha256 of every download before extracting. If the digest is missing, stop and ask the user.
5. HTTPS only. Protect against zip-slip and tar path traversal: reject absolute paths and entries that escape the destination.
6. Start the game with `std::process::Command` and an argument array. Never build shell command strings.
7. No telemetry or analytics.
8. Do not copy code or assets from the Horizon launcher. It is a different project. UX inspiration only.

## Process
- Work one step at a time. Each step must compile and run (`cargo check`, `cargo clippy`, `cargo fmt`, `npm run build`) before you finish, and you must tell the user exactly how to test it.
- Do not add features that were not requested. If something is unknown (file names, flags, paths), inspect or ask. Never invent it.
- Use cross-platform path APIs. Put platform-specific code behind `#[cfg(target_os = "...")]`.
- Use typed errors. Show readable pt-BR messages in the UI and keep details in the log.
- All UI strings go through `src/i18n` (pt-BR and en) from the start.
- Commit after each finished step with a clear message (if git is initialized).
