import { useI18n } from "../i18n/useI18n";
import { useGameState } from "../hooks/useGameState";
import { SidebarFooter } from "./SidebarFooter";
import weftIcon from "../assets/weft.webp";

export function Sidebar() {
  const { t } = useI18n();
  const { anyRunning } = useGameState();

  return (
    <aside className="sidebar">
      <div className="sidebar-header">
        <h2>{t("instancesTitle")}</h2>
      </div>
      <div className="sidebar-instance">
        <div className="instance-item">
          <img className="instance-icon" src={weftIcon} alt="" />
          <div className="instance-info">
            <span className="instance-name">{t("instanceName")}</span>
            <span className="instance-status">
              <span className="status-dot" />
              {anyRunning ? t("statusPlaying") : t("statusReady")}
            </span>
          </div>
        </div>
      </div>
      <SidebarFooter />
    </aside>
  );
}