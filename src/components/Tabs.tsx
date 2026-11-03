import { useI18n } from "../i18n/useI18n";
import { LanguageMenu } from "./LanguageMenu";
import { UpdateButton } from "./UpdateButton";
import homeIcon from "../assets/item1-pixel.png";
import versionsIcon from "../assets/item2-pixel.png";
import notesIcon from "../assets/item3-pixel.png";

type TabId = "home" | "versions" | "notes";

interface TabsProps {
  activeTab: TabId;
  onChange: (tab: TabId) => void;
  hasNewVersions: boolean;
}

export function Tabs({ activeTab, onChange, hasNewVersions }: TabsProps) {
  const { t } = useI18n();

  const tabs: { id: TabId; label: string; icon: string }[] = [
    { id: "home", label: t("tabHome"), icon: homeIcon },
    { id: "versions", label: t("tabVersions"), icon: versionsIcon },
    { id: "notes", label: t("tabNotes"), icon: notesIcon },
  ];

  return (
    <nav className="tabs" role="tablist" aria-label={t("tabsLabel")}>
      {tabs.map((tab) => (
        <button
          key={tab.id}
          role="tab"
          aria-selected={activeTab === tab.id}
          aria-controls={`panel-${tab.id}`}
          id={`tab-${tab.id}`}
          className={`tab-btn ${activeTab === tab.id ? "active" : ""}`}
          onClick={() => onChange(tab.id)}
          aria-label={tab.label}
          title={tab.label}
        >
          <span className="tab-number">
            <img src={tab.icon} alt="" />
          </span>
          {tab.id === "versions" && hasNewVersions && <span className="new-dot" aria-label="New versions available" />}
        </button>
      ))}
      <div className="tabs-right">
        <UpdateButton />
        <LanguageMenu />
      </div>
    </nav>
  );
}