import { useI18n } from "../i18n/useI18n";
import { useGameState } from "../hooks/useGameState";
import { useLauncherUpdate } from "../hooks/useLauncherUpdate";

export function UpdateButton() {
  const { t } = useI18n();
  const { anyRunning } = useGameState();
  const { status, version, percent, errorMsg, install } = useLauncherUpdate();

  if (status === "idle") return null;

  const downloading = status === "downloading";
  const label = downloading
    ? t("updateDownloading", String(percent))
    : status === "error"
      ? t("updateFailed")
      : t("updateAvailable");
  const title = anyRunning
    ? t("updateCloseGame")
    : status === "error"
      ? (errorMsg ?? "")
      : t("updateTooltip", version ?? "");

  return (
    <button
      type="button"
      className="update-btn"
      onClick={install}
      disabled={downloading || anyRunning}
      title={title}
    >
      {label}
    </button>
  );
}