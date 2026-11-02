import { useState, useEffect, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useI18n } from "./i18n/useI18n";
import { Tabs } from "./components/Tabs";
import { Home } from "./pages/Home";
import { VersionsPage } from "./pages/Versions";
import { NotesPage } from "./pages/Notes";
import { getLatestTag } from "./utils/version";
import "./App.css";

interface AssetInfo {
  name: string;
  url: string;
  size: number;
  sha256: string | null;
}

interface Release {
  tag: string;
  name: string;
  published_at: string;
  body: string;
  prerelease: boolean;
  asset: AssetInfo;
  from_cache: boolean;
}

type TabId = "home" | "versions" | "notes";

function App() {
  const { t, ready } = useI18n();
  const [releases, setReleases] = useState<Release[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [fromCache, setFromCache] = useState(false);
  const [activeTab, setActiveTab] = useState<TabId>("home");
  const [hasNewVersions, setHasNewVersions] = useState(false);

  const latestTag = useMemo(() => getLatestTag(releases), [releases]);

  async function fetchReleases(forceRefresh = false) {
    setLoading(true);
    setError(null);
    try {
      const data = await invoke<Release[]>("list_versions", { forceRefresh: forceRefresh });
      setReleases(data);
      setFromCache(data.some((r) => r.from_cache));
    } catch (err) {
      const errorStr = err as string;
      setError(errorStr);
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    fetchReleases(false);
  }, []);

  if (!ready) {
    return <div className="loading-screen">{t('loading')}</div>;
  }

  const renderPage = () => {
    switch (activeTab) {
      case "home":
        return (
          <Home
            onGoToVersions={() => setActiveTab("versions")}
            onGoToNotes={() => setActiveTab("notes")}
            latestTag={latestTag}
            releases={releases}
          />
        );
      case "versions":
        return (
          <VersionsPage
            releases={releases}
            loading={loading}
            error={error}
            fromCache={fromCache}
            onRefresh={fetchReleases}
            onNewVersionsChange={setHasNewVersions}
            latestTag={latestTag}
          />
        );
      case "notes":
        return <NotesPage releases={releases} />;
      default:
        return null;
    }
  };

  return (
    <div className="app-layout">
      <div className="main-content">
        <Tabs activeTab={activeTab} onChange={setActiveTab} hasNewVersions={hasNewVersions} />
        <div className="page-container" role="main">
          {renderPage()}
        </div>
      </div>
    </div>
  );
}

export default App;
