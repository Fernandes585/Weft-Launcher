import { useState, useMemo, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useI18n } from "../i18n/useI18n";
import { translateError } from "../i18n/errors";
import { useInstall } from "../hooks/useInstall";
import { useGameState } from "../hooks/useGameState";
import { PixelBanner } from "../components/PixelBanner";
import { compareTags } from "../utils/version";
import { SidebarFooter } from "../components/SidebarFooter";
import { formatDate } from "../utils/date";
import { summarizeNotes } from "../utils/notes";

export function Home({
  onGoToVersions,
  onGoToNotes,
  latestTag,
  releases,
}: {
  onGoToVersions: () => void;
  onGoToNotes: () => void;
  latestTag: string | null;
  releases: { tag: string; published_at: string; body: string }[];
}) {
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

  const latestRelease = useMemo(() => {
    if (releases.length === 0) return null;
    return (
      releases.find((r) => r.tag === latestTag) ??
      [...releases].sort((a, b) => new Date(b.published_at).getTime() - new Date(a.published_at).getTime())[0]
    );
  }, [releases, latestTag]);
  const notesSummary = latestRelease ? summarizeNotes(latestRelease.body) : "";

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
          <div className="hero-content">
            <h1 className="hero-title">Weft</h1>
            <p className="empty-message">{t("noVersionsInstalled")}</p>
            <button className="btn-flat btn-primary" onClick={onGoToVersions}>
              {t("goToVersionsTab")}
            </button>
          </div>
        </section>
      </main>
    );
  }

  return (
    <main className="page home-page">
      <section className="hero">
        <PixelBanner />
        <div className="hero-content">
          <div className="hero-header">
            <h1 className="hero-title">Weft</h1>
            <div className="hero-version-badge">{t("heroBadge", effectiveTag || "—")}</div>
          </div>
          <div className="hero-play-area">
            <button
              className="btn-play"
              onClick={() => effectiveTag && handleLaunch(effectiveTag)}
              disabled={!effectiveTag || playDisabled}
              aria-label={isSelectedRunning ? t("playing") : t("playButton")}
            >
              {isSelectedRunning ? t("playing") : t("playButton")}
            </button>
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
        </div>
      </section>

      <div className="home-lower">
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

        <section className="card notes-summary-card">
          <div className="card-header">
            <h2 className="card-title">
              {t("latestNotesTitle")}{latestRelease ? ` · ${latestRelease.tag}` : ""}
            </h2>
          </div>
          <div className="card-body">
            {latestRelease && (
              <div className="notes-summary-date">{formatDate(latestRelease.published_at, locale)}</div>
            )}
            <div className="notes-summary-body">{notesSummary || t("noChangelog")}</div>
            <button className="btn-flat btn-secondary" onClick={onGoToNotes}>
              {t("viewAllNotes")}
            </button>
          </div>
        </section>
        <SidebarFooter />
      </div>
    </main>
  );
}