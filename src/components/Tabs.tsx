import { useI18n } from "../i18n/useI18n";
import { LanguageMenu } from "./LanguageMenu";
import { UpdateButton } from "./UpdateButton";

type TabId = "home" | "versions" | "notes";

interface TabsProps {
  activeTab: TabId;
  onChange: (tab: TabId) => void;
  hasNewVersions: boolean;
}

export function Tabs({ activeTab, onChange, hasNewVersions }: TabsProps) {
  const { t } = useI18n();

  const tabs: { id: TabId; label: string }[] = [
    { id: "home", label: t("tabHome") },
    { id: "versions", label: t("tabVersions") },
    { id: "notes", label: t("tabNotes") },
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
        >
          {tab.label}
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