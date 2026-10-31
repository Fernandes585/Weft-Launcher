import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useI18n } from "./i18n/useI18n";
import { useInstall, InstalledVersion } from "./hooks/useInstall";
import { VersionActions } from "./components/VersionActions";
import "./App.css";
import "./components/install.css";

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

function formatDate(dateString: string): string {
  const date = new Date(dateString);
  return date.toLocaleString("pt-BR", {
    day: "2-digit",
    month: "2-digit",
    year: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function App() {
  const { t, formatSize, ready } = useI18n();
  const { installedVersions, refreshInstalled } = useInstall();
  const [releases, setReleases] = useState<Release[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [fromCache, setFromCache] = useState(false);

  const getInstalledVersion = useCallback(
    (tag: string): InstalledVersion | null => {
      return installedVersions.find((v) => v.tag === tag) ?? null;
    },
    [installedVersions]
  );

  async function fetchReleases(forceRefresh = false) {
    setLoading(true);
    setError(null);
    try {
      const data = await invoke<Release[]>("list_versions", { forceRefresh: forceRefresh });
      setReleases(data);
      setFromCache(data.some((r) => r.from_cache));
    } catch (err) {
      // Translate error if it matches known error codes
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

  return (
    <main className="container">
      <header className="header">
        <h1>{t('appTitle')}</h1>
        <button className="refresh-btn" onClick={() => fetchReleases(true)} disabled={loading}>
          {loading ? t('loading') : t('refreshButton')}
        </button>
      </header>

      {fromCache && (
        <div className="cache-banner">
          {t('cacheBanner')}
        </div>
      )}

      {error && (
        <div className="error-container">
          <div className="error">{t('errorPrefix')}{error}</div>
          <button className="refresh-btn" onClick={() => fetchReleases(true)} disabled={loading}>
            {t('refreshButton')}
          </button>
        </div>
      )}

      {!error && (
        <ul className="versions-list">
          {releases.map((release) => (
            <li key={release.tag} className="version-item">
              <div className="version-header">
                <span className="version-tag">{release.tag}</span>
                <span className="version-name">{release.name}</span>
                {release.prerelease && <span className="prerelease-badge">{t('prereleaseBadge')}</span>}
              </div>
              <div className="version-meta">
                <span>📅 {formatDate(release.published_at)}</span>
                <span>📦 {release.asset.name} ({formatSize(release.asset.size)})</span>
                {release.asset.sha256 && (
                  <span>🔐 SHA256: {release.asset.sha256.slice(0, 16)}...</span>
                )}
              </div>
              {release.body && (
                <details className="changelog">
                  <summary>{t('changelogSummary')}</summary>
                  <pre>{release.body}</pre>
                </details>
              )}
              <VersionActions
                release={release}
                installedVersion={getInstalledVersion(release.tag)}
                onInstallComplete={refreshInstalled}
              />
            </li>
          ))}
        </ul>
      )}

      {!error && releases.length === 0 && !loading && (
        <p className="empty-state">{t('emptyState')}</p>
      )}
    </main>
  );
}

export default App;
