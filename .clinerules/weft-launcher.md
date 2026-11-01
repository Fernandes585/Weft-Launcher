# Weft Launcher: project rules

## What this is
Desktop launcher for Weft (voxel sandbox engine in Rust, https://github.com/Weft-Rust/Weft).
It lists GitHub releases, installs them side by side, and starts the game.
Weft ships its own assets: no Mojang account, Java or Minecraft files.
Windows is the only tested platform. Keep code cross-platform, but never claim Linux/macOS works until tested.

## How to talk to the user
- Reply in Brazilian Portuguese. Code, comments, identifiers and commit messages in English.
- The user is learning: short plain sentences, exact PowerShell commands.
- When finished, show the FULL output of `cargo check` (run inside `src-tauri`) and `npm run build`, and say exactly how to test.

## Working rules (the most important section)
- Create and edit files ONLY with the editor tools. NEVER write files through shell, node, echo, cmd or `c:\temp`.
- NEVER rewrite a whole existing file. Make small edits. For big new code, create new small files (max ~120 lines each).
- If an edit fails, STOP and tell the user. Do not try another method on your own.
- Never run `npm run tauri dev` or leave servers running. The user tests.
- Do not add features, dependencies or files that were not requested. For each new dependency, say why it is needed.
- If something is unknown (file names, flags, paths), inspect the code or ask. Never invent it.
- Do not run the game `weft.exe` to probe flags: `weft --help` ignores the flag and just opens the game.

## Stack
- Tauri 2. Rust backend in `src-tauri/src`, React + TypeScript + Vite in `src`.
- Rust crates in use: tokio, reqwest (rustls), serde, serde_json, sha2, zip, flate2, tar, thiserror, futures-util, time.
- Plain CSS with variables. No UI framework.

## Data layout (inside Tauri's app data dir, `dev.weft.launcher`)
```
versions/<tag>/weft.exe   disposable installs (assets/ and run/ come from the zip)
downloads/                temporary downloads
logs/<tag>.log            stdout/stderr of the game
cache/                    cached GitHub responses
```
- The GAME keeps worlds, config, logs and resource packs by itself in `%APPDATA%\Weft` (Windows). That folder is outside the launcher's control.
- Do NOT create `instances/` or `backups/`. `src-tauri/src/instances.rs` is legacy: do not edit it unless asked.

## Hard rules (data safety)
1. NEVER read, write, move or delete anything inside `%APPDATA%\Weft`. The only allowed access is opening it in the file manager (read-only).
2. Launch the game with NO arguments, working directory `versions/<tag>/`, hidden console on Windows. `--run-dir` appears in the Weft README but is untested: do not use it unless asked.
3. Install atomically: download to `downloads/`, verify SHA-256, extract to `versions/.tmp-<tag>`, check that the executable exists there, then rename to `versions/<tag>/`. On any failure delete the partial file and the temp dir. A folder without the executable is not "installed".
4. The SHA-256 of every download is mandatory. If the digest is missing, stop with `HashMissing`.
5. HTTPS only. Block zip-slip and tar path traversal: reject absolute paths, `..`, `Prefix` and `RootDir` components with an error. Do NOT validate paths with `canonicalize` (it silently skipped every file on Windows because of the `\\?\` prefix).
6. Only one game at a time. Check and register "running" inside the same Mutex lock, BEFORE spawning, and never hold a `std::sync::MutexGuard` across an `.await`.
7. `remove_version` refuses while any game runs (`GameRunning`). It renames `versions/<tag>` to `versions/.removing-<tag>` first (fails if a file is locked: `VersionInUse`) and only then deletes.
8. Spawn processes with `std::process::Command` / `tokio::process::Command` and an argument array. Never build shell strings.
9. No telemetry. Do not copy code, art or the name of the Horizon launcher (visual inspiration only).

## Backend contract (do not break it)
- Commands: `list_versions(forceRefresh)`, `install_version(tag, assetUrl, assetName, expectedSha256)`, `get_installed_versions`, `remove_version(tag)`, `launch_version(tag)`, `get_running_versions`.
- Events: `install-progress` {tag, stage, downloaded, total, percent, speedMbps} and `game-state` {tag, state: "running" | "stopped", exitCode}. Field names are camelCase.
- `invoke` arguments are camelCase in JavaScript (`assetUrl`, not `asset_url`).
- Errors: typed codes in Rust (e.g. `AlreadyRunning`, `HashMissing`, `Sha256Mismatch`, `GameRunning`, `VersionInUse`), translated in the frontend. Never match error text with `startsWith`.
- Weft release assets: `Weft-<ver>-windows-x64.zip`, `Weft-<ver>-linux-x64.tar.gz`, `Weft-<ver>-macos-arm64.tar.gz`. Tags look like `v26.10.4`. The repo name lives only in `src-tauri/src/config.rs`.

## Frontend rules
- All UI text goes through `src/i18n` with 4 languages: `en` (default), `pt-BR`, `es`, `ru`. Add a key to ALL four JSON files and to `I18nMessages` in `index.ts`. Only ADD lines to a JSON, never rewrite the file. Placeholders are `{0}`, `{1}`.
- Shared state lives in providers/hooks (`useGameState`, `useInstall`). Do not create a separate copy of this state per list item: pass props or use the provider. After install or delete, call the refresh function so cards update without restarting.
- Pages: `src/pages/Home.tsx`, `Versions.tsx`, `Notes.tsx`. Components in `src/components`. The Delete button lives only in the Versions page.
- Style: colors are CSS variables in `src/theme.css`. `src/overrides.css` is imported LAST in `main.tsx` and wins over everything: put visual adjustments there. Square corners, thick borders, flat buttons, no shadows or gradients.
- Fonts: system font everywhere. The pixel font (Silkscreen) is ONLY for the "WEFT" title, the version selector and the Play button.
- The app must feel native: nothing is selectable or draggable except the release notes page and inputs; right click, F12, Ctrl+Shift+I/J/C and Ctrl+U are blocked (set in `main.tsx` and `overrides.css`).
- Never use `window.confirm` or `alert`. Delete uses the two-click pattern (Delete, then "Delete for real", reverts after 4 s).
- Use `localStorage` only inside try/catch.

## Process
- One small step at a time. Each step must pass `cargo check`, `cargo clippy` and `npm run build` before you finish.
- Put platform-specific code behind `#[cfg(target_os = "...")]`. Executables extracted on Linux/macOS need the execute bit (`chmod +x`).
- The launcher license is MPL-2.0 (same as Weft), pending the owner's confirmation.
- Commit after each finished step, with a clear message, only if the user asks.
