import { useState, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { t } from "../i18n";

export interface InstalledVersion {
  tag: string;
  path: string;
  executable: string;
  installed_at: string;
}

export interface InstallProgress {
  tag: string;
  stage: "downloading" | "verifying" | "extracting" | "finalizing" | "done" | "error";
  percent?: number;
  downloaded?: number;
  total?: number;
  speedMbps?: number;
  message?: string;
}

export function useInstall() {
  const [installedVersions, setInstalledVersions] = useState<InstalledVersion[]>([]);
  const [progress, setProgress] = useState<InstallProgress | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const refreshInstalled = useCallback(async () => {
    try {
      const versions = await invoke<InstalledVersion[]>("get_installed_versions");
      setInstalledVersions(versions);
    } catch (err) {
      console.error("Failed to load installed versions:", err);
    }
  }, []);

  useEffect(() => {
    refreshInstalled();
  }, [refreshInstalled]);

  const translateError = useCallback((err: string): string => {
    if (err.startsWith("Erro de rede:") || err.startsWith("Network error:")) {
      return t("errors.network", err.replace(/^(Erro de rede:|Network error:)\s*/, ""));
    }
    if (err.startsWith("Erro de I/O:") || err.startsWith("I/O error:")) {
      return t("errors.io", err.replace(/^(Erro de I\/O:|I\/O error:)\s*/, ""));
    }
    if (err.startsWith("Erro de serialização:") || err.startsWith("Serialization error:")) {
      return t("errors.serialization", err.replace(/^(Erro de serialização:|Serialization error:)\s*/, ""));
    }
    if (err.startsWith("SHA256 não confere:") || err.startsWith("SHA256 mismatch:")) {
      const match = err.match(/esperado\s+([^,]+),\s*obtido\s+(.+)$|expected\s+([^,]+),\s*got\s+(.+)$/i);
      if (match) {
        const expected = match[1] || match[3] || "";
        const actual = match[2] || match[4] || "";
        return t("errorHashMismatch", expected, actual);
      }
      return t("errorHashMismatch", "", "");
    }
    if (err.startsWith("SHA256 obrigatório não fornecido") || err.startsWith("Required SHA256 not provided")) {
      return t("errorHashMissing");
    }
    if (err.startsWith("Falha no download:") || err.startsWith("Download failed:")) {
      return t("errorDownloadFailed", err.replace(/^(Falha no download:|Download failed:)\s*/, ""));
    }
    if (err.startsWith("Falha na extração:") || err.startsWith("Extraction failed:")) {
      return t("errorExtractFailed", err.replace(/^(Falha na extração:|Extraction failed:)\s*/, ""));
    }
    if (err.startsWith("Versão já instalada:") || err.startsWith("Version already installed:")) {
      return t("errorAlreadyInstalled", err.replace(/^(Versão já instalada:|Version already installed:)\s*/, ""));
    }
    if (err.startsWith("Versão não instalada:") || err.startsWith("Version not installed:")) {
      return t("errorNotInstalled", err.replace(/^(Versão não instalada:|Version not installed:)\s*/, ""));
    }
    if (err.includes("Asset não encontrado") || err.includes("Asset not found")) {
      return t("errors.assetNotFound");
    }
    if (err.includes("Digest SHA256 ausente") || err.includes("SHA256 digest missing")) {
      return t("errors.missingDigest");
    }
    if (err.includes("Limite de taxa") || err.includes("rate limit")) {
      return t("errors.rateLimited");
    }
    if (err.includes("Acesso negado") || err.includes("access denied") || err.includes("403")) {
      return t("errors.forbidden");
    }
    if (err.includes("Sem conexão") || err.includes("Offline and no cache")) {
      return t("errors.offlineNoCache");
    }
    return err;
  }, []);

  const install = useCallback(async (tag: string, assetUrl: string, assetName: string, expectedSha256: string | null) => {
    setLoading(true);
    setError(null);
    setProgress({ tag, stage: "downloading", percent: 0 });

    let unlisten: (() => void) | null = null;
    try {
      unlisten = await listen<InstallProgress>("install-progress", (event) => {
        // Filter events by tag to avoid cross-contamination
        if (event.payload.tag === tag) {
          setProgress(event.payload);
        }
      });
    } catch {
      // Event listening failed, continue without progress updates
    }

    try {
      await invoke<InstalledVersion>("install_version", {
        tag,
        assetUrl,
        assetName,
        expectedSha256,
      });
      setProgress({ tag, stage: "done", percent: 100 });
      await refreshInstalled();
    } catch (err) {
      const translated = translateError(err as string);
      setError(translated);
      setProgress({ tag, stage: "error", message: translated });
      throw err;
    } finally {
      setLoading(false);
      if (unlisten) unlisten();
    }
  }, [refreshInstalled, translateError]);

  const remove = useCallback(async (tag: string) => {
    setLoading(true);
    setError(null);
    try {
      await invoke("remove_version", { tag });
      await refreshInstalled();
    } catch (err) {
      const translated = translateError(err as string);
      setError(translated);
      throw err;
    } finally {
      setLoading(false);
    }
  }, [refreshInstalled, translateError]);

  const isInstalled = useCallback((tag: string) => {
    return installedVersions.some((v) => v.tag === tag);
  }, [installedVersions]);

  const clearError = useCallback(() => {
    setError(null);
    if (progress?.stage === "error") {
      setProgress(null);
    }
  }, [progress]);

  return {
    installedVersions,
    progress,
    error,
    loading,
    refreshInstalled,
    install,
    remove,
    isInstalled,
    clearError,
  };
}