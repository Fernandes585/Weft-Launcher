import { useState, useEffect, useCallback } from "react";
import { useInstall, InstalledVersion, InstallProgress } from "../hooks/useInstall";
import { useGameState } from "../hooks/useGameState";
import { t } from "../i18n";
import { listen } from "@tauri-apps/api/event";

interface VersionActionsProps {
  release: {
    tag: string;
    asset: {
      name: string;
      url: string;
      size: number;
      sha256: string | null;
    };
  };
  installedVersion: InstalledVersion | null;
  onInstallStart?: () => void;
  onInstallComplete?: () => void;
}

export function VersionActions({
  release,
  installedVersion,
  onInstallStart,
  onInstallComplete,
}: VersionActionsProps) {
  const {
    error,
    loading,
    install,
    remove,
    clearError,
  } = useInstall();

  const { anyRunning, isRunning, launch, runningTags } = useGameState();

  const [confirmDelete, setConfirmDelete] = useState(false);
  const [localProgress, setLocalProgress] = useState<InstallProgress | null>(null);
  const [launchError, setLaunchError] = useState<string | null>(null);

  const isCurrentlyInstalling = loading && localProgress?.stage !== "done" && localProgress?.stage !== "error";
  const thisVersionRunning = isRunning(release.tag);

  const translateLaunchError = useCallback((err: string): string => {
    if (err.startsWith("Versão já está rodando:") || err.startsWith("This version is already running:")) {
      return t("errorAlreadyRunning");
    }
    if (err.startsWith("Outro jogo já está rodando:") || err.startsWith("Another game is running:")) {
      const match = err.match(/rodando:\s*(.+)$|running:\s*(.+)$/i);
      const otherTag = match?.[1] || match?.[2] || "";
      return t("errorGameRunning", otherTag);
    }
    if (err.startsWith("Executável não encontrado:") || err.startsWith("Executable not found:")) {
      return t("errorExecutableNotFound", err.replace(/^(Executável não encontrado:|Executable not found:)\s*/, ""));
    }
    if (err.startsWith("Versão em uso") || err.startsWith("Version in use")) {
      return t("errorVersionInUse", release.tag);
    }
    return err;
  }, [release.tag]);

  useEffect(() => {
    if (confirmDelete) {
      const timer = setTimeout(() => {
        setConfirmDelete(false);
      }, 4000);
      return () => clearTimeout(timer);
    }
  }, [confirmDelete]);

useEffect(() => {
  let unlisten: (() => void) | undefined;
  let cancelled = false;

  listen<InstallProgress>("install-progress", (event) => {
    if (event.payload.tag !== release.tag) return;
    setLocalProgress(event.payload);
  }).then((fn) => {
    if (cancelled) fn();
    else unlisten = fn;
  });

  return () => {
    cancelled = true;
    unlisten?.();
  };
}, [release.tag]);

  const handleInstall = async () => {
    if (!release.asset.sha256) {
      clearError();
      const err = t("errorHashMissing");
      setLocalProgress({ tag: release.tag, stage: "error", message: err });
      return;
    }

    clearError();
    onInstallStart?.();
    setLocalProgress({ tag: release.tag, stage: "downloading", percent: 0 });

    try {
      await install(
        release.tag,
        release.asset.url,
        release.asset.name,
        release.asset.sha256
      );
      onInstallComplete?.();
    } catch {
      // Error already handled in hook
    }
  };

  const handleRemove = async () => {
    if (!confirmDelete) {
      setConfirmDelete(true);
      return;
    }

    // Second click - proceed with deletion
    clearError();
    setLocalProgress({ tag: release.tag, stage: "verifying" });

    try {
      await remove(release.tag);
      setLocalProgress({ tag: release.tag, stage: "done", percent: 100 });
      onInstallComplete?.();
    } catch {
      // Error already handled in hook
    } finally {
      setConfirmDelete(false);
    }
  };

  const handleLaunch = async () => {
    setLaunchError(null);
    const err = await launch(release.tag);
    if (err) {
      setLaunchError(translateLaunchError(err));
    }
  };

  const getProgressLabel = (stage: InstallProgress["stage"]): string => {
    switch (stage) {
      case "downloading":
        return t(
          "progressDownloading",
          String(localProgress?.percent ?? 0),
          (localProgress?.speedMbps ?? 0).toFixed(1)
        );
      case "verifying":
        return t("progressVerifying");
      case "extracting":
        return t("progressExtracting");
      case "finalizing":
        return t("progressFinalizing");
      case "done":
        return t("installedBadge");
      case "error":
        return localProgress?.message || t("error");
      default:
        return "";
    }
  };

  const otherRunningTag = anyRunning ? Array.from(runningTags).find((t) => t !== release.tag) : null;
  const playTooltip = anyRunning && otherRunningTag ? t("playDisabledTooltip", otherRunningTag) : undefined;

  const deleteDisabled = anyRunning || loading;
  const playDisabled = anyRunning || thisVersionRunning || loading;

  if (installedVersion) {
    return (
      <div className="version-actions installed">
        <span className="installed-badge">{t("installedBadge")}</span>
        <button
          className="play-btn"
          onClick={handleLaunch}
          disabled={playDisabled}
          title={playTooltip}
          aria-label={thisVersionRunning ? t("playing") : t("playButton")}
        >
          {thisVersionRunning ? t("playing") : t("playButton")}
        </button>
        <button
          className={`delete-btn${confirmDelete ? " confirming" : ""}`}
          onClick={handleRemove}
          disabled={deleteDisabled}
          title={anyRunning ? t("deleteDisabledTooltip") : undefined}
          aria-label={confirmDelete ? t("deleteForReal") : t("deleteButton")}
        >
          {confirmDelete ? t("deleteForReal") : t("deleteButton")}
        </button>
        {launchError && (
          <div className="install-error" role="alert">
            {launchError}
          </div>
        )}
      </div>
    );
  }

  return (
    <div className="version-actions">
      {isCurrentlyInstalling && (
        <div className="progress-container">
          <div
            className="progress-bar"
            role="progressbar"
            aria-valuenow={localProgress?.percent ?? 0}
            aria-valuemin={0}
            aria-valuemax={100}
            aria-label={getProgressLabel(localProgress?.stage ?? "downloading")}
          >
            <div
              className="progress-fill"
              style={{ width: `${localProgress?.percent ?? 0}%` }}
            />
          </div>
          <span className="progress-label">{getProgressLabel(localProgress?.stage ?? "downloading")}</span>
        </div>
      )}

      {error && (
        <div className="install-error" role="alert">
          {error}
        </div>
      )}

      <button
        className="install-btn"
        onClick={handleInstall}
        disabled={loading}
        aria-label={t("installButton")}
      >
        {isCurrentlyInstalling ? t("installing") : t("installButton")}
      </button>
    </div>
  );
}