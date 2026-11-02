import { useState, useMemo, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useI18n } from "../i18n/useI18n";
import { translateError } from "../i18n/errors";
import { useInstall } from "../hooks/useInstall";
import { useGameState } from "../hooks/useGameState";
import { PixelBanner } from "../components/PixelBanner";
import { compareTags } from "../utils/version";

export function Home({ onGoToVersions, latestTag }: { onGoToVersions: () => void; latestTag: string | null }) {
  const { t, locale } = useI18n();
  const { installedVersions } = useInstall();
  const { anyRunning, isRunning, launch } = useGameState();

  const [selectedTag, setSelectedTag] = useState<string | null>(null);
  const [launchError, setLaunchError] = useState<string | null>(null);
  const [gameError, setGameError] = useState<{ tag: string; exitCode: number } | null>(null);
  const [gameFolderPath, setGameFolderPath] = useState<string | null>(null);

  const sortedVersions = useMemo(
  () => [...installedVersions].sort((a, b) => compareTags(b.tag, a.tag)),
  [installedVersions]
  );

  // Initialize selectedTag from localStorage (lastPlayed) on mount
  useEffect(() => {
    try {
      const lastPlayed = localStorage.getItem("lastPlayed");
      if (lastPlayed && installedVersions.some((v) => v.tag === lastPlayed)) {
        setSelectedTag(lastPlayed);
      }
    } catch {
      // ignore localStorage errors
    }
  }, [installedVersions]);

  // Listen for game-state events to detect error exit codes
  useEffect(() => {
    let unlisten: (() => void) | null = null;
    let mounted = true;

    listen<{ tag: string; state: string; exit_code: number | null }>("game-state", (event) => {
      if (!mounted) return;
      const { tag, state, exit_code } = event.payload;
      if (state === "stopped" && exit_code !== null) {
        // Only show error for CLEARLY abnormal exits:
        // - Negative exit codes on Windows = exception codes (e.g., -1073741819 = Access Violation, STATUS_DLL_NOT_FOUND, etc.)
        // - Codes >= 128 = Unix signals (128 + signal number, e.g., SIGSEGV=11 -> 139, SIGKILL=9 -> 137)
        // All small positive codes (0, 1, 2, 3... up to 127) = normal closes (X button, Alt+F4, Quit Game, etc.)
        // Weft may return various small codes for normal shutdown depending on code path.
        const isLikelyCrash = exit_code < 0 || exit_code >= 128;
        if (isLikelyCrash) {
          setGameError({ tag, exitCode: exit_code });
        }
      }
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      mounted = false;
      if (unlisten) unlisten();
    };
  }, []);

  const effectiveTag = selectedTag ?? sortedVersions[0]?.tag ?? null;

  const selectedVersion = useMemo(
  () => sortedVersions.find((v) => v.tag === effectiveTag) ?? null,
  [sortedVersions, effectiveTag]
);

  const handleLaunch = async (tag: string) => {
    setLaunchError(null);
    setGameError(null);
    const err = await launch(tag);
    if (err) {
      setLaunchError(translateError(err));
    } else {
      // Save lastPlayed on successful launch
      try {
        localStorage.setItem("lastPlayed", tag);
      } catch {
        // ignore localStorage errors
      }
    }
  };

  const handleOpenLog = async (tag: string) => {
    setGameError(null);
    try {
      await invoke("open_log", { tag });
    } catch (err) {
      setLaunchError(translateError(err));
    }
  };

  const handleOpenFolder = async () => {
    if (!effectiveTag) return;
    setLaunchError(null);
    try {
      const path = await invoke<string>("open_game_folder", { tag: effectiveTag });
      setGameFolderPath(path);
    } catch (err) {
      setLaunchError(translateError(err));
    }
  };

  const isSelectedRunning = effectiveTag ? isRunning(effectiveTag) : false;
  const playDisabled = anyRunning || isSelectedRunning;

  if (installedVersions.length === 0) {
    return (
      <main className="page home-page">
        <section className="hero empty">
          <PixelBanner />
          <h1 className="hero-title">Weft</h1>
          <p className="empty-message">{t("noVersionsInstalled")}</p>
          <button className="btn-flat btn-primary" onClick={onGoToVersions}>
            {t("goToVersionsTab")}
          </button>
        </section>
      </main>
    );
  }

  return (
    <main className="page home-page">
      <section className="hero">
        <PixelBanner />
        <div className="hero-header">
          <h1 className="hero-title">Weft</h1>
          <div className="hero-version-badge">
            {t("heroBadge", effectiveTag || "—")}
          </div>
        </div>
        <div className="hero-play-area">
          <div className="version-selector">
            <label htmlFor="version-select" className="visually-hidden">
              {t("selectVersionLabel")}
            </label>
            <select
              id="version-select"
              value={effectiveTag || ""}
              onChange={(e) => setSelectedTag(e.target.value || null)}
              disabled={anyRunning}
            >
              {sortedVersions.map((v) => (
              <option key={v.tag} value={v.tag}>
              {latestTag && v.tag === latestTag ? `${v.tag} — ${t("latestBadge")}` : v.tag}
              </option>
             ))}
            </select>
          </div>
          <button
              className="btn-play"
              onClick={() => effectiveTag && handleLaunch(effectiveTag)}
              disabled={!effectiveTag || playDisabled}
              aria-label={isSelectedRunning ? t("playing") : t("playButton")}
            >
            {isSelectedRunning ? t("playing") : t("playButton")}
          </button>
        </div>
        {launchError && (
          <div className="launch-error" role="alert">
            {launchError}
          </div>
        )}
        {gameError && (
          <div className="launch-error" role="alert">
            <span>{t("gameClosedError", String(gameError.exitCode))}</span>
            <button className="btn-flat btn-secondary" onClick={() => handleOpenLog(gameError.tag)}>
              {t("openLogButton")}
            </button>
          </div>
        )}
      </section>

      <section className="card">
        <div className="card-header">
          <h2 className="card-title">{t("instanceCardTitle")}</h2>
        </div>
        <div className="card-body">
          {selectedVersion ? (
            <>
              <div className="detail-row">
                <span className="detail-label">{t("versionLabel")}</span>
                <span className="detail-value">{selectedVersion.tag}</span>
              </div>
              <div className="detail-row">
                <span className="detail-label">{t("installedAtLabel")}</span>
                <span className="detail-value">
                  {new Date(selectedVersion.installed_at).toLocaleString(locale)}
                </span>
              </div>
              <div className="detail-row">
                <span className="detail-label">{t("executableLabel")}</span>
                <span className="detail-value path">{selectedVersion.executable}</span>
              </div>
              <div className="detail-row">
                <span className="detail-label">{t("gameFolderTitle")}</span>
                <span className="detail-value path">{gameFolderPath || t("gameFolderPath")}</span>
              </div>
              <div className="detail-row">
                <button className="btn-flat btn-secondary btn-open-folder" onClick={handleOpenFolder}>
                  {t("openFolderButton")}
                </button>
              </div>
            </>
          ) : (
            <p className="select-version-hint">{t("selectVersionHint")}</p>
          )}
        </div>
      </section>
    </main>
  );
}