import { useState, useMemo } from "react";
import { useI18n } from "../i18n/useI18n";
import { getLocale } from "../i18n";
import { useInstall } from "../hooks/useInstall";
import { useGameState } from "../hooks/useGameState";
import { PixelBanner } from "../components/PixelBanner";

const parseTag = (s: string) =>
  s.replace(/^v/i, "").split(/[.\-]/).map((n) => parseInt(n, 10) || 0);

function compareTags(a: string, b: string) {
  const x = parseTag(a);
  const y = parseTag(b);
  for (let i = 0; i < Math.max(x.length, y.length); i++) {
    const d = (x[i] || 0) - (y[i] || 0);
    if (d) return d;
  }
  return 0;
}

export function Home({ onGoToVersions }: { onGoToVersions: () => void }) {
  const { t } = useI18n();
  const { installedVersions } = useInstall();
  const { anyRunning, isRunning, launch } = useGameState();

  const [selectedTag, setSelectedTag] = useState<string | null>(null);
  const [launchError, setLaunchError] = useState<string | null>(null);

  const sortedVersions = useMemo(
  () => [...installedVersions].sort((a, b) => compareTags(b.tag, a.tag)),
  [installedVersions]
);

  const effectiveTag = selectedTag ?? sortedVersions[0]?.tag ?? null;

  const selectedVersion = useMemo(
  () => sortedVersions.find((v) => v.tag === effectiveTag) ?? null,
  [sortedVersions, effectiveTag]
);

  const handleLaunch = async (tag: string) => {
    setLaunchError(null);
    const err = await launch(tag);
    if (err) {
      if (err.startsWith("Versão já está rodando:") || err.startsWith("This version is already running:")) {
        setLaunchError(t("errorAlreadyRunning"));
      } else if (err.startsWith("Outro jogo já está rodando:") || err.startsWith("Another game is running:")) {
        const match = err.match(/rodando:\s*(.+)$|running:\s*(.+)$/i);
        const otherTag = match?.[1] || match?.[2] || "";
        setLaunchError(t("errorGameRunning", otherTag));
      } else if (err.startsWith("Executável não encontrado:") || err.startsWith("Executable not found:")) {
        setLaunchError(t("errorExecutableNotFound", err.replace(/^(Executável não encontrado:|Executable not found:)\s*/, "")));
      } else if (err.startsWith("Versão em uso") || err.startsWith("Version in use")) {
        setLaunchError(t("errorVersionInUse", tag));
      } else {
        setLaunchError(err);
      }
    }
  };

  const isSelectedRunning = effectiveTag ? isRunning(effectiveTag) : false;
  const playDisabled = anyRunning || isSelectedRunning;

  if (installedVersions.length === 0) {
    return (
      <main className="page home-page">
        <section className="hero empty">
          <PixelBanner />
          <h1 className="hero-title">Weft</h1>
          <p className="empty-message">{t("noVersionsInstalled")}</p>
          <button className="btn-flat btn-primary" onClick={onGoToVersions}>
            {t("goToVersionsTab")}
          </button>
        </section>
      </main>
    );
  }

  return (
    <main className="page home-page">
      <section className="hero">
        <PixelBanner />
        <div className="hero-header">
          <h1 className="hero-title">Weft</h1>
          <div className="hero-version-badge">
            {t("heroBadge", effectiveTag || "—")}
          </div>
        </div>
        <div className="hero-play-area">
          <div className="version-selector">
            <label htmlFor="version-select" className="visually-hidden">
              {t("selectVersionLabel")}
            </label>
            <select
              id="version-select"
              value={effectiveTag || ""}
              onChange={(e) => setSelectedTag(e.target.value || null)}
              disabled={anyRunning}
            >
              {sortedVersions.map((v, i) => (
              <option key={v.tag} value={v.tag}>
              {i === 0 ? `${v.tag} — ${t("latestBadge")}` : v.tag}
              </option>
             ))}
            </select>
          </div>
          <button
              className="btn-play"
              onClick={() => effectiveTag && handleLaunch(effectiveTag)}
              disabled={!effectiveTag || playDisabled}
              aria-label={isSelectedRunning ? t("playing") : t("playButton")}
            >
            {isSelectedRunning ? t("playing") : t("playButton")}
          </button>
        </div>
        {launchError && (
          <div className="launch-error" role="alert">
            {launchError}
          </div>
        )}
      </section>

      <section className="card">
        <div className="card-header">
          <h2 className="card-title">{t("instanceCardTitle")}</h2>
        </div>
        <div className="card-body">
          {selectedVersion ? (
            <>
              <div className="detail-row">
                <span className="detail-label">{t("versionLabel")}</span>
                <span className="detail-value">{selectedVersion.tag}</span>
              </div>
              <div className="detail-row">
                <span className="detail-label">{t("installedAtLabel")}</span>
                <span className="detail-value">
                  {new Date(selectedVersion.installed_at).toLocaleString(getLocale())}
                </span>
              </div>
              <div className="detail-row">
                <span className="detail-label">{t("executableLabel")}</span>
                <span className="detail-value path">{selectedVersion.executable}</span>
              </div>
            </>
          ) : (
            <p className="select-version-hint">{t("selectVersionHint")}</p>
          )}
        </div>
      </section>
    </main>
  );
}