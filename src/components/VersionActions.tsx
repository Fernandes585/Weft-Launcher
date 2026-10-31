import { useState } from "react";
import { useInstall, InstalledVersion, InstallProgress } from "../hooks/useInstall";
import { t } from "../i18n";
import "./install.css";

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

  const [confirmDelete, setConfirmDelete] = useState(false);
  const [localProgress, setLocalProgress] = useState<InstallProgress | null>(null);

  const isCurrentlyInstalling = loading && localProgress?.stage !== "done" && localProgress?.stage !== "error";

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

    const confirmed = window.confirm(t("confirmDelete", release.tag));
    if (!confirmed) {
      setConfirmDelete(false);
      return;
    }

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

  if (installedVersion) {
    return (
      <div className="version-actions installed">
        <span className="installed-badge">{t("installedBadge")}</span>
        <button
          className="delete-btn"
          onClick={handleRemove}
          disabled={loading}
          aria-label={t("deleteButton")}
        >
          {confirmDelete ? t("deleteButton") + "?" : t("deleteButton")}
        </button>
        {confirmDelete && (
          <span className="confirm-hint">{t("confirmDelete", release.tag)}</span>
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