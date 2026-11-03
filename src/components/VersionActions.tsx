import { useState, useEffect } from "react";
import { useInstall, InstallProgress } from "../hooks/useInstall";
import { useGameState } from "../hooks/useGameState";
import { useI18n } from "../i18n/useI18n";
import { translateError } from "../i18n/errors";

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
}

export function VersionActions({ release }: VersionActionsProps) {
  const { t } = useI18n();
  const {
    install,
    remove,
    clearError,
    installedVersions,
    getProgress,
    isInstalling,
    isRemoving,
    getError,
  } = useInstall();

  const { anyRunning, isRunning, launch, runningTags } = useGameState();

  const [confirmDelete, setConfirmDelete] = useState(false);
  const [launchError, setLaunchError] = useState<string | null>(null);
  const [hashError, setHashError] = useState<string | null>(null);

  const installedVersion = installedVersions.find((v) => v.tag === release.tag) ?? null;

  const progress = getProgress(release.tag);
  const installing = isInstalling(release.tag);
  const removing = isRemoving(release.tag);
  const installError = getError(release.tag);
  const thisVersionRunning = isRunning(release.tag);

  useEffect(() => {
    if (confirmDelete) {
      const timer = setTimeout(() => {
        setConfirmDelete(false);
      }, 4000);
      return () => clearTimeout(timer);
    }
  }, [confirmDelete]);

  const handleInstall = async () => {
    setHashError(null);

    if (!release.asset.sha256) {
      clearError(release.tag);
      setHashError(t("errorHashMissing"));
      return;
    }

    clearError(release.tag);

    try {
      await install(
        release.tag,
        release.asset.url,
        release.asset.name,
        release.asset.sha256
      );
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
    clearError(release.tag);

    try {
      await remove(release.tag);
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
      setLaunchError(translateError(err));
    }
  };

  const getProgressLabel = (current: InstallProgress): string => {
    switch (current.stage) {
      case "downloading":
        return t(
          "progressDownloading",
          String(current.percent ?? 0),
          (current.speedMbps ?? 0).toFixed(1)
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
        return current.message || t("error");
      default:
        return "";
    }
  };

  const otherRunningTag = anyRunning ? Array.from(runningTags).find((t) => t !== release.tag) : null;
  const playTooltip = anyRunning && otherRunningTag ? t("playDisabledTooltip", otherRunningTag) : undefined;

  const deleteDisabled = anyRunning || removing;
  const playDisabled = anyRunning;

  if (installedVersion) {
    return (
      <div className="version-actions installed">
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
        {installError && (
          <div className="install-error" role="alert">
            {installError}
          </div>
        )}
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
      {installing && progress && (
        <div className="progress-container">
          <div
            className="progress-bar"
            role="progressbar"
            aria-valuenow={progress.percent ?? 0}
            aria-valuemin={0}
            aria-valuemax={100}
            aria-label={getProgressLabel(progress)}
          >
            <div
              className="progress-fill"
              style={{ width: `${progress.percent ?? 0}%` }}
            />
          </div>
          <span className="progress-label">{getProgressLabel(progress)}</span>
        </div>
      )}

      {(hashError || installError) && (
        <div className="install-error" role="alert">
          {hashError || installError}
        </div>
      )}

      <button
        className="install-btn"
        onClick={handleInstall}
        disabled={installing}
        aria-label={t("installButton")}
      >
        {installing ? t("installing") : t("installButton")}
      </button>
    </div>
  );
}