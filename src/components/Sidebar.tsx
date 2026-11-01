import { useI18n } from "../i18n/useI18n";
import { useGameState } from "../hooks/useGameState";

export function Sidebar() {
  const { t } = useI18n();
  const { anyRunning } = useGameState();

  return (
    <aside className="sidebar">
      <div className="sidebar-header">
        <h2>{t("appTitle")}</h2>
      </div>
      <div className="sidebar-instance">
        <div className="instance-info">
          <span className="instance-name">{t("instanceName")}</span>
          <span className={`instance-status ${anyRunning ? "running" : "ready"}`}>
            {anyRunning ? t("statusPlaying") : t("statusReady")}
          </span>
        </div>
      </div>
      <div className="sidebar-footer">
        <a href="https://discord.gg" target="_blank" rel="noopener noreferrer">
          {t("linkDiscord")}
        </a>
        <a href="https://github.com/Weft-Rust/Weft" target="_blank" rel="noopener noreferrer">
          {t("linkWebsite")}
        </a>
      </div>
    </aside>
  );
}