import { useI18n } from "../i18n/useI18n";

type TabId = "home" | "versions" | "notes";

interface TabsProps {
  activeTab: TabId;
  onChange: (tab: TabId) => void;
}

export function Tabs({ activeTab, onChange }: TabsProps) {
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
        </button>
      ))}
    </nav>
  );
}