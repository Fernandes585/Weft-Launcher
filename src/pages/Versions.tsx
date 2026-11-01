import { useI18n } from "../i18n/useI18n";
import { useInstall } from "../hooks/useInstall";
import { VersionActions } from "../components/VersionActions";
import { translateError } from "../i18n/errors";

interface Release {
  tag: string;
  name: string;
  published_at: string;
  body: string;
  prerelease: boolean;
  asset: {
    name: string;
    url: string;
    size: number;
    sha256: string | null;
  };
  from_cache: boolean;
}

interface VersionsPageProps {
  releases: Release[];
  loading: boolean;
  error: string | null;
  fromCache: boolean;
  onRefresh: (forceRefresh?: boolean) => void;
}

export function VersionsPage({
  releases,
  loading,
  error,
  fromCache,
  onRefresh,
}: VersionsPageProps) {
  const { t, formatSize: fmtSize, locale } = useI18n();
  const { installedVersions, refreshInstalled } = useInstall();

  const installedMap = new Map(installedVersions.map((v) => [v.tag, v]));

  const sortedReleases = [...releases].sort((a, b) =>
    new Date(b.published_at).getTime() - new Date(a.published_at).getTime()
  );

  return (
    <main className="page versions-page">
      <header className="page-header">
        <h1>{t("tabVersions")}</h1>
        <div className="header-actions">
          {fromCache && (
            <span className="cache-banner" role="status">{t("cacheBanner")}</span>
          )}
          {error && (
            <div className="error-banner" role="alert">
              {t("errorPrefix")}{translateError(error)}
            </div>
          )}
          <button className="btn-refresh" onClick={() => onRefresh(true)} disabled={loading}>
            {loading ? t("loading") : t("refreshButton")}
          </button>
        </div>
      </header>

      {sortedReleases.length === 0 ? (
        <div className="empty-state">{t("emptyState")}</div>
      ) : (
        <div className="versions-list" role="list">
          {sortedReleases.map((release) => {
            const installedVersion = installedMap.get(release.tag) ?? null;
            const asset = release.asset;

            return (
              <article
                key={release.tag}
                className={`version-entry${installedVersion ? " in-use" : ""}`}
                role="listitem"
              >
                <div className="version-header">
                  <div className="version-main">
                    <span className="version-tag">{release.tag}</span>
                    {release.prerelease && (
                      <span className="prerelease-badge">{t("prereleaseBadge")}</span>
                    )}
                  </div>
                  <div className="version-meta">
                    <span className="version-date">
                      {t("publishedDate")} {new Date(release.published_at).toLocaleDateString(locale)}
                    </span>
                    <span className="version-size">
                      {t("sizeLabel")} {fmtSize(asset.size)}
                    </span>
                    {asset.sha256 && (
                      <span className="version-sha256" title={asset.sha256}>
                        {t("sha256Label", asset.sha256.slice(0, 16) + "...")}
                      </span>
                    )}
                  </div>
                </div>

                <VersionActions
                  release={{
                    tag: release.tag,
                    asset: {
                      name: asset.name,
                      url: asset.url,
                      size: asset.size,
                      sha256: asset.sha256,
                    },
                  }}
                  installedVersion={installedVersion}
                  onInstallComplete={refreshInstalled}
                />
              </article>
            );
          })}
        </div>
      )}
    </main>
  );
}