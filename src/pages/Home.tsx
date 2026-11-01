import { useState, useMemo } from "react";
import { useI18n } from "../i18n/useI18n";
import { useInstall } from "../hooks/useInstall";
import { useGameState } from "../hooks/useGameState";

export function Home({ onGoToVersions }: { onGoToVersions: () => void }) {
  const { t } = useI18n();
  const { installedVersions } = useInstall();
  const { anyRunning, isRunning, launch } = useGameState();

  const [selectedTag, setSelectedTag] = useState<string | null>(null);
  const [launchError, setLaunchError] = useState<string | null>(null);

  const selectedVersion = useMemo(
    () => installedVersions.find((v) => v.tag === selectedTag) ?? null,
    [installedVersions, selectedTag]
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

  const isSelectedRunning = selectedTag ? isRunning(selectedTag) : false;
  const playDisabled = anyRunning || isSelectedRunning;

  if (installedVersions.length === 0) {
    return (
      <main className="page home-page">
        <section className="hero empty">
          <h1>{t("appTitle")}</h1>
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
        <div className="hero-header">
          <div>
            <h1 className="hero-title">Weft</h1>
          </div>
          <div className="hero-version-badge">
            VERSÃO {selectedTag || installedVersions[0]?.tag || "—"} · PRONTO
          </div>
        </div>
        <div className="hero-play-area">
          <div className="version-selector">
            <label htmlFor="version-select" className="visually-hidden">
              {t("selectVersionLabel")}
            </label>
            <select
              id="version-select"
              value={selectedTag || ""}
              onChange={(e) => setSelectedTag(e.target.value || null)}
              disabled={anyRunning}
            >
              <option value="">{t("selectVersionPlaceholder")}</option>
              {installedVersions.map((v) => (
                <option key={v.tag} value={v.tag}>
                  {v.tag}
                </option>
              ))}
            </select>
          </div>
          <button
            className="btn-play"
            onClick={() => selectedTag && handleLaunch(selectedTag)}
            disabled={!selectedTag || playDisabled}
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
                  {new Date(selectedVersion.installed_at).toLocaleString("pt-BR")}
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

      <section className="card">
        <div className="card-header">
          <h2 className="card-title">{t("gameFolderTitle")}</h2>
        </div>
        <div className="card-body">
          <p className="detail-value path">{t("gameFolderPath")}</p>
        </div>
        <div className="card-actions">
          <button className="btn-flat btn-secondary" disabled>
            {t("openFolderButton")}
          </button>
        </div>
      </section>
    </main>
  );
}